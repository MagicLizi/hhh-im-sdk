//! `hhh-im-check`：拿一个插件把 HHH-IM-Protocol 的用例跑一遍（protocol-v1 §17）。
//! 第三方开发者自测、HHH 验收都用它。
//!
//! 用法：`hhh-im-check <清单路径> [--mock] [--live --read-only] [--allow-send <会话编号>] [--login <登录方式编号>]`
//!
//! 每项检查给出 通过 / 失败 / 跳过，失败写明原因。整体有失败时进程以非 0 退出。
//! `--login` 只在给了时才开始一次登录（可能连平台，所以要显式给）：看插件推来的第一步合不合格（契约 provider-protocol v36
//! 的登录窗口一步按协议包的规则核对起始网址与要截的开头），然后取消这次登录。

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use hhh_im_protocol::platform::{Capabilities, HistoryMode, Manifest, PlatformDesc};
use hhh_im_protocol::transport::{Connection, Incoming, StdioChild};
use serde_json::{Value, json};

#[path = "hhh_im_check/rich_send.rs"]
mod rich_send;

#[derive(Clone, Copy, PartialEq)]
enum Status {
    Pass,
    Fail,
    Skip,
}

struct Report {
    rows: Vec<(String, Status, String)>,
}

impl Report {
    fn new() -> Self {
        Self { rows: Vec::new() }
    }
    fn pass(&mut self, name: &str) {
        self.rows.push((name.into(), Status::Pass, String::new()));
    }
    fn fail(&mut self, name: &str, why: impl Into<String>) {
        self.rows.push((name.into(), Status::Fail, why.into()));
    }
    fn skip(&mut self, name: &str, why: impl Into<String>) {
        self.rows.push((name.into(), Status::Skip, why.into()));
    }
    fn print_and_code(&self) -> i32 {
        println!("\n== hhh-im-check 结果 ==");
        let mut failed = 0;
        for (name, st, why) in &self.rows {
            let tag = match st {
                Status::Pass => "通过",
                Status::Fail => {
                    failed += 1;
                    "失败"
                }
                Status::Skip => "跳过",
            };
            if why.is_empty() {
                println!("  [{tag}] {name}");
            } else {
                println!("  [{tag}] {name} —— {why}");
            }
        }
        let pass = self.rows.iter().filter(|r| r.1 == Status::Pass).count();
        let skip = self.rows.iter().filter(|r| r.1 == Status::Skip).count();
        println!("\n通过 {pass} / 失败 {failed} / 跳过 {skip}");
        i32::from(failed > 0)
    }
}

struct Args {
    manifest: PathBuf,
    mock: bool,
    live: bool,
    read_only: bool,
    allow_send: Option<String>,
    /// 要开始一次登录看第一步的登录方式编号（v36）。
    login: Option<String>,
}

fn parse_args() -> Result<Args, String> {
    let mut a = std::env::args().skip(1);
    let manifest = a.next().ok_or(
        "用法：hhh-im-check <清单路径> [--mock] [--live --read-only] [--allow-send <会话编号>] [--login <登录方式编号>]",
    )?;
    let mut args = Args {
        manifest: PathBuf::from(manifest),
        mock: false,
        live: false,
        read_only: false,
        allow_send: None,
        login: None,
    };
    while let Some(flag) = a.next() {
        match flag.as_str() {
            "--mock" => args.mock = true,
            "--live" => args.live = true,
            "--read-only" => args.read_only = true,
            "--allow-send" => {
                args.allow_send = Some(a.next().ok_or("--allow-send 后要带会话编号")?);
            }
            "--login" => {
                args.login = Some(a.next().ok_or("--login 后要带登录方式编号")?);
            }
            other => return Err(format!("不认识的参数：{other}")),
        }
    }
    Ok(args)
}

/// 后台收集插件主动发来的东西：自动确认 events/push 与 credential/save，记录推送事件与登录步骤。
#[derive(Default)]
struct Collected {
    events: Vec<Value>,
    login_steps: Vec<Value>,
    push_count: usize,
    credential_saved: bool,
    /// 收到的 `perf/report`（契约 provider-protocol v32）。
    perf_reports: Vec<Value>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    let code = run(args).await;
    std::process::exit(code);
}

