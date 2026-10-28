//! 插件运行时（feature = "runtime"，契约 provider-protocol v15 §3.4）：写 IM 插件程序时，和平台无关、每个插件都要的
//! 那一半都在这里，插件只写平台自己的事。
//!
//! - 协议服务（[`serve_stdio`]、[`serve`]）：从标准输入一行一条地读，宿主的每个请求各开一个任务并发处理（查询不被
//!   下载、登录堵住，protocol-v1 §13）；回复、插件发出的请求与通知排进同一条队列，由一个写手按顺序一行一条写出去；
//!   `$/cancel` 停掉对应的任务并回“已取消”；标准输入关了就停掉还在跑的请求、退出。
//! - 向宿主发请求并等回复（[`Host::request`]）：`credential/save`、`events/push` 这类按编号等宿主回话，“先存后用”靠它。
//! - 日志（[`install_log`]）：插件里的 tracing 日志转成 `log/write` 通知，宿主并进 HHH 的日志。
//! - 资源分段（[`Segments`]）：宿主一段一段要资源时，同一个资源顺着读复用同一条读流。
//! - 上传攒齐（[`Uploads`]）：`uploads/begin`、`chunk`、`finish` 攒齐、核对长度与 SHA-256。
//! - 请求计数（[`metering`]）：一次宿主调用里向平台发了几次请求，运行时随结果报给宿主（`platformRequests`）。
//! - 性能自报（契约 provider-protocol v32）：替插件声明 `perf`；宿主剖析开着时记每个请求的排队与处理耗时、平台请求、
//!   等待放行、分段与上传字节，每个间隔发一条 `perf/report`。插件代码不用改。
//!
//! 插件实现 [`Plugin`]：收到一个请求怎么办（方法名与参数），通知怎么办（可不管）。
//!
//! # Examples
//!
//! ```
//! use std::sync::Arc;
//! use hhh_im_protocol::runtime::{self, Call, Plugin, Reply};
//! use serde_json::json;
//!
//! struct Demo;
//!
//! impl Plugin for Demo {
//!     async fn request(self: Arc<Self>, call: Call) -> Reply {
//!         Reply::from(match call.method.as_str() {
//!             "ping" => Ok(json!({})),
//!             other => Err(runtime::method_not_found(other)),
//!         })
//!     }
//! }
//!
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() {
//! // 正式运行：runtime::serve_stdio(Demo).await;
//! let input = &b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n"[..];
//! let mut output = Vec::new();
//! runtime::serve(Demo, input, &mut output).await;
//! let reply: serde_json::Value = serde_json::from_slice(&output).unwrap();
//! assert_eq!(reply["id"], 1);
//! assert_eq!(reply["result"], json!({}));
//! # }
//! ```

mod host;
mod log;
pub mod metering;
mod perf;
mod segments;
mod uploads;
#[cfg(target_os = "wasi")]
mod wasi_stdio;

use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex};

use serde::de::DeserializeOwned;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::task::AbortHandle;

pub use self::host::{Host, Writes};
pub use self::log::install_log;
pub use self::segments::{ByteStream, NextChunk, Opened, Segments, chunk_of_bytes};
pub use self::uploads::{Upload, Uploads};
use crate::rpc::{ErrorKind, RpcError, RpcId, RpcMessage, code, codec};

/// 结果里“这次向平台发了几个请求”的字段名（protocol-v1 §8，v14）。
const PLATFORM_REQUESTS_FIELD: &str = "platformRequests";
/// 输入读完后，还在跑的请求最多再等多久。
const DRAIN_WAIT: std::time::Duration = std::time::Duration::from_secs(3);

/// 宿主发来的一个请求。
#[derive(Debug, Clone)]
pub struct Call {
    /// 请求编号（自己回复时用，见 [`Reply::Responded`]）。
    pub id: RpcId,
    /// 方法名，如 `messages/list`。
    pub method: String,
    /// 参数；没给时是 `null`。用 [`params`] 读成协议类型。
    pub params: Value,
    /// 宿主那一头：发通知、发请求、自己回复。
    pub host: Host,
}

