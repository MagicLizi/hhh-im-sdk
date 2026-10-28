//! 运行时的协议服务：用 `transport` 一侧的写法当宿主，经内存管道和一个用运行时写的小插件对话。

use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, DuplexStream};

use super::*;
use crate::rpc::RpcId;

#[tokio::test]
async fn reading_notifications_require_explicit_host_negotiation() {
    let (host, mut writes) = Host::new();
    let state = crate::reading::ReadState {
        scope: crate::reading::ReadScope {
            conversation_id: "chat".into(),
            thread_root: None,
        },
        through: Some("9007199254740992001".into()),
        read_message_ids: Vec::new(),
    };
    host.reading_changed(&state);
    assert!(
        tokio::time::timeout(Duration::from_millis(10), writes.next())
            .await
            .is_err()
    );
    host.set_reading_events(true);
    host.reading_changed(&state);
    let message = writes.next().await.unwrap();
    assert_eq!(message.method.as_deref(), Some("reading/changed"));
    assert_eq!(message.params.unwrap()["through"], "9007199254740992001");
    host.set_reading_events(false);
    host.notify(
        "reading/changed",
        json!({"scope":{"conversationId":"chat"}}),
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(10), writes.next())
            .await
            .is_err()
    );
}

/// 小插件：`slow` 睡很久（等着被取消）；`fast` 马上回；`save` 先向宿主发 `credential/save` 等回复，把回复交回；
/// `count` 数三次平台请求；`self` 自己回复后再推一条通知；`fail` 回错误。
struct Small;

impl Plugin for Small {
    async fn request(self: Arc<Self>, call: Call) -> Reply {
        match call.method.as_str() {
            "slow" => {
                tokio::time::sleep(Duration::from_secs(60)).await;
                Reply::from(Ok(json!({ "slow": true })))
            }
            "fast" => Reply::from(Ok(json!({ "fast": true }))),
            "save" => Reply::from(
                call.host
                    .request("credential/save", json!({ "credential": { "token": "t" } }))
                    .await
                    .map(|saved| json!({ "saved": saved })),
            ),
            "count" => {
                for _ in 0..3 {
                    metering::request();
                }
                Reply::from(Ok(json!({ "items": [] })))
            }
            "gated" => {
                for _ in 0..3 {
                    if let Err(error) = metering::read_request().await {
                        return Reply::from(Err(error));
                    }
                }
                Reply::from(Ok(json!({"items":[]})))
            }
            "self" => {
                call.host
                    .respond(call.id.clone(), Ok(json!({ "started": true })));
                call.host.notify("login/step", json!({ "flowId": "f1" }));
                Reply::Responded
            }
            other => Reply::from(Err(method_not_found(other))),
        }
    }

    async fn notification(self: Arc<Self>, method: String, _params: Value, host: Host) {
        if method == "login/cancel" {
            host.notify("status", json!({ "state": "online", "seen": method }));
        }
    }
}

/// 宿主这一头：往插件写一行、读插件写出来的下一行。
struct FakeHost {
    to_plugin: DuplexStream,
    from_plugin: tokio::io::Lines<BufReader<DuplexStream>>,
}

impl FakeHost {
    async fn send(&mut self, message: Value) {
        let line = format!("{message}\n");
        self.to_plugin.write_all(line.as_bytes()).await.unwrap();
    }

    async fn next(&mut self) -> Value {
        let line = tokio::time::timeout(Duration::from_secs(5), self.from_plugin.next_line())
            .await
            .expect("插件 5 秒内该有输出")
            .unwrap()
            .expect("插件的输出不该已经关了");
        serde_json::from_str(&line).unwrap()
    }
}

fn start() -> (FakeHost, tokio::task::JoinHandle<()>) {
    let (host_side, plugin_in) = tokio::io::duplex(64 * 1024);
    let (plugin_out, host_read) = tokio::io::duplex(64 * 1024);
    let server = tokio::spawn(serve(Small, plugin_in, plugin_out));
    (
        FakeHost {
            to_plugin: host_side,
            from_plugin: BufReader::new(host_read).lines(),
        },
        server,
    )
}