async fn run(args: Args) -> i32 {
    let mut report = Report::new();

    // 1. 清单合法。
    let manifest_text = match std::fs::read_to_string(&args.manifest) {
        Ok(t) => t,
        Err(e) => {
            report.fail("清单可读", format!("{e}"));
            return report.print_and_code();
        }
    };
    let manifest: Manifest = match serde_json::from_str(&manifest_text) {
        Ok(m) => m,
        Err(e) => {
            report.fail("清单是合法 JSON", format!("{e}"));
            return report.print_and_code();
        }
    };
    if Manifest::id_ok(&manifest.id) {
        report.pass("清单编号合法");
    } else {
        report.fail("清单编号合法", format!("id={} 不符合规则", manifest.id));
    }
    // IM 类型可选：没写不检查（自成一类），写了要符合编号同一套规则。
    match manifest.im_type.as_deref() {
        None => report.skip("IM 类型合法", "清单没写 imType，自成一类"),
        Some(_) if manifest.im_type_ok() => report.pass("IM 类型合法"),
        Some(kind) => report.fail("IM 类型合法", format!("imType={kind} 不符合规则")),
    }
    // 发布字段（目标、更新来源、最低 HHH 版本）都可选：没写不检查，写了要合规（契约 provider-protocol v10 §4.2）。
    if manifest.target.is_none() && manifest.source.is_none() && manifest.min_host.is_none() {
        report.skip("发布字段合法", "清单没写 target / source / minHost");
    } else {
        match manifest.release_fields_ok() {
            Ok(()) => report.pass("发布字段合法"),
            Err(reason) => report.fail("发布字段合法", reason),
        }
    }
    let this_os = if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    };
    if manifest.os.iter().any(|o| o == this_os) {
        report.pass("清单声明支持当前系统");
    } else {
        report.fail(
            "清单声明支持当前系统",
            format!("当前 {this_os} 不在 {:?}", manifest.os),
        );
    }

    // 启动插件。
    let dir = args
        .manifest
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let program = resolve_command(&dir, &manifest.command);
    let logs: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let logs_cb = logs.clone();
    let temp = std::env::temp_dir().join(format!("hhh-im-check-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&temp);
    let child = match StdioChild::spawn(&program, &manifest.args, &temp, move |line| {
        logs_cb.lock().unwrap().push(line)
    }) {
        Ok(c) => c,
        Err(e) => {
            report.fail("启动插件进程", format!("{e}（命令：{program}）"));
            return report.print_and_code();
        }
    };
    let conn = Connection::new(child.clone());

    // 后台收集插件主动发来的东西。
    let collected = Arc::new(Mutex::new(Collected::default()));
    {
        let conn = conn.clone();
        let collected = collected.clone();
        tokio::spawn(async move {
            while let Some(inc) = conn.next_incoming().await {
                match inc {
                    Incoming::Request { id, method, params } => {
                        match method.as_str() {
                            "events/push" => {
                                let mut c = collected.lock().unwrap();
                                c.push_count += 1;
                                if let Some(evs) = params["events"].as_array() {
                                    c.events.extend(evs.iter().cloned());
                                }
                                // 锁在这里释放，再 await 回复（守卫不能跨 await）。
                            }
                            "credential/save" => {
                                collected.lock().unwrap().credential_saved = true;
                            }
                            _ => {}
                        }
                        let _ = conn.respond(id, json!({})).await;
                    }
                    Incoming::Notification { method, params } => {
                        if method == "login/step" {
                            collected
                                .lock()
                                .unwrap()
                                .login_steps
                                .push(params["step"].clone());
                        } else if method == "perf/report" {
                            collected.lock().unwrap().perf_reports.push(params);
                        }
                    }
                }
            }
        });
    }

    // 2. initialize 及时。
    let init = call_t(
        &conn,
        "initialize",
        json!({
            "protocolVersions": [1],
            "host": { "name": "hhh-im-check", "version": "0.1.0" },
            "locale": "zh-CN",
            "tempDir": temp.to_string_lossy(),
            "network": { "proxy": null }
        }),
        Duration::from_secs(10),
    )
    .await;
    let init = match init {
        Ok(v) => {
            report.pass("initialize 在 10 秒内响应");
            v
        }
        Err(e) => {
            report.fail("initialize 在 10 秒内响应", e);
            child.shutdown().await;
            return report.print_and_code();
        }
    };

    // 3. 平台说明、能力、组合自洽。
    let platform: Result<PlatformDesc, _> = serde_json::from_value(init["platform"].clone());
    let caps: Result<Capabilities, _> = serde_json::from_value(init["capabilities"].clone());
    let (platform, caps) = match (platform, caps) {
        (Ok(p), Ok(c)) => {
            report.pass("平台说明与能力清单格式合法");
            (p, c)
        }
        _ => {
            report.fail(
                "平台说明与能力清单格式合法",
                "无法解析 platform 或 capabilities",
            );
            child.shutdown().await;
            return report.print_and_code();
        }
    };
    match caps.check_consistent(platform.history) {
        Ok(()) => report.pass("能力组合自洽"),
        Err(e) => report.fail("能力组合自洽", e),
    }
    if init["protocolVersion"].as_u64() == Some(1) {
        report.pass("协议版本是宿主支持的");
    } else {
        report.fail(
            "协议版本是宿主支持的",
            format!("{:?}", init["protocolVersion"]),
        );
    }
    check_v12_declarations(&mut report, &manifest, &platform, &init);
    match platform.check_maintenance(&caps) {
        Ok(()) if caps.message_maintenance => {
            report.pass("旧消息维护声明合法");
            for (mode, rules) in [
                ("repair", &platform.content_repairs),
                ("reread", &platform.content_rereads),
            ] {
                for rule in rules {
                    let check = format!("旧消息维护 {mode}/{rule} 可调用");
                    match call_t(
                        &conn,
                        "messages/maintain",
                        json!({"mode": mode, "rule": rule, "messages": []}),
                        Duration::from_secs(10),
                    )
                    .await
                    {
                        Ok(value) => match serde_json::from_value::<
                            hhh_im_protocol::method::MaintenanceResult,
                        >(value)
                        {
                            Ok(result) if result.updates.is_empty() && result.reread.is_empty() => {
                                report.pass(&check)
                            }
                            _ => report.fail(&check, "空消息维护必须返回空结果"),
                        },
                        Err(error) => report.fail(&check, error),
                    }
                }
            }
        }
        Ok(()) => report.skip("旧消息维护", "未声明可选能力"),
        Err(error) => report.fail("旧消息维护声明合法", error),
    }
    check_v13_declarations(&mut report, &init);
    check_v31_declarations(&mut report, &platform);

    // 4. session/open。
    let open = call_t(
        &conn,
        "session/open",
        json!({
            "credential": null, "settings": {}, "syncState": null
        }),
        Duration::from_secs(30),
    )
    .await;
    let online = match &open {
        Ok(v) if v["state"] == "online" => {
            report.pass("session/open 返回 online 或 needsLogin");
            true
        }
        Ok(v) if v["state"] == "needsLogin" => {
            report.pass("session/open 返回 online 或 needsLogin");
            false
        }
        Ok(v) => {
            report.fail(
                "session/open 返回 online 或 needsLogin",
                format!("state={:?}", v["state"]),
            );
            false
        }
        Err(e) => {
            report.fail("session/open 返回 online 或 needsLogin", e.clone());
            false
        }
    };

    if !online && !args.live {
        report.skip(
            "数据相关检查",
            "需要先登录（--live）或插件支持 --mock 直接在线",
        );
    }

    // 给推送一点时间到达。
    tokio::time::sleep(Duration::from_millis(300)).await;
    check_login_start(
        &conn,
        &mut report,
        &collected,
        args.login.as_deref(),
        online,
    )
    .await;
    check_v12_calls(&conn, &mut report, &caps, online).await;
    check_v13_calls(&conn, &mut report, &init, online).await;
    check_v31_calls(&conn, &mut report, &platform, &caps, online).await;
    check_stickers(&conn, &mut report, &caps, online).await;
    if caps.perf {
        check_perf(&conn, &mut report, &collected).await;
    }

    // 5–7. 推送数据的检查（push-only 平台）。
    if platform.history == HistoryMode::None && online {
        let evs = collected.lock().unwrap().events.clone();
        if evs.is_empty() {
            report.skip("推送消息检查", "这次没有收到推送事件");
        } else {
            check_pushed_messages(&mut report, &evs);
        }
    }

    // 8. 资源按段读（如果声明了且找得到一个资源）。
    if caps.resources.read {
        let found = find_resource(&collected.lock().unwrap().events);
        if let Some((conv, msg, key, kind)) = found {
            check_resource_read(&conn, &mut report, &conv, &msg, &key, &kind).await;
            if caps.resources.variants {
                check_resource_variants(&conn, &mut report, &conv, &msg, &key, &kind).await;
            }
        } else {
            report.skip("资源分段读", "推送里没有可读资源样本");
        }
    }

    // 9. 发送。
    match (&args.allow_send, caps.send.as_ref()) {
        (Some(conv), Some(send)) if send.text => {
            check_send(&conn, &mut report, conv, caps.agent_attribution.send).await;
        }
        (Some(_), _) => report.skip("发送检查", "插件未声明发文字能力"),
        (None, Some(_)) if caps.is_read_only() => {
            report.pass("只读插件不声明发送能力");
        }
        (None, _) => report.skip("发送检查", "未给 --allow-send，跳过真实发送"),
    }
    rich_send::check(
        &conn,
        &mut report,
        args.allow_send.as_deref(),
        Some(&caps.rich_send),
    )
    .await;

    // 9b. 宿主头像（provider-protocol v35）：交头像会改本人在这家 IM 的头像、发给别人，算写操作，同发送一样要 --allow-send。
    match (caps.account.host_avatar, &args.allow_send) {
        (false, _) => report.skip("宿主头像能收", "没声明 account.hostAvatar"),
        (true, None) => report.skip("宿主头像能收", "未给 --allow-send，跳过会改本人头像的检查"),
        (true, Some(_)) => check_host_avatar(&conn, &mut report).await,
    }

    // 10. ping 及时、shutdown 退出。
    match call_t(&conn, "ping", json!({}), Duration::from_secs(10)).await {
        Ok(_) => report.pass("ping 及时响应"),
        Err(e) => report.fail("ping 及时响应", e),
    }
    let _ = conn.call("shutdown", json!({})).await;
    child.shutdown().await;
    report.pass("shutdown 后进程结束");

    report.print_and_code()
}

/// 契约 provider-protocol v12 的新声明：声明了就要写对，没声明的跳过。
fn check_v12_declarations(
    report: &mut Report,
    manifest: &Manifest,
    platform: &PlatformDesc,
    init: &Value,
) {
    use hhh_im_protocol::platform::{Adjustable, ContentKind};
    if platform.user_adjustable.is_empty() {
        report.skip("开放的读取项认得出", "没声明 userAdjustable");
    } else if platform.user_adjustable.contains(&Adjustable::Unknown) {
        report.fail(
            "开放的读取项认得出",
            "userAdjustable 里有宿主不认识的取值（这一项不会出现在参数配置里）",
        );
    } else {
        report.pass("开放的读取项认得出");
    }
    if platform.edits_by_content.is_empty() {
        report.skip("按内容认编辑的取值认得出", "没声明 editsByContent");
    } else if platform.edits_by_content.contains(&ContentKind::Unknown) {
        report.fail(
            "按内容认编辑的取值认得出",
            "editsByContent 里有宿主不认识的内容块种类",
        );
    } else {
        report.pass("按内容认编辑的取值认得出");
    }
    if platform.retranslate_rules.is_empty() {
        report.skip("重译规则名不重复", "没声明 retranslateRules");
    } else {
        let mut seen = std::collections::BTreeSet::new();
        match platform
            .retranslate_rules
            .iter()
            .find(|rule| rule.name.trim().is_empty() || !seen.insert(rule.name.as_str()))
        {
            None => report.pass("重译规则名不重复"),
            Some(rule) => report.fail(
                "重译规则名不重复",
                format!("规则名“{}”是空的或重复了（宿主不用它）", rule.name),
            ),
        }
    }
    match platform
        .emoji
        .as_ref()
        .and_then(|emoji| emoji.text_pattern.as_deref())
    {
        None => report.skip("正文表情写法能编译", "没声明 emoji.textPattern"),
        Some(pattern) => match pattern_problem(pattern, "表情名") {
            None => report.pass("正文表情写法能编译"),
            Some(why) => report.fail("正文表情写法能编译", why),
        },
    }
    match &platform.avatar_identity {
        None => report.skip("头像身份写法能编译", "没声明 avatarIdentity"),
        Some(identity) => match pattern_problem(&identity.key_pattern, "头像编号").or_else(|| {
            identity
                .variant_pattern
                .as_deref()
                .and_then(|pattern| pattern_problem(pattern, "变体"))
        }) {
            None => report.pass("头像身份写法能编译"),
            Some(why) => report.fail("头像身份写法能编译", why),
        },
    }
    if manifest.logo_ink.is_none() {
        report.skip("标志外框在画布内", "清单没写 logoInk");
    } else if manifest.logo_ok() {
        report.pass("标志外框在画布内");
    } else {
        report.fail(
            "标志外框在画布内",
            "logoInk 的四个数要是有限数，宽、高大于 0",
        );
    }
    check_login_methods(report, manifest, init);
}

/// 登录方式（缺口 6）：类型认得出、编号不重复、不超过宿主上限；声明的回调路径在这个 IM 自己的 `/hhh-friend-im/<编号>/` 下面。
fn check_login_methods(report: &mut Report, manifest: &Manifest, init: &Value) {
    use hhh_im_protocol::method::{
        LoginKind, LoginMethod, callback_path_ok, resolve_callback_path,
    };
    let Some(declared) = init.get("loginMethods").filter(|v| !v.is_null()) else {
        report.skip("登录方式写法对", "没声明 loginMethods");
        report.skip("回调路径在本 IM 目录下", "没声明回调路径");
        return;
    };
    let methods: Vec<LoginMethod> = match serde_json::from_value(declared.clone()) {
        Ok(methods) => methods,
        Err(e) => {
            report.fail("登录方式写法对", format!("loginMethods 格式不对：{e}"));
            return;
        }
    };
    let mut ids = std::collections::BTreeSet::new();
    let problem =
        if methods.len() > hhh_im_protocol::budget::LOGIN_METHODS_MAX {
            Some(format!(
                "至多 {} 种，声明了 {} 种",
                hhh_im_protocol::budget::LOGIN_METHODS_MAX,
                methods.len()
            ))
        } else {
            methods.iter().find_map(|m| {
                if m.kind == LoginKind::Unknown {
                    Some(format!("“{}”的类型宿主不认识（不会出现在登录框里）", m.id))
                } else if m.id.trim().is_empty() || !ids.insert(m.id.as_str()) {
                    Some(format!("编号“{}”是空的或重复了", m.id))
                } else if m.default_value.as_deref().is_some_and(|value| {
                    m.kind != LoginKind::Relay || !value.starts_with("https://")
                }) {
                    // v14：默认中继只给“通过中继登录”，发布的插件只写 https（本机测试的回环地址宿主也认，但不该发出去）。
                    Some(format!(
                        "“{}”的默认值只能给通过中继登录、写 https 网址",
                        m.id
                    ))
                } else {
                    None
                }
            })
        };
    match problem {
        None => report.pass("登录方式写法对"),
        Some(why) => report.fail("登录方式写法对", why),
    }
    let paths: Vec<&str> = methods
        .iter()
        .filter_map(|m| m.callback_path.as_deref())
        .collect();
    if paths.is_empty() {
        report.skip("回调路径在本 IM 目录下", "没声明回调路径");
        return;
    }
    // 平台编号由宿主给（一般在插件编号前加前缀，保留编号就是编号本身），自检不替宿主推；相对路径（v14）怎么编号都拼得对，
    // 以 `/` 开头的完整路径（v12 写法）只能核对是不是按编号本身写的。
    if let Some(bad) = paths
        .iter()
        .find(|p| !p.starts_with('/') && resolve_callback_path(p, &manifest.id).is_none())
    {
        report.fail(
            "回调路径在本 IM 目录下",
            format!("{bad} 不是合规的相对路径（每段只用字母、数字与 - _ .）"),
        );
    } else if paths.iter().all(|p| !p.starts_with('/')) {
        report.pass("回调路径在本 IM 目录下");
    } else if let Some(bad) = paths
        .iter()
        .find(|p| p.starts_with('/') && !callback_path_ok(p, &manifest.id))
    {
        report.fail(
            "回调路径在本 IM 目录下",
            format!(
                "{bad} 是完整路径：改成相对路径（如 callback），宿主会拼在这个 IM 自己的目录下"
            ),
        );
    } else {
        report.skip(
            "回调路径在本 IM 目录下",
            format!(
                "路径在 /hhh-friend-im/{}/ 下：只有带官方签名、用保留编号的插件这样写才算数；写成相对路径就不用管这个",
                manifest.id
            ),
        );
    }
}

/// 契约 provider-protocol v13 的新声明：操作写对；声明了用宿主账号登录时提示只有官方插件能用。没声明的跳过。
fn check_v13_declarations(report: &mut Report, init: &Value) {
    use hhh_im_protocol::budget::ACTIONS_MAX;
    use hhh_im_protocol::method::{ActionDecl, LoginKind, LoginMethod, action_problem};
    match init.get("actions").filter(|v| !v.is_null()) {
        None => report.skip("操作声明写法对", "没声明 actions"),
        Some(declared) => match serde_json::from_value::<Vec<ActionDecl>>(declared.clone()) {
            Err(e) => report.fail("操作声明写法对", format!("actions 格式不对：{e}")),
            Ok(actions) if actions.len() > ACTIONS_MAX => report.fail(
                "操作声明写法对",
                format!(
                    "至多 {ACTIONS_MAX} 项，声明了 {} 项（多出的不会出现在菜单里）",
                    actions.len()
                ),
            ),
            Ok(actions) => {
                let mut ids = std::collections::BTreeSet::new();
                match actions.iter().find_map(|a| {
                    action_problem(a)
                        .map(|why| format!("“{}”：{why}", a.id))
                        .or_else(|| {
                            (!ids.insert(a.id.as_str())).then(|| format!("编号“{}”重复了", a.id))
                        })
                }) {
                    None => report.pass("操作声明写法对"),
                    Some(why) => report.fail("操作声明写法对", why),
                }
            }
        },
    }
    let host = init
        .get("loginMethods")
        .and_then(|v| serde_json::from_value::<Vec<LoginMethod>>(v.clone()).ok())
        .is_some_and(|methods| methods.iter().any(|m| m.kind == LoginKind::Host));
    if host {
        report.skip(
            "用宿主账号登录",
            "声明了 host：只有带官方签名的插件能用，自定义插件宿主不登记这一类（登录框里没有它）",
        );
    } else {
        report.skip("用宿主账号登录", "没声明 host 登录方式");
    }
}

/// 契约 provider-protocol v13 的新方法：声明了操作就要能开始（`actions/start` 回流程编号），试完立刻取消。只有会话级
/// 操作时要一个会话才能试，跳过。
/// `--login <方式>`（契约 provider-protocol v36）：没在线时开始一次登录，等插件推第一步（至多 5 秒），取消这次登录，再按
/// [`check_login_steps`] 核对收到的步骤。不给 `--login` 不开始登录（可能连平台）。
async fn check_login_start(
    conn: &Arc<Connection>,
    report: &mut Report,
    collected: &Arc<Mutex<Collected>>,
    method: Option<&str>,
    online: bool,
) {
    const NAME: &str = "登录步骤合格";
    let Some(method) = method else {
        report.skip(NAME, "没给 --login，不开始登录");
        return;
    };
    if online {
        report.skip(NAME, "已经在线，不开始登录");
        return;
    }
    let flow = match call_t(
        conn,
        "login/start",
        json!({ "method": method, "values": {} }),
        Duration::from_secs(10),
    )
    .await
    {
        Ok(v) => v["flowId"].as_str().unwrap_or_default().to_owned(),
        Err(e) => {
            report.fail(NAME, format!("login/start（{method}）：{e}"));
            return;
        }
    };
    let waited = tokio::time::timeout(Duration::from_secs(5), async {
        while collected.lock().unwrap().login_steps.is_empty() {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await;
    let _ = conn.notify("login/cancel", json!({ "flowId": flow })).await;
    if waited.is_err() {
        report.fail(NAME, format!("开始“{method}”登录后 5 秒内没推登录步骤"));
        return;
    }
    let steps = collected.lock().unwrap().login_steps.clone();
    check_login_steps(report, &steps);
}

/// 收到的每一步宿主都认（读得出来、不超上限；登录窗口一步的起始网址与要截的开头合规）。
fn check_login_steps(report: &mut Report, steps: &[Value]) {
    use hhh_im_protocol::method::{LoginStep, step_problem};
    const NAME: &str = "登录步骤合格";
    let problem = steps.iter().find_map(|raw| {
        let kind = raw["kind"].as_str().unwrap_or("?").to_owned();
        match serde_json::from_value::<LoginStep>(raw.clone()) {
            Err(e) => Some(format!("“{kind}”一步读不出来：{e}")),
            Ok(step) => step_problem(&step).map(|why| format!("“{kind}”一步：{why}")),
        }
    });
    match (steps.is_empty(), problem) {
        (true, _) => report.skip(NAME, "没收到登录步骤"),
        (false, Some(why)) => report.fail(NAME, why),
        (false, None) => report.pass(NAME),
    }
}

async fn check_v13_calls(conn: &Arc<Connection>, report: &mut Report, init: &Value, online: bool) {
    use hhh_im_protocol::method::{ActionDecl, ActionScope, ActionsStartResult};
    let actions: Vec<ActionDecl> = init
        .get("actions")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    let Some(action) = actions.iter().find(|a| a.scope == ActionScope::Account) else {
        if actions.is_empty() {
            report.skip("操作能开始", "没声明 actions");
        } else {
            report.skip("操作能开始", "只有会话级操作，要一个会话才能试");
        }
        return;
    };
    if !online {
        report.skip("操作能开始", "没登录，试不了");
        return;
    }
    match call_t(
        conn,
        "actions/start",
        json!({ "actionId": action.id }),
        Duration::from_secs(10),
    )
    .await
    {
        Ok(v) => match serde_json::from_value::<ActionsStartResult>(v) {
            Ok(r) if !r.flow_id.is_empty() => {
                let _ = conn
                    .notify("action/cancel", json!({ "flowId": r.flow_id }))
                    .await;
                report.pass("操作能开始");
            }
            Ok(_) => report.fail("操作能开始", format!("“{}”回的流程编号是空的", action.id)),
            Err(e) => report.fail("操作能开始", format!("“{}”的结果格式不对：{e}", action.id)),
        },
        Err(e) => report.fail("操作能开始", format!("“{}”：{e}", action.id)),
    }
}

/// 契约 provider-protocol v32：声明了 `perf` 时，`perf/start` 后按间隔收到合规的 `perf/report`，`perf/stop` 后不再收到。
async fn check_perf(
    conn: &Arc<Connection>,
    report: &mut Report,
    collected: &Arc<Mutex<Collected>>,
) {
    use hhh_im_protocol::budget::{PERF_INTERVAL_MIN_MS, PERF_REPORT_BYTES_MAX};
    use hhh_im_protocol::method::PerfReportParams;
    const SESSION: u64 = 1;
    let interval = Duration::from_millis(PERF_INTERVAL_MIN_MS);
    if let Err(e) = call_t(
        conn,
        "perf/start",
        json!({ "session": SESSION, "intervalMs": PERF_INTERVAL_MIN_MS }),
        Duration::from_secs(10),
    )
    .await
    {
        report.fail("声明了 perf 时按间隔自报", format!("perf/start：{e}"));
        return;
    }
    tokio::time::sleep(interval * 3 + Duration::from_millis(200)).await;
    let reports = std::mem::take(&mut collected.lock().unwrap().perf_reports);
    let problem = if reports.is_empty() {
        Some("perf/start 之后没收到 perf/report".to_owned())
    } else {
        reports.iter().find_map(|raw| {
            if raw.to_string().len() > PERF_REPORT_BYTES_MAX {
                return Some("一条报告超过 64 KiB".to_owned());
            }
            match serde_json::from_value::<PerfReportParams>(raw.clone()) {
                Err(e) => Some(format!("报告格式不对：{e}")),
                Ok(r) if r.session != SESSION => Some(format!("报告的会话编号是 {}", r.session)),
                Ok(r) => r.check().err().map(str::to_owned),
            }
        })
    };
    match problem {
        None if reports.len() <= 4 => report.pass("声明了 perf 时按间隔自报"),
        None => report.fail(
            "声明了 perf 时按间隔自报",
            format!("1.7 秒里收到 {} 条，比间隔密", reports.len()),
        ),
        Some(why) => report.fail("声明了 perf 时按间隔自报", why),
    }
    if let Err(e) = call_t(
        conn,
        "perf/stop",
        json!({ "session": SESSION }),
        Duration::from_secs(10),
    )
    .await
    {
        report.fail("perf/stop 后不再自报", format!("perf/stop：{e}"));
        return;
    }
    // 停之前已经在路上的那一条不算。
    tokio::time::sleep(Duration::from_millis(100)).await;
    collected.lock().unwrap().perf_reports.clear();
    tokio::time::sleep(interval * 2 + Duration::from_millis(200)).await;
    match collected.lock().unwrap().perf_reports.len() {
        0 => report.pass("perf/stop 后不再自报"),
        n => report.fail("perf/stop 后不再自报", format!("停了以后还收到 {n} 条")),
    }
}

/// 带一个括号的写法（正文表情写法、头像身份）的问题：编不过、或没有一个括号（第一个括号是 `group`）。按常用写法核对，
/// 不支持前后断言等少见写法。
fn pattern_problem(pattern: &str, group: &str) -> Option<String> {
    if pattern.trim().is_empty() {
        return Some("写法是空的".into());
    }
    match regex_lite::Regex::new(pattern) {
        Err(e) => Some(format!("编不过：{e}")),
        Ok(re) if re.captures_len() < 2 => Some(format!("要有一个括号，第一个括号是{group}")),
        Ok(_) => None,
    }
}

/// 契约 provider-protocol v31：表情写进文字的写法，声明了就要写对，没声明的跳过（输入框没有表情按钮）。
fn check_v31_declarations(report: &mut Report, platform: &PlatformDesc) {
    use hhh_im_protocol::platform::TextTemplate;
    match platform
        .emoji
        .as_ref()
        .and_then(|emoji| emoji.text_template.as_deref())
    {
        None => report.skip(
            "表情写法格式对",
            "没声明 emoji.textTemplate（输入框没有表情按钮）",
        ),
        Some(template) => match TextTemplate::parse(template) {
            Ok(_) => report.pass("表情写法格式对"),
            Err(why) => report.fail("表情写法格式对", why),
        },
    }
}

/// 只读检查声明的账号贴纸目录；不会增删收藏或发送消息。
async fn check_stickers(
    conn: &Arc<Connection>,
    report: &mut Report,
    caps: &Capabilities,
    online: bool,
) {
    use hhh_im_protocol::{Sticker, StickerPack, StickerPage, valid_sticker_id};
    let mut pack = None;
    for (name, method, enabled) in [
        ("表情包目录", "stickers/packs", caps.stickers.packs),
        (
            "收藏贴纸目录",
            "stickers/favorites",
            caps.stickers.favorites,
        ),
        ("包内贴纸目录", "stickers/list", caps.stickers.packs),
    ] {
        if !enabled {
            report.skip(name, "未声明可选能力");
            continue;
        }
        if !online {
            report.skip(name, "尚未登录，需登录后补验");
            continue;
        }
        if method == "stickers/list" && pack.is_none() {
            report.skip(name, "账号没有表情包样本");
            continue;
        }
        let mut args = json!({"limit":10});
        if method == "stickers/list" {
            args["packId"] = json!(pack);
        }
        match call_t(conn, method, args, Duration::from_secs(30)).await {
            Ok(value) => {
                let valid = if method == "stickers/packs" {
                    serde_json::from_value::<StickerPage<StickerPack>>(value).is_ok_and(|page| {
                        pack = page.items.first().map(|item| item.id.clone());
                        page.items.len() <= 10
                            && page.next.as_deref().is_none_or(valid_sticker_id)
                            && page.items.iter().all(|item| {
                                valid_sticker_id(&item.id)
                                    && item.name.len() <= 512
                                    && item.cover.as_deref().is_none_or(valid_sticker_id)
                            })
                    })
                } else {
                    serde_json::from_value::<StickerPage<Sticker>>(value).is_ok_and(|page| {
                        page.items.len() <= 10
                            && page.next.as_deref().is_none_or(valid_sticker_id)
                            && page.items.iter().all(|item| {
                                valid_sticker_id(&item.id)
                                    && valid_sticker_id(&item.image_key)
                                    && item.name.len() <= 512
                            })
                    })
                };
                if valid {
                    report.pass(name);
                } else {
                    report.fail(name, "目录字段或分页预算不合格");
                }
            }
            Err(error) => report.fail(name, error),
        }
    }
}

/// 契约 provider-protocol v31：读一次表情目录，核对它与写法、分类图标对得上（[`check_v31_catalog`]）。
async fn check_v31_calls(
    conn: &Arc<Connection>,
    report: &mut Report,
    platform: &PlatformDesc,
    caps: &Capabilities,
    online: bool,
) {
    const CHECKS: [&str; 2] = ["写进文字的表情认得回来", "分类图标宿主认得"];
    if !caps.reactions.catalog {
        for name in CHECKS {
            report.skip(name, "没声明 reactions.catalog");
        }
        return;
    }
    let catalog = match call_t(conn, "emoji/catalog", json!({}), Duration::from_secs(30)).await {
        Ok(value) => serde_json::from_value::<hhh_im_protocol::method::EmojiCatalogResult>(value)
            .map_err(|e| format!("表情目录格式不对：{e}")),
        Err(_) if !online => {
            for name in CHECKS {
                report.skip(name, "没登录，读不了表情目录");
            }
            return;
        }
        Err(e) => Err(e),
    };
    match catalog {
        Ok(catalog) => check_v31_catalog(report, platform, &catalog),
        Err(why) => {
            for name in CHECKS {
                report.fail(name, why.clone());
            }
        }
    }
}

/// 表情目录与写法、分类图标对得上（v31）：能写进文字的每个表情，按 `textTemplate` 写出来的文字要被 `textPattern`
/// 整段认出、认出的名字或编号（按占位）对得回它——插进输入框发出去的在 HHH 里显示成图；分类给的图标在宿主的图标库里。
fn check_v31_catalog(
    report: &mut Report,
    platform: &PlatformDesc,
    catalog: &hhh_im_protocol::method::EmojiCatalogResult,
) {
    use hhh_im_protocol::platform::{TemplateSlot, TextTemplate};
    let rules = platform.emoji.as_ref();
    let template = rules
        .and_then(|e| e.text_template.as_deref())
        .and_then(|t| TextTemplate::parse(t).ok());
    let pattern = rules
        .and_then(|e| e.text_pattern.as_deref())
        .and_then(|p| regex_lite::Regex::new(p).ok());
    const ROUND_TRIP: &str = "写进文字的表情认得回来";
    match (template, pattern) {
        (None, _) => report.skip(ROUND_TRIP, "没声明 emoji.textTemplate，或写法不对"),
        (Some(_), None) => report.skip(
            ROUND_TRIP,
            "没声明能用的 emoji.textPattern（宿主不把正文里写的表情换成图，插件读回时自己翻成表情的不受影响）",
        ),
        (Some(template), Some(pattern)) => {
            let mut wrong = Vec::new();
            let mut checked = 0usize;
            for emoji in catalog.emojis.iter().filter(|e| e.in_text) {
                let value = match template.slot() {
                    TemplateSlot::Type => emoji.emoji_type.as_str(),
                    // 用名字写的写法里，没有名字的表情写不进文字。
                    TemplateSlot::Name => match emoji.name.as_deref() {
                        Some(name) => name,
                        None => continue,
                    },
                };
                checked += 1;
                let text = template.write(value);
                let found = pattern.captures(&text).filter(|c| c.get(0).is_some_and(|m| m.as_str() == text));
                if found.and_then(|c| c.get(1)).map(|m| m.as_str()) != Some(value) {
                    wrong.push(text);
                }
            }
            if checked == 0 {
                report.skip(ROUND_TRIP, "目录里没有能写进文字的表情");
            } else if wrong.is_empty() {
                report.pass(ROUND_TRIP);
            } else {
                let shown: Vec<&str> = wrong.iter().take(5).map(String::as_str).collect();
                report.fail(
                    ROUND_TRIP,
                    format!(
                        "{} 个写出来的文字 textPattern 认不回原表情（如 {}）；写不进文字的表情标 inText: false",
                        wrong.len(),
                        shown.join("、")
                    ),
                );
            }
        }
    }
    const ICONS: &str = "分类图标宿主认得";
    if catalog.categories.is_empty() {
        report.skip(ICONS, "目录不分类");
    } else {
        let unknown: Vec<&str> = catalog
            .categories
            .iter()
            .filter(|c| {
                c.icon.as_deref().is_some_and(|icon| {
                    !hhh_im_protocol::types::EMOJI_CATEGORY_ICONS.contains(&icon)
                })
            })
            .map(|c| c.id.as_str())
            .collect();
        if unknown.is_empty() {
            report.pass(ICONS);
        } else {
            report.fail(
                ICONS,
                format!(
                    "分类 {} 的 icon 不在宿主图标库里（会画成圆），可用：{}",
                    unknown.join("、"),
                    hhh_im_protocol::types::EMOJI_CATEGORY_ICONS.join("、")
                ),
            );
        }
    }
}

/// 契约 provider-protocol v12 的新方法：声明了就要能调。
async fn check_v12_calls(
    conn: &Arc<Connection>,
    report: &mut Report,
    caps: &Capabilities,
    online: bool,
) {
    if !caps.conversations.recent_activity {
        report.skip("会话发现能调", "没声明 conversations.recentActivity");
    } else if !online {
        report.skip("会话发现能调", "没登录，读不了");
    } else {
        let since = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
            - 86_400_000;
        // 照宿主的问法要第一页（v43），附上的要守条数与合计大小。
        const FIRST_PAGE_LIMIT: usize = 50;
        match call_t(
            conn,
            "conversations/recentActivity",
            json!({ "sinceMs": since, "pagesMax": 1, "firstPageLimit": FIRST_PAGE_LIMIT }),
            Duration::from_secs(30),
        )
        .await
        {
            Ok(v) => {
                match serde_json::from_value::<hhh_im_protocol::method::RecentActivityResult>(v) {
                    Ok(r) if r.items.iter().any(|i| i.conversation_id.is_empty()) => {
                        report.fail("会话发现能调", "有一项没有会话编号");
                    }
                    Ok(r) => {
                        let pages: Vec<_> = r
                            .items
                            .iter()
                            .filter_map(|i| i.first_page.as_ref())
                            .collect();
                        let bytes: usize = pages
                            .iter()
                            .map(|p| serde_json::to_vec(p).map_or(usize::MAX, |b| b.len()))
                            .fold(0, usize::saturating_add);
                        if pages.iter().any(|p| p.items.len() > FIRST_PAGE_LIMIT) {
                            report.fail("会话发现能调", "附上的第一页超过了要的条数");
                        } else if bytes > hhh_im_protocol::budget::RECENT_ACTIVITY_PAGES_BYTES_MAX {
                            report.fail("会话发现能调", "附上的第一页合起来超过预算");
                        } else {
                            report.pass("会话发现能调");
                        }
                    }
                    Err(e) => report.fail("会话发现能调", format!("结果格式不对：{e}")),
                }
            }
            Err(e) => report.fail("会话发现能调", e),
        }
    }
    if !caps.people.contacts {
        report.skip("好友目录只返回私聊资料", "没声明 people.contacts");
    } else if !online {
        report.skip("好友目录只返回私聊资料", "没登录，读不了");
    } else {
        match call_t(
            conn,
            "contacts/list",
            json!({"limit": 20}),
            Duration::from_secs(30),
        )
        .await
        {
            Ok(value) => match serde_json::from_value::<
                hhh_im_protocol::method::Page<hhh_im_protocol::types::Conversation>,
            >(value)
            {
                Ok(page)
                    if page.items.len() <= 20
                        && page.items.iter().all(|c| {
                            c.kind == hhh_im_protocol::types::ConversationKind::Direct
                                && !c.id.is_empty()
                                && c.peer.as_ref().is_some_and(|id| !id.is_empty())
                        }) =>
                {
                    report.pass("好友目录只返回私聊资料")
                }
                Ok(_) => report.fail(
                    "好友目录只返回私聊资料",
                    "条数超过请求上限，或有缺少编号/对方的非私聊",
                ),
                Err(error) => {
                    report.fail("好友目录只返回私聊资料", format!("结果格式不对：{error}"))
                }
            },
            Err(error) => report.fail("好友目录只返回私聊资料", error),
        }
    }
    if !caps.people.get {
        report.skip("人名册分清没有与没读到", "没声明 people.get");
    } else if !online {
        report.skip("人名册分清没有与没读到", "没登录，读不了");
    } else {
        // 问一个不存在的编号（声明了读机器人时人与机器人各一个）：结果读得懂，同一个编号不能既读到又“没有”（v15）。
        let mut params = json!({ "ids": ["hhh-im-check-nobody"] });
        if caps.people.bots {
            params["bots"] = json!(["hhh-im-check-nobot"]);
        }
        match call_t(conn, "people/get", params, Duration::from_secs(30)).await {
            Ok(v) => match serde_json::from_value::<hhh_im_protocol::method::PeopleGetResult>(v) {
                Ok(r) if r.found.iter().any(|p| r.missing.contains(&p.id)) => {
                    report.fail("人名册分清没有与没读到", "有编号既在 found 又在 missing 里");
                }
                Ok(_) => report.pass("人名册分清没有与没读到"),
                Err(e) => report.fail("人名册分清没有与没读到", format!("结果格式不对：{e}")),
            },
            Err(e) => report.fail("人名册分清没有与没读到", e),
        }
    }
    if caps.session.renew {
        // 拿一份假凭据叫它续期：回成功或回错误都算能答，超时不算。
        match tokio::time::timeout(
            Duration::from_secs(10),
            conn.call("session/renew", json!({ "credential": "hhh-im-check" })),
        )
        .await
        {
            Ok(_) => report.pass("续期能答"),
            Err(_) => report.fail("续期能答", "超过 10 秒没有响应"),
        }
    } else {
        report.skip("续期能答", "没声明 session.renew");
    }
}

/// 自检交给插件的样图：1×1 的 PNG。
fn host_avatar_sample() -> hhh_im_protocol::method::AccountAvatarParams {
    hhh_im_protocol::method::AccountAvatarParams {
        avatar: Some(hhh_im_protocol::method::HostAvatar {
            mime: "image/png".into(),
            data: "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGMor6z4DwAFOwJoUO6VrQAAAABJRU5ErkJggg==".into(),
            sha256: "1523d93314347302031d1b73aa7e78d3524608d2664f8f8e732b26e6392e51fc".into(),
            size: 70,
        }),
    }
}

/// 交一张样图（`account/avatar`），插件 10 秒内回成功才算能收。
async fn check_host_avatar(conn: &Arc<Connection>, report: &mut Report) {
    let params = serde_json::to_value(host_avatar_sample()).expect("样图参数总能写成 JSON");
    match call_t(conn, "account/avatar", params, Duration::from_secs(10)).await {
        Ok(_) => report.pass("宿主头像能收"),
        Err(e) => report.fail("宿主头像能收", e),
    }
}

fn check_pushed_messages(report: &mut Report, events: &[Value]) {
    let mut saw_message = false;
    let mut ms_ok = true;
    let mut sender_ok = true;
    let mut deleted_ok = true;
    // 视频封面（provider-protocol v9）：给了 cover 时键不能为空。
    let mut covers = 0;
    let mut cover_ok = true;
    for ev in events {
        if ev["type"] == "messages"
            && let Some(msgs) = ev["messages"].as_array()
        {
            for m in msgs {
                saw_message = true;
                // 时间看着像毫秒（晚于 2001-09-09，即 1e12 毫秒）。
                if m["createdAtMs"].as_i64().unwrap_or(0) < 1_000_000_000_000 {
                    ms_ok = false;
                }
                // 普通消息有发送者。
                let kind = m["sender"]["kind"].as_str().unwrap_or("");
                if kind != "system" && m["sender"]["id"].as_str().unwrap_or("").is_empty() {
                    sender_ok = false;
                }
                for block in m["content"].as_array().into_iter().flatten() {
                    if let Some(cover) = block["resource"].get("cover") {
                        covers += 1;
                        cover_ok &= cover["key"].as_str().is_some_and(|key| !key.is_empty());
                    }
                }
                // 撤回的消息内容为空。
                if m["deleted"].as_bool().unwrap_or(false)
                    && !m["content"]
                        .as_array()
                        .map(|c| c.is_empty())
                        .unwrap_or(true)
                {
                    deleted_ok = false;
                }
            }
        }
    }
    if saw_message {
        report.pass("推送里有消息");
        if ms_ok {
            report.pass("消息时间是毫秒");
        } else {
            report.fail("消息时间是毫秒", "createdAtMs 看起来不是毫秒");
        }
        if sender_ok {
            report.pass("普通消息有真实发送者");
        } else {
            report.fail("普通消息有真实发送者", "有非系统消息缺 sender.id");
        }
        if deleted_ok {
            report.pass("撤回的消息内容为空");
        } else {
            report.fail("撤回的消息内容为空", "有 deleted 消息仍带内容");
        }
        match (covers, cover_ok) {
            (0, _) => report.skip("视频封面键不为空", "推送里没有带封面的资源"),
            (_, true) => report.pass("视频封面键不为空"),
            (_, false) => report.fail("视频封面键不为空", "有资源的 cover.key 为空"),
        }
    } else {
        report.skip("推送消息检查", "推送里没有消息事件");
    }
}

fn find_resource(events: &[Value]) -> Option<(String, String, String, String)> {
    for ev in events {
        if ev["type"] == "messages" {
            for m in ev["messages"].as_array()? {
                let conv = m["conversationId"].as_str()?;
                let mid = m["id"].as_str()?;
                for b in m["content"].as_array()? {
                    if b["type"] == "resource" {
                        let key = b["resource"]["key"].as_str()?;
                        let kind = b["resource"]["kind"].as_str()?;
                        return Some((conv.into(), mid.into(), key.into(), kind.into()));
                    }
                }
            }
        }
    }
    None
}

/// 版本声明必须有效；每份实际字节都单独读一段，不把原件的大文件整份载入自检。
async fn check_resource_variants(
    conn: &Arc<Connection>,
    report: &mut Report,
    conv: &str,
    msg: &str,
    key: &str,
    kind: &str,
) {
    use base64::Engine as _;
    let params = json!({"conversationId":conv,"messageId":msg,"key":key,"kind":kind});
    let value = match conn.call("resources/describe", params.clone()).await {
        Ok(value) => value,
        Err(error) => {
            report.fail("资源版本说明", error.message);
            return;
        }
    };
    let description = match serde_json::from_value::<hhh_im_protocol::ResourceVariants>(value) {
        Ok(description) => description,
        Err(_) => {
            report.fail("资源版本说明", "结果不是规范的资源版本列表");
            return;
        }
    };
    if let Err(reason) = description.validate() {
        report.fail("资源版本说明", reason);
        return;
    }
    report.pass("资源版本说明合法且不重复原件");
    for variant in &description.variants {
        let mut read = params.clone();
        read["variant"] = variant.id.clone().into();
        read["offset"] = 0.into();
        read["length"] = (64 * 1024).into();
        let label = format!("资源版本 {} 可单独分段读取", variant.id);
        let value = match conn.call("resources/read", read).await {
            Ok(value) => value,
            Err(error) => {
                report.fail(&label, error.message);
                continue;
            }
        };
        let chunk = match serde_json::from_value::<hhh_im_protocol::method::ResourceChunk>(value) {
            Ok(chunk) => chunk,
            Err(_) => {
                report.fail(&label, "分段结果格式不正确");
                continue;
            }
        };
        let bytes = match base64::engine::general_purpose::STANDARD.decode(&chunk.data) {
            Ok(bytes) if !bytes.is_empty() && bytes.len() <= 64 * 1024 => bytes.len() as u64,
            _ => {
                report.fail(&label, "分段数据为空、超限或不能解码");
                continue;
            }
        };
        if chunk
            .total
            .is_some_and(|total| bytes > total || (chunk.eof && bytes != total))
            || variant
                .bytes
                .zip(chunk.total)
                .is_some_and(|(expected, actual)| expected != actual)
        {
            report.fail(&label, "版本大小或分段 eof 与实际数据不一致");
        } else {
            report.pass(&label);
        }
    }
}

async fn check_resource_read(
    conn: &Arc<Connection>,
    report: &mut Report,
    conv: &str,
    msg: &str,
    key: &str,
    kind: &str,
) {
    let mut offset = 0u64;
    let mut total_bytes = 0usize;
    let mut declared_total: Option<u64> = None;
    for _ in 0..64 {
        let r = conn
            .call(
                "resources/read",
                json!({
                    "conversationId": conv, "messageId": msg, "key": key, "kind": kind,
                    "offset": offset, "length": 64 * 1024
                }),
            )
            .await;
        let chunk = match r {
            Ok(v) => v,
            Err(e) => {
                report.fail("资源分段读", e.message);
                return;
            }
        };
        if let Some(t) = chunk["total"].as_u64() {
            declared_total = Some(t);
        }
        let data = chunk["data"].as_str().unwrap_or("");
        total_bytes += base64_len(data);
        offset = total_bytes as u64;
        if chunk["eof"].as_bool().unwrap_or(true) {
            break;
        }
    }
    match declared_total {
        Some(t) if t as usize == total_bytes => report.pass("资源分段读的总长一致、eof 正确"),
        Some(t) => report.fail(
            "资源分段读的总长一致、eof 正确",
            format!("声明 {t}，实读 {total_bytes}"),
        ),
        None => report.pass("资源分段读到 eof（平台未给总长）"),
    }
}

async fn check_send(conn: &Arc<Connection>, report: &mut Report, conv: &str, attribution: bool) {
    let op = format!("check-{}", std::process::id());
    let mut params = json!({
        "operationId": op,
        "target": { "type": "conversation", "conversationId": conv },
        "content": [ { "type": "text", "text": "hhh-im-check 测试消息" } ]
    });
    let signature = json!({"name":"HHH 自检伙伴","automatic":true});
    if attribution {
        params["agentAttribution"] = signature.clone();
    }
    let r = conn.call("messages/send", params).await;
    match r {
        Ok(v) if v["state"] == "accepted" || v["state"] == "unknown" => {
            report.pass("发送返回 accepted 或 unknown");
            if attribution {
                if v["state"] == "unknown" {
                    report.skip("伙伴署名发送回包", "平台未确认是否发出，不能判定署名已保留");
                } else if v["message"]["agentAttribution"] == signature {
                    report.pass("伙伴署名发送回包");
                } else {
                    report.fail(
                        "伙伴署名发送回包",
                        "已声明传递能力，但发送回包未保留伙伴署名",
                    );
                }
            }
        }
        Ok(v) => report.fail(
            "发送返回 accepted 或 unknown",
            format!("state={:?}", v["state"]),
        ),
        Err(e) => report.fail("发送返回 accepted 或 unknown", e.message),
    }
}

async fn call_t(
    conn: &Arc<Connection>,
    method: &str,
    params: Value,
    timeout: Duration,
) -> Result<Value, String> {
    match tokio::time::timeout(timeout, conn.call(method, params)).await {
        Ok(Ok(v)) => Ok(v),
        Ok(Err(e)) => Err(e.message),
        Err(_) => Err(format!("超过 {} 秒没有响应", timeout.as_secs())),
    }
}

fn resolve_command(dir: &std::path::Path, command: &str) -> String {
    let p = std::path::Path::new(command);
    if p.is_absolute() {
        return command.to_string();
    }
    let joined = dir.join(command);
    if joined.exists() {
        return joined.to_string_lossy().into_owned();
    }
    // Windows：清单的命令不写扩展名时试 `<命令>.exe`（同宿主）。
    let exe = dir.join(format!("{command}.exe"));
    if cfg!(windows) && exe.exists() {
        return exe.to_string_lossy().into_owned();
    }
    // 当作 PATH 里的程序（node、python 等）。
    command.to_string()
}

/// 估算一段 base64 解码后的字节数（忽略换行与填充）。
#[cfg(test)]
mod v12_tests {
    use super::*;

    fn manifest() -> Manifest {
        serde_json::from_value(json!({
            "manifestVersion": 1, "id": "sample", "name": "示例", "version": "0.1.0", "command": "p", "os": ["windows"]
        }))
        .unwrap()
    }

    fn rows(report: &Report) -> Vec<(String, &'static str)> {
        report
            .rows
            .iter()
            .map(|(name, st, _)| {
                (
                    name.clone(),
                    match st {
                        Status::Pass => "pass",
                        Status::Fail => "fail",
                        Status::Skip => "skip",
                    },
                )
            })
            .collect()
    }

    fn status_of(report: &Report, name: &str) -> &'static str {
        rows(report)
            .into_iter()
            .find(|(n, _)| n == name)
            .map_or("missing", |(_, s)| s)
    }

    // 契约 provider-protocol v36：登录步骤合格——合规的登录窗口一步通过，不合格的开头或起始网址失败，没收到步骤跳过。
    #[test]
    fn v36_login_steps_pass_fail_and_skip() {
        let good = json!({ "kind": "loginWindow", "url": "http://127.0.0.1:9/window?state=s", "label": "打开登录窗口",
            "capture": ["hhh-sample://"] });
        let mut report = Report::new();
        check_login_steps(&mut report, std::slice::from_ref(&good));
        assert_eq!(status_of(&report, "登录步骤合格"), "pass");
        let bad_capture = json!({ "kind": "loginWindow", "url": "https://a.test/s", "label": "打开", "capture": ["https://example.com/"] });
        let mut report = Report::new();
        check_login_steps(&mut report, &[good.clone(), bad_capture]);
        assert_eq!(status_of(&report, "登录步骤合格"), "fail");
        assert!(
            report.rows[0].2.contains("跳转开头"),
            "{}",
            report.rows[0].2
        );
        let bad_url = json!({ "kind": "loginWindow", "url": "http://a.test/s", "label": "打开", "capture": ["a://"] });
        let mut report = Report::new();
        check_login_steps(&mut report, &[bad_url]);
        assert!(
            report.rows[0].2.contains("起始网址"),
            "{}",
            report.rows[0].2
        );
        let unreadable = json!({ "kind": "loginWindow", "url": "https://a.test/s" });
        let mut report = Report::new();
        check_login_steps(&mut report, &[unreadable]);
        assert_eq!(status_of(&report, "登录步骤合格"), "fail");
        let mut report = Report::new();
        check_login_steps(&mut report, &[]);
        assert_eq!(status_of(&report, "登录步骤合格"), "skip");
    }

    // 每项新检查一正一反（契约 provider-protocol v12）：没声明跳过，写对通过，写错失败。
    #[test]
    fn v12_declarations_pass_fail_and_skip() {
        let desc = |extra: Value| -> PlatformDesc {
            let mut base = json!({ "avatarShape": "rounded", "history": "none", "actions": {} });
            for (k, v) in extra.as_object().unwrap() {
                base[k] = v.clone();
            }
            serde_json::from_value(base).unwrap()
        };
        let mut report = Report::new();
        check_v12_declarations(&mut report, &manifest(), &desc(json!({})), &json!({}));
        for name in [
            "开放的读取项认得出",
            "按内容认编辑的取值认得出",
            "重译规则名不重复",
            "正文表情写法能编译",
            "头像身份写法能编译",
            "标志外框在画布内",
            "登录方式写法对",
            "回调路径在本 IM 目录下",
        ] {
            assert_eq!(status_of(&report, name), "skip", "{name}");
        }

        let good = desc(json!({
            "userAdjustable": ["readIntervals", "readsPerMinute"], "editsByContent": ["text"],
            "retranslateRules": [{ "name": "a" }, { "name": "b" }],
            "emoji": { "unlisted": { "placeholderLabel": "表情" }, "textPattern": "\\{([^{}\\r\\n]{1,12})\\}" },
            "avatarIdentity": { "keyPattern": "/(v3_[^/?#]+)", "variantPattern": "[?&]image_size=([^&#]+)" }
        }));
        let good_init = json!({ "loginMethods": [
            { "id": "relay", "kind": "relay", "label": "中继" },
            { "id": "own", "kind": "ownApp", "label": "自己的应用", "callbackPath": "callback" }
        ] });
        let mut marked = manifest();
        marked.logo_ink =
            serde_json::from_value(json!({ "x": 1, "y": 1, "width": 20, "height": 20 })).unwrap();
        let mut report = Report::new();
        check_v12_declarations(&mut report, &marked, &good, &good_init);
        assert!(
            rows(&report).iter().all(|(_, s)| *s == "pass"),
            "{:?}",
            rows(&report)
        );

        let bad = desc(json!({
            "userAdjustable": ["readIntervals", "somethingNew"], "editsByContent": ["card"],
            "retranslateRules": [{ "name": "a" }, { "name": "a" }],
            "emoji": { "unlisted": { "placeholderLabel": "表情" }, "textPattern": "\\{[^}]+\\}" },
            "avatarIdentity": { "keyPattern": "/v3_[^/?#]+" }
        }));
        let bad_init = json!({ "loginMethods": [
            { "id": "x", "kind": "passkey", "label": "通行密钥" },
            { "id": "own", "kind": "ownApp", "label": "自己的应用", "callbackPath": "/hhh-friend-im/feishu/callback" }
        ] });
        let mut broken = manifest();
        broken.logo_ink =
            serde_json::from_value(json!({ "x": 1, "y": 1, "width": 0, "height": 20 })).unwrap();
        let mut report = Report::new();
        check_v12_declarations(&mut report, &broken, &bad, &bad_init);
        assert!(
            rows(&report).iter().all(|(_, s)| *s == "fail"),
            "{:?}",
            rows(&report)
        );
        assert!(pattern_problem("(", "表情名").is_some(), "编不过");
    }

    // 契约 provider-protocol v31：写法没声明跳过、写对通过、写错失败；目录里写进文字的要认得回来，标了不能写进文字的不查；
    // 分类图标不在图标库里失败。
    #[test]
    fn v31_text_template_and_catalog_checks() {
        let desc = |emoji: Value| -> PlatformDesc {
            serde_json::from_value(json!({ "avatarShape": "rounded", "history": "none", "actions": {}, "emoji": emoji }))
                .unwrap()
        };
        let unlisted = json!({ "placeholderLabel": "表情" });
        let catalog = |value: Value| -> hhh_im_protocol::method::EmojiCatalogResult {
            serde_json::from_value(value).unwrap()
        };
        let mut report = Report::new();
        check_v31_declarations(&mut report, &desc(json!({ "unlisted": unlisted })));
        assert_eq!(status_of(&report, "表情写法格式对"), "skip");
        let mut report = Report::new();
        check_v31_declarations(
            &mut report,
            &desc(json!({ "unlisted": unlisted, "textTemplate": "[{name}]" })),
        );
        assert_eq!(status_of(&report, "表情写法格式对"), "pass");
        let mut report = Report::new();
        check_v31_declarations(
            &mut report,
            &desc(json!({ "unlisted": unlisted, "textTemplate": "[赞]" })),
        );
        assert_eq!(status_of(&report, "表情写法格式对"), "fail");

        // 名字写法：认得回来通过；不能写进文字的、没有名字的不查。
        let named = desc(
            json!({ "unlisted": unlisted, "textTemplate": "[{name}]", "textPattern": "\\[([^\\[\\]\\r\\n]{1,12})\\]" }),
        );
        let feishu_like = catalog(json!({ "updatedAtMs": 1, "emojis": [
            { "emojiType": "THUMBSUP", "name": "赞" },
            { "emojiType": "2021", "name": "这个名字太长了写进去认不回来一二三", "inText": false },
            { "emojiType": "NONAME" }
        ] }));
        let mut report = Report::new();
        check_v31_catalog(&mut report, &named, &feishu_like);
        assert_eq!(
            status_of(&report, "写进文字的表情认得回来"),
            "pass",
            "{:?}",
            rows(&report)
        );
        assert_eq!(status_of(&report, "分类图标宿主认得"), "skip");

        // 编号写法：肤色版本也要整段认出；认出的不是原编号就失败。
        let slack_like = catalog(json!({ "updatedAtMs": 1, "emojis": [
            { "emojiType": "+1" }, { "emojiType": "wave::skin-tone-3", "panel": false }
        ], "categories": [{ "id": "people", "name": "人物", "icon": "smileys" }, { "id": "mine", "name": "自定义" }] }));
        let typed = |pattern: &str| {
            desc(
                json!({ "unlisted": unlisted, "textTemplate": ":{type}:", "textPattern": pattern }),
            )
        };
        let mut report = Report::new();
        check_v31_catalog(
            &mut report,
            &typed(":([a-z0-9_+'-]+(?:::skin-tone-[2-6])?):"),
            &slack_like,
        );
        assert_eq!(
            status_of(&report, "写进文字的表情认得回来"),
            "pass",
            "{:?}",
            rows(&report)
        );
        assert_eq!(
            status_of(&report, "分类图标宿主认得"),
            "pass",
            "没给图标的分类画圆，不算错"
        );
        let mut report = Report::new();
        check_v31_catalog(&mut report, &typed(":([a-z0-9_+'-]+):"), &slack_like);
        let row = report
            .rows
            .iter()
            .find(|(n, _, _)| n == "写进文字的表情认得回来")
            .unwrap();
        assert!(
            matches!(row.1, Status::Fail) && row.2.contains(":wave::skin-tone-3:"),
            "{}",
            row.2
        );

        // 只写了写法、没写正文写法：跳过；分类图标不认识：失败。
        let odd_icon = catalog(
            json!({ "updatedAtMs": 1, "emojis": [], "categories": [{ "id": "a", "name": "甲", "icon": "rocket" }] }),
        );
        let mut report = Report::new();
        check_v31_catalog(
            &mut report,
            &desc(json!({ "unlisted": unlisted, "textTemplate": ":{type}:" })),
            &odd_icon,
        );
        assert_eq!(status_of(&report, "写进文字的表情认得回来"), "skip");
        assert_eq!(status_of(&report, "分类图标宿主认得"), "fail");
    }

    // 契约 provider-protocol v13：操作声明没声明跳过、写对通过、写错失败；声明了 host 登录时给出“只有官方插件能用”的提示。
    #[test]
    fn v13_declarations_pass_fail_and_skip() {
        let mut report = Report::new();
        check_v13_declarations(&mut report, &json!({}));
        assert_eq!(status_of(&report, "操作声明写法对"), "skip");
        assert_eq!(status_of(&report, "用宿主账号登录"), "skip");

        let mut report = Report::new();
        check_v13_declarations(
            &mut report,
            &json!({
                "actions": [
                    { "id": "devices", "label": "我的电脑", "scope": "account" },
                    { "id": "members", "label": "群成员", "scope": "conversation", "conversationKinds": ["group"] }
                ],
                "loginMethods": [{ "id": "cindy", "kind": "host", "label": "用 Cindy 账号登录" }]
            }),
        );
        assert_eq!(status_of(&report, "操作声明写法对"), "pass");
        let hint = report
            .rows
            .iter()
            .find(|(n, _, _)| n == "用宿主账号登录")
            .unwrap();
        assert!(hint.2.contains("官方"), "{}", hint.2);

        for bad in [
            json!([{ "id": "a1", "label": "甲", "scope": "account" }, { "id": "a1", "label": "乙", "scope": "account" }]),
            json!([{ "id": "Bad Id", "label": "甲", "scope": "account" }]),
            json!([{ "id": "a1", "label": "这是一个超过二十个字的菜单项文字一二三四五六", "scope": "account" }]),
            json!([{ "id": "a1", "label": "甲", "scope": "toolbar" }]),
            json!(
                (0..17)
                    .map(|i| json!({ "id": format!("a{i}"), "label": "项", "scope": "account" }))
                    .collect::<Vec<_>>()
            ),
        ] {
            let mut report = Report::new();
            check_v13_declarations(&mut report, &json!({ "actions": bad }));
            assert_eq!(status_of(&report, "操作声明写法对"), "fail", "{bad}");
        }
    }
}

fn base64_len(s: &str) -> usize {
    let clean: usize = s.chars().filter(|c| !c.is_whitespace()).count();
    let pad = s.chars().rev().take_while(|&c| c == '=').count();
    if clean == 0 { 0 } else { clean / 4 * 3 - pad }
}

#[cfg(test)]
mod tests {
    use super::base64_len;

    // 自检交的样图本身合规（类型、大小、摘要、base64 长度对得上），插件不会因样图不合规而回错。
    #[test]
    fn the_host_avatar_sample_is_well_formed() {
        let sample = super::host_avatar_sample().avatar.unwrap();
        assert_eq!(sample.problem(), None);
        assert_eq!(base64_len(&sample.data), sample.size as usize);
    }

    #[test]
    fn base64_len_counts_decoded_bytes() {
        assert_eq!(base64_len(""), 0);
        assert_eq!(base64_len("AAAA"), 3);
        assert_eq!(base64_len("AA=="), 1);
        assert_eq!(base64_len("AAA="), 2);
    }
}