/// 一个请求的结局。
#[derive(Debug)]
pub enum Reply {
    /// 回这个结果（运行时替你写回去；数到的平台请求多于一次时，对象结果里补上 `platformRequests`）。
    Result(Result<Value, RpcError>),
    /// 已经用 [`Host::respond`] 自己回过了（如 `login/start` 先回“开始了”、再接着推登录步骤）。
    Responded,
}

impl From<Result<Value, RpcError>> for Reply {
    fn from(result: Result<Value, RpcError>) -> Self {
        Self::Result(result)
    }
}

/// 一个 IM 插件：收到宿主的请求与通知怎么办。每个请求在自己的任务里跑，彼此并发。
pub trait Plugin: Send + Sync + 'static {
    /// 宿主发来的一个请求。
    fn request(self: Arc<Self>, call: Call) -> impl Future<Output = Reply> + Send;

    /// 宿主发来的一个通知（`login/cancel`、`login/submit` 等）；`$/cancel` 由运行时处理，不会到这里。默认不管。
    fn notification(
        self: Arc<Self>,
        method: String,
        params: Value,
        host: Host,
    ) -> impl Future<Output = ()> + Send {
        let _ = (method, params, host);
        async {}
    }
}

/// 用标准输入、标准输出跑协议服务，直到宿主关掉标准输入。日志要交给宿主时，先调 [`install_log`]
/// （拿 [`serve_stdio_with`] 给的 [`Host`]）。
///
/// 编 iPhone / iPad 用的 WASM 包（`wasm32-wasip2`，契约 provider-protocol v18）时插件代码不用改：标准输入输出自动改走
/// WASI 的接口。没内容可读时按计时醒来看（tokio 在 WASI 上等不了标准输入），空闲后第一条请求至多晚 16 毫秒。
/// 用到 tokio 的网络、文件等能力的插件编 WASM 包时要带 `RUSTFLAGS="--cfg tokio_unstable"`。
pub async fn serve_stdio<P: Plugin>(plugin: P) {
    serve_stdio_with(plugin, |_| {}).await;
}

/// 同 [`serve_stdio`]，开始读之前先把宿主那一头交给 `ready`（装日志、留着推送用）。
pub async fn serve_stdio_with<P: Plugin>(plugin: P, ready: impl FnOnce(&Host)) {
    let (host, writes) = Host::new();
    ready(&host);
    serve_stdio_on(Arc::new(plugin), host, writes).await;
}

/// 同 [`serve_on`]，用标准输入输出（WASM 包上同 [`serve_stdio`] 自动改走 WASI 的接口）：插件要先建好 [`Host`]、
/// 插件本身要拿着它（推送用）时用这个。
pub async fn serve_stdio_on<P: Plugin>(plugin: Arc<P>, host: Host, writes: Writes) {
    let (input, output) = stdio();
    run(plugin, host, input, output, writes).await;
}

/// 这个目标上的标准输入与标准输出。
#[cfg(not(target_os = "wasi"))]
fn stdio() -> (tokio::io::Stdin, tokio::io::Stdout) {
    (tokio::io::stdin(), tokio::io::stdout())
}

/// 这个目标上的标准输入与标准输出：WASI 上 tokio 自带的用不了，走 WASI 的接口。
#[cfg(target_os = "wasi")]
fn stdio() -> (wasi_stdio::Stdin, wasi_stdio::Stdout) {
    (wasi_stdio::Stdin::new(), wasi_stdio::Stdout)
}

/// 在给定的输入、输出上跑协议服务（测试用内存里的管道），直到输入读完。
pub async fn serve<P, R, W>(plugin: P, input: R, output: W)
where
    P: Plugin,
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let (host, writes) = Host::new();
    run(Arc::new(plugin), host, input, output, writes).await;
}

/// 同 [`serve`]，插件已经在 `Arc` 里、宿主那一头事先建好（测试里先拿到 [`Host`] 再开始服务）。
pub async fn serve_on<P, R, W>(plugin: Arc<P>, host: Host, writes: Writes, input: R, output: W)
where
    P: Plugin,
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    run(plugin, host, input, output, writes).await;
}