fn request(id: i64, method: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": {} })
}

// 没协商新能力时不发回调，旧宿主照旧从结果结算平台请求数。
#[tokio::test]
async fn read_gating_keeps_legacy_hosts_on_result_counting() {
    let (mut host, _server) = start();
    host.send(request(1, "gated")).await;
    let reply = host.next().await;
    assert_eq!(reply["id"], 1);
    assert_eq!(reply["result"]["platformRequests"], 3);
}

// 等额度期间还能回心跳；每次明确放行只放一条，拒绝时结束原读取。
#[tokio::test]
async fn each_read_waits_for_a_grant_without_blocking_other_calls() {
    let (mut host, _server) = start();
    let mut read = request(1, "gated");
    read["params"]["_meta"] = json!({"readPermit":"r7"});
    host.send(read).await;
    let first = host.next().await;
    assert_eq!(first["method"], "requests/acquire");
    assert_eq!(first["params"]["permit"], "r7");
    host.send(request(2, "fast")).await;
    assert_eq!(host.next().await["id"], 2, "读取等额度时其他请求仍完成");
    host.send(json!({"jsonrpc":"2.0","id":first["id"],"result":{"granted":true}}))
        .await;
    let second = host.next().await;
    assert_eq!(second["method"], "requests/acquire");
    host.send(
        serde_json::to_value(RpcMessage::err(
            serde_json::from_value(second["id"].clone()).unwrap(),
            RpcError::from_kind(ErrorKind::Cancelled, "读取不再需要"),
        ))
        .unwrap(),
    )
    .await;
    let stopped = host.next().await;
    assert_eq!(stopped["id"], 1);
    assert_eq!(stopped["error"]["data"]["kind"], "cancelled");
}

// 每次平台读取前先确认许可交付；不把网络响应完成当作回执。
#[tokio::test]
async fn a_read_receipt_precedes_the_next_platform_request() {
    let (mut host, _server) = start();
    let mut read = request(1, "gated");
    read["params"]["_meta"] = json!({"readPermit":"r-receipt"});
    host.send(read).await;
    let ask = host.next().await;
    assert_eq!(ask["params"]["receipt"], true, "主动协商许可回执");
    host.send(
        json!({"jsonrpc":"2.0","id":ask["id"],"result":{"granted":true,"receipt":"issued-1"}}),
    )
    .await;
    let received = host.next().await;
    assert_eq!(received["method"], "requests/received");
    assert_eq!(received["params"]["receipt"], "issued-1");
    assert!(received.get("id").is_none(), "回执是通知，不等网络读取完成");
    assert_eq!(host.next().await["method"], "requests/acquire");
}

#[tokio::test]
async fn an_invalid_read_receipt_does_not_start_the_platform_request() {
    let (mut host, _server) = start();
    let mut read = request(1, "gated");
    read["params"]["_meta"] = json!({"readPermit":"r-invalid-receipt"});
    host.send(read).await;
    let ask = host.next().await;
    host.send(json!({"jsonrpc":"2.0","id":ask["id"],"result":{"granted":true,"receipt":""}}))
        .await;
    let refused = host.next().await;
    assert_eq!(refused["id"], 1);
    assert_eq!(refused["error"]["data"]["kind"], "protocolError");
}

// 老宿主对未知方法可能回空对象，不能误当放行。
#[tokio::test]
async fn an_empty_acknowledgement_never_grants_a_platform_read() {
    let (mut host, _server) = start();
    let mut read = request(1, "gated");
    read["params"]["_meta"] = json!({"readPermit":"r8"});
    host.send(read).await;
    let ask = host.next().await;
    host.send(json!({"jsonrpc":"2.0","id":ask["id"],"result":{}}))
        .await;
    let refused = host.next().await;
    assert_eq!(refused["id"], 1);
    assert_eq!(refused["error"]["data"]["kind"], "protocolError");
}