/// 读手与写手：读到宿主的消息就分出去；输入读完后停掉还在跑的请求，等写手把已经排着的写完。
async fn run<P, R, W>(plugin: Arc<P>, host: Host, input: R, output: W, writes: Writes)
where
    P: Plugin,
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let tasks: Arc<Mutex<HashMap<String, AbortHandle>>> = Arc::default();
    let reader = async {
        let mut lines = BufReader::new(input).lines();
        loop {
            let line = match lines.next_line().await {
                Ok(Some(line)) => line,
                Ok(None) => break,
                Err(error) => {
                    tracing::warn!(%error, "标准输入读不下去了，停下");
                    break;
                }
            };
            if line.trim().is_empty() {
                continue;
            }
            match codec::decode_line(&line) {
                Ok(message) => receive(&plugin, &host, &tasks, message),
                Err(error) => tracing::warn!(%error, "宿主发来一行不是 JSON-RPC，跳过"),
            }
        }
        // 宿主关了标准输入（之前已经发过 shutdown）：还在跑的请求给一小会儿做完（测试里输入是有限的一段，最后几个
        // 请求还没回），到时还没完的停掉；写手写完排着的就停。
        let deadline = tokio::time::Instant::now() + DRAIN_WAIT;
        while !tasks.lock().expect("任务表锁中毒").is_empty()
            && tokio::time::Instant::now() < deadline
        {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let handles: Vec<AbortHandle> = tasks
            .lock()
            .expect("任务表锁中毒")
            .drain()
            .map(|(_, handle)| handle)
            .collect();
        for handle in handles {
            handle.abort();
        }
        host.close();
    };
    tokio::join!(reader, write_loop(output, writes));
}

/// 写手：一条一行写到输出，每条都立刻送出去；读手说输入读完了（排在后面的“停”）就停。
async fn write_loop<W: AsyncWrite + Unpin>(mut output: W, mut writes: Writes) {
    while let Some(out) = writes.0.recv().await {
        let message = match out {
            host::Out::Message(message) => *message,
            host::Out::Close => break,
        };
        let Ok(line) = codec::encode_line(&message) else {
            tracing::error!("要写给宿主的一条消息编不出来，丢掉");
            continue;
        };
        if output.write_all(line.as_bytes()).await.is_err() || output.flush().await.is_err() {
            break;
        }
    }
}

/// 收到宿主的一条消息：回复交给在等的请求，通知当场开任务处理，请求各开一个任务。
fn receive<P: Plugin>(
    plugin: &Arc<P>,
    host: &Host,
    tasks: &Arc<Mutex<HashMap<String, AbortHandle>>>,
    message: RpcMessage,
) {
    if message.is_response() {
        host.answer(message);
        return;
    }
    let (Some(method), params) = (message.method, message.params.unwrap_or(Value::Null)) else {
        return;
    };
    let Some(id) = message.id else {
        if method == "$/cancel" {
            cancel(host, tasks, params);
            return;
        }
        let (plugin, host) = (plugin.clone(), host.clone());
        tokio::spawn(async move { plugin.notification(method, params, host).await });
        return;
    };
    // 性能自报的开与关由运行时自己回，不交给插件（契约 provider-protocol v32）。
    if method == crate::Method::PerfStart.as_str() || method == crate::Method::PerfStop.as_str() {
        host.respond(id, perf::handle(host, &method, params));
        return;
    }
    let key = id.to_string();
    let permit = params.pointer("/_meta/readPermit").cloned();
    let initialize = method == crate::Method::Initialize.as_str();
    if initialize {
        host.set_reading_events(params.get("readingEvents").and_then(Value::as_bool) == Some(true));
    }
    // 剖析开着时才留方法名（记排队与处理耗时用）；关着时只多这一次原子读取。
    let probed = host.perf().active().then(|| {
        (
            host.perf().clone(),
            method.clone(),
            std::time::Instant::now(),
        )
    });
    let call = Call {
        id: id.clone(),
        method,
        params,
        host: host.clone(),
    };
    let (plugin, host_for_task, table) = (plugin.clone(), host.clone(), tasks.clone());
    // 先拿着任务表再开任务：任务做完删自己那一项时，表里一定已经记上了。
    let mut guard = tasks.lock().expect("任务表锁中毒");
    let task = tokio::spawn(async move {
        let began = std::time::Instant::now();
        if let Some((recorder, method, received)) = &probed {
            recorder.time(
                &format!("{} · {method}", perf::QUEUE),
                *received,
                began - *received,
            );
        }
        let recorder = probed.as_ref().map(|(recorder, ..)| recorder.clone());
        let (reply, sent) = perf::scoped(
            recorder,
            metering::counted_with_permit(plugin.request(call), host_for_task.clone(), permit),
        )
        .await;
        if let Some((recorder, method, _)) = &probed {
            recorder.time(
                &format!("{} · {method}", perf::HANDLE),
                began,
                began.elapsed(),
            );
            recorder.count(
                &format!("{} · {method}", perf::PLATFORM_REQUESTS),
                u64::from(sent),
            );
        }
        if let Reply::Result(mut result) = reply {
            if initialize && let Ok(value) = &mut result {
                perf::declare(value);
            }
            host_for_task.respond(id.clone(), with_platform_requests(result, sent));
        }
        table.lock().expect("任务表锁中毒").remove(&id.to_string());
    });
    guard.insert(key, task.abort_handle());
}