// 请求并发：慢的不挡快的；`$/cancel` 停掉慢的并回“已取消”。
#[tokio::test]
async fn requests_run_concurrently_and_can_be_cancelled() {
    let (mut host, _server) = start();
    host.send(request(1, "slow")).await;
    host.send(request(2, "fast")).await;
    let fast = host.next().await;
    assert_eq!(
        (fast["id"].clone(), fast["result"]["fast"].clone()),
        (json!(2), json!(true))
    );
    host.send(json!({ "jsonrpc": "2.0", "method": "$/cancel", "params": { "id": 1 } }))
        .await;
    let cancelled = host.next().await;
    assert_eq!(cancelled["id"], 1);
    assert_eq!(cancelled["error"]["data"]["kind"], "cancelled");
}

// 插件向宿主发请求并等回复；宿主回了才把结果交回原请求。
#[tokio::test]
async fn plugin_requests_wait_for_the_host_reply() {
    let (mut host, _server) = start();
    host.send(request(7, "save")).await;
    let asked = host.next().await;
    assert_eq!(asked["method"], "credential/save");
    assert_eq!(asked["params"]["credential"]["token"], "t");
    let id = asked["id"].clone();
    assert!(
        id.as_str().is_some_and(|id| id.starts_with('p')),
        "插件的编号是字符串，不和宿主撞号"
    );
    host.send(json!({ "jsonrpc": "2.0", "id": id, "result": { "ok": true } }))
        .await;
    let reply = host.next().await;
    assert_eq!(reply["id"], 7);
    assert_eq!(reply["result"]["saved"]["ok"], true);
}

// 数到的平台请求多于一次时补上 platformRequests；自己回复的不再多回；不认识的方法回 -32601；通知交给插件。
#[tokio::test]
async fn counts_self_replies_unknown_methods_and_notifications() {
    let (mut host, _server) = start();
    host.send(request(3, "count")).await;
    let counted = host.next().await;
    assert_eq!(counted["result"]["platformRequests"], 3);
    host.send(request(4, "self")).await;
    let started = host.next().await;
    assert_eq!(
        (started["id"].clone(), started["result"]["started"].clone()),
        (json!(4), json!(true))
    );
    let step = host.next().await;
    assert_eq!(step["method"], "login/step");
    host.send(request(5, "nope")).await;
    let unknown = host.next().await;
    assert_eq!(unknown["error"]["code"], crate::rpc::code::METHOD_NOT_FOUND);
    host.send(json!({ "jsonrpc": "2.0", "method": "login/cancel", "params": { "flowId": "f1" } }))
        .await;
    let status = host.next().await;
    assert_eq!(status["params"]["seen"], "login/cancel");
}

// 输入读完：还在跑的请求给一小会儿做完，写完就退出（不会因为有人还拿着宿主那一头而一直等）。
#[tokio::test]
async fn ends_when_the_input_closes() {
    let (mut host, server) = start();
    host.send(request(9, "fast")).await;
    host.send(request(10, "slow")).await;
    drop(host.to_plugin);
    let fast = tokio::time::timeout(Duration::from_secs(5), host.from_plugin.next_line())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(fast.contains("\"fast\":true"));
    tokio::time::timeout(Duration::from_secs(10), server)
        .await
        .expect("输入关了，服务该退出")
        .unwrap();
}

#[test]
fn platform_requests_are_added_only_when_more_than_one() {
    assert_eq!(with_platform_requests(Ok(json!({})), 1).unwrap(), json!({}));
    assert_eq!(
        with_platform_requests(Ok(json!({})), 2).unwrap(),
        json!({ "platformRequests": 2 })
    );
    assert_eq!(
        with_platform_requests(Ok(json!({ "platformRequests": 9 })), 2).unwrap(),
        json!({ "platformRequests": 9 }),
        "插件自己写了就不改"
    );
    assert_eq!(
        with_platform_requests(Ok(json!([1])), 3).unwrap(),
        json!([1])
    );
    let _ = RpcId::Number(1);
}