/// `$/cancel`：停掉对应的任务并回“已取消”；已经做完的不管。
fn cancel(host: &Host, tasks: &Mutex<HashMap<String, AbortHandle>>, params: Value) {
    let Ok(cancel) = serde_json::from_value::<crate::method::CancelParams>(params) else {
        tracing::debug!("读不懂的 $/cancel，忽略");
        return;
    };
    let handle = tasks
        .lock()
        .expect("任务表锁中毒")
        .remove(&cancel.id.to_string());
    if let Some(handle) = handle {
        handle.abort();
        host.respond(
            cancel.id,
            Err(RpcError::from_kind(
                ErrorKind::Cancelled,
                "宿主取消了这个请求。",
            )),
        );
    }
}

/// 数到的平台请求多于一次、结果是对象时补上 `platformRequests`（一次是协议的默认值，不写）。
fn with_platform_requests(result: Result<Value, RpcError>, sent: u32) -> Result<Value, RpcError> {
    match result {
        Ok(Value::Object(mut object))
            if sent > 1 && !object.contains_key(PLATFORM_REQUESTS_FIELD) =>
        {
            object.insert(PLATFORM_REQUESTS_FIELD.into(), sent.into());
            Ok(Value::Object(object))
        }
        other => other,
    }
}

/// 把请求参数读成协议类型；读不懂时回“参数不对”（-32602）。
///
/// # Errors
/// 参数的形状与类型对不上时返回 [`invalid_params`]。
///
/// # Examples
///
/// ```
/// use hhh_im_protocol::method::PeopleGetParams;
/// let ok: PeopleGetParams = hhh_im_protocol::runtime::params(serde_json::json!({ "ids": ["u1"] })).unwrap();
/// assert_eq!(ok.ids, ["u1"]);
/// let bad = hhh_im_protocol::runtime::params::<PeopleGetParams>(serde_json::json!({ "ids": 1 }));
/// assert_eq!(bad.unwrap_err().code, hhh_im_protocol::rpc::code::INVALID_PARAMS);
/// ```
pub fn params<T: DeserializeOwned>(value: Value) -> Result<T, RpcError> {
    serde_json::from_value(value).map_err(invalid_params)
}

/// 参数不合法（-32602）：宿主发来的参数读不懂。
pub fn invalid_params(error: impl std::fmt::Display) -> RpcError {
    RpcError {
        code: code::INVALID_PARAMS,
        ..RpcError::from_kind(ErrorKind::InvalidRequest, format!("参数不对：{error}"))
    }
}

/// 方法不存在（-32601）：这个插件不支持这个方法。
pub fn method_not_found(method: &str) -> RpcError {
    RpcError {
        code: code::METHOD_NOT_FOUND,
        ..RpcError::from_kind(ErrorKind::Unsupported, format!("这个插件不支持 {method}。"))
    }
}

#[cfg(test)]
mod tests;
