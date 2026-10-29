//! 示例 IM：把契约 provider-protocol v12、v13 的新声明与新方法都用上的演示插件，验收与插件作者参照用。
//!
//! - 三种登录方式：通过中继登录、使用自己的应用（声明本机回调路径）、在登录窗口里登录（v36）。插件自带一个本机回环的假授权页：
//!   前两种推“打开登录页”一步，页上点“同意”——中继方式由插件自己完成登录；自己的应用方式带着 `code`、`state` 跳回宿主给的
//!   回调地址，宿主经 `login/callback` 交回来，核对 `state` 后完成。登录窗口方式推“登录窗口”一步（起始网址是假登录页
//!   `/window`、要截的开头 `hhh-sample://`），页上点“完成登录”跳到 `hhh-sample://done?state=…&code=…`，宿主截下、经
//!   `login/callback` 交回来；链接对不上这次登录或没有登录码时 `login/failed` 说原因。页上另有一个 `other-app://` 链接（宿主
//!   应当拦下）。参数 `--login-window-url <网址>` 或工作目录里的 `login-window-url` 写了网址就拿它当起始网址（看真实网页在窗口里的样子），`login-window-bad-capture`
//!   在时（或带参数 `--bad-capture`）故意声明不合格的开头（宿主与 `hhh-im-check --login window` 都应拒绝）。存凭据时带 `renewAtMs`（60 秒后），宿主到时
//!   叫 `session/renew`。
//!   假登录页、假授权页与之后的几页都用 HHH 收据页的版式（v37，见 [`sample_page`]），不引用任何外部资源。
//! - 会话发现（`conversations/recentActivity`）、开放读取频率与“每分钟最多读几次”（默认 120）、按内容认编辑、正文表情写法
//!   `{名字}` 与白底表情图、每条消息自己说能不能编辑（`editDraft`）、一对话题副本、按规则名重译、续期、账号的工作区。
//! - 平台是 `history=server`：宿主按读取频率来读（`messages/list`），每次读都在标准错误里写一行“距上次 N 秒”，看读取间隔用。
//! - 会话：群“示例团队群”（你是群主）、群“示例读书会”（群主阿博，你是普通成员）、私聊“阿美”，登录后推给宿主；群“示例公告”
//!   （群主阿博）不推，留给会话发现。好友：阿美、阿博、小林、大卫（“加个朋友”加上王小明）。
//!
//! v13（契约 provider-protocol v13 §3、§6）：
//!
//! - **用宿主账号登录**：`--host` 时 `loginMethods` 只声明这一种（`kind: host`，编号 `cindy`），`--host-extra` 时声明它再加上
//!   原来的两种（演示“还声明了别的方式的插件照常有登录”）；不带这两个参数照旧。`login/start` 选它时要有 `values.hostToken`，
//!   没有就 `login/failed`“没收到宿主的令牌”（不能重试）；令牌不打印，标准错误里只写它的长度与 `hostAccount.name`。之后先推
//!   一步“正在开通…”（`message`），再看工作目录里的开关文件：都没有——存凭据（账号名取 `hostAccount.name`，之后
//!   `session/open` 从凭据里取回同一个账号）、等宿主回了“存好了”再 `login/finished`，不带标题（自动登录时宿主什么都不弹）；
//!   `host-key`——再推出示恢复密钥一步（`showText`：12 个编号的词、能复制、勾“我已保存恢复密钥”才能点“完成”、不能跳过），点了
//!   “完成”存凭据，`login/finished` 带成功页的标题与一段话；`host-fail`——`login/failed`（能重试，带失败页标题“没开通成”与
//!   重试按钮“再试一次”）。这两个文件留着不删（测试来回切）。
//! - **插件操作**（除 `--plain` 外都声明）：管理菜单里“我的设备”“我的链接”“加个朋友”“关于示例”，群的会话菜单里“群成员”
//!   “邀请成员”“退出群”（`conversationKinds: ["group"]`），再加不写种类、私聊与群都有的“删除会话”（v19）。`actions/start` 回
//!   `{flowId}`，第一步在回复之后推；新的 `actions/start` 或 `login/start` 取消还没结束的旧流程，`action/cancel` 静静结束。各操作：
//!   - 我的设备：设备列表（本机标“本机”、没有按钮，别的设备行尾“移除”，危险色），下面“重新生成恢复密钥”。移除 → 确认
//!     “移除「…」？”（危险）→ 移除后回到列表、带一句提示；重新生成 → 确认 → 出示新的 12 个词（不能跳过）→ 回到列表。
//!   - 我的链接：出示好友链接（一行等宽、能复制），下面“换一条链接” → 确认 → 出示新链接、带一句提示。
//!   - 加个朋友：填好友链接（“下一步”）；不以 `https://` 开头就再推这一步、带警告；对了 → 核对“王小明”（身份指纹）→ “加好友”
//!     → 成功页“已添加王小明”。
//!   - 关于示例：几段带小标题的说明，只有“知道了”（不能跳过），点了对话框直接关掉。
//!   - 群成员：成员列表（群主标“群主”）；你是群主时别人行尾有“移出”（危险）→ 确认 → 移出后回到列表，不是群主看不到“移出”。
//!   - 邀请成员：勾选不在群里的好友（至少 1 个，“邀请”），下面“改用群邀请链接” → 出示群链接，下面“改回勾选好友”；勾好确认
//!     → 成功页“已邀请”。
//!   - 退出群：确认（危险，“退出群”）→ 成功页；只在内存里记下，重启插件后回到群里。
//!   - 删除会话：确认“删除「…」？”（不危险，“删除”）→ `action/finished { removeFromList: true }`，不给标题与说明（对话框直接
//!     关掉、会话离开宿主的列表）；平台上什么都不动，这个会话再来新消息时宿主的会话发现把它带回来。
//!   - 各步的确认（移除、换链接、核对是谁、移出）点“取消”回到上一步（`cancelChoice: back`）。
//! - **请你处理的事**：`approve` 开关让新设备「示例平板」来请求批准：`session/notice` 里多一条带“查看”（`action.id` 是
//!   `approve:1`，不在 `initialize.actions` 里）与 `prompt` 的提醒，处理完之前每次给全提醒时都带着它。`actions/start
//!   {actionId: "approve:1"}` 回对话框标题“批准新设备”，推填 4 个词的表单（“拒绝 / 批准”，“拒绝”是一个选择）；对的词是
//!   `松果 飞机 河流 积木`（空白不论多少）：对了成功页“已批准「示例平板」”、设备列表多一台、提醒撤掉；错了再推这一步、带
//!   “还能试 N 次”的警告，错满 5 次失败“请求已作废”、提醒撤掉；点“拒绝”失败“已拒绝”、提醒撤掉（都不能重试）。
//!
//! 工作目录（宿主给的临时目录）里出现这些文件时照做：`expire` 说登录失效；`notice` 报一条要重新登录的问题、`notice-clear`
//! 清掉它；`approve` 来一条等批准的新设备请求、`approve-clear` 撤掉它（批准框开着就以“请求已作废”结束）；`new-message` 在
//! 没关注的“示例公告”里来一条新消息（会话发现用；文件里写了会话编号如 `c-direct` 就来在那个会话里）；`edit` 改掉第一条的文字（按内容认编辑用）——以上做完删掉文件。
//! `deny-reactions` 在时读表情一律回没有权限；`rule-v2` 在时多声明一条重译规则 `sample-v2`（下次打开生效）；`host-key`、
//! `host-fail` 见上文（用宿主账号登录时看，不删）。
//!
//! 凭据是一段 JSON：签发时间与账号。交接时沿用内置平台核验过的账号（同编号共用一份本机数据，按账号分库），之后每次打开、
//! 续期都从凭据里取回这个账号。
//!
//! 参数：`--id <编号>` 换插件编号（同一台电脑装两份时用）；`--reserved feishu` 用保留编号 `feishu`（交接验收用：
//! `session/open` 收到 `legacy` 时先存自己的凭据，再回在线与 `adoptedSettings`）；`--plain` 什么都不开放（参数配置里没有它，
//! 也不声明操作）；`--callback-path <路径>` 换掉“使用自己的应用”声明的回调路径（演示 `hhh-im-check` 报出不在本 IM 目录下的
//! 路径）；`--host` / `--host-extra` 见上文。
//! 第一条消息上有一个目录外的表情 `wave`：声明了猜图，宿主来要图时给，别的目录外类型回没有。表情图都是 32×32 的白底
//! 色块圆（声明了 `whiteBackground`，宿主显示前抠掉白底）。
//!
//! 运行：`cargo run -p hhh-im-protocol --example sample_provider --features transport`（通常由 HHH Friend 作为子进程启动）。
//! 不联网、全在内存。

#[path = "sample_provider/reading.rs"]
mod reading;

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

use hhh_im_protocol::rpc::{RpcError, RpcId, RpcMessage, codec};
use serde_json::{Value, json};

/// “赞”的表情图：白底上蓝色圆（32×32，白底，宿主按 `whiteBackground` 抠掉）。
const THUMBSUP_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0x20, 0x08, 0x02, 0x00, 0x00, 0x00, 0xFC, 0x18, 0xED,
    0xA3, 0x00, 0x00, 0x00, 0x66, 0x49, 0x44, 0x41, 0x54, 0x78, 0xDA, 0xED, 0xD6, 0xB9, 0x0D, 0x00,
    0x20, 0x0C, 0x03, 0x40, 0xEF, 0xC4, 0x62, 0xEC, 0xDF, 0x84, 0x9E, 0x4F, 0x41, 0x18, 0x4B, 0x40,
    0x10, 0xB5, 0xAF, 0x09, 0xC4, 0xB0, 0xC3, 0x07, 0x7F, 0x00, 0x29, 0xF7, 0x2F, 0x01, 0x18, 0x45,
    0xFB, 0x19, 0xEC, 0x44, 0x7B, 0x18, 0x50, 0xD2, 0x27, 0x06, 0x58, 0xE9, 0x23, 0x03, 0xC4, 0xF4,
    0xAE, 0xA1, 0x05, 0xF6, 0xD3, 0x5B, 0x43, 0x08, 0xB0, 0xD2, 0x2B, 0xC3, 0x0B, 0xAC, 0x0E, 0x5B,
    0x00, 0x01, 0x1C, 0x19, 0xD3, 0xFB, 0x5F, 0xF2, 0x13, 0xBF, 0xA9, 0x62, 0xE1, 0x28, 0x56, 0xA6,
    0x62, 0xE9, 0x2B, 0x6A, 0x8B, 0xA8, 0x78, 0x45, 0xF9, 0x35, 0x2B, 0x64, 0x65, 0x49, 0x60, 0xC8,
    0x25, 0xFE, 0x0F, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

/// “微笑”的表情图：白底上黄色圆。
const SMILE_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0x20, 0x08, 0x02, 0x00, 0x00, 0x00, 0xFC, 0x18, 0xED,
    0xA3, 0x00, 0x00, 0x00, 0x6B, 0x49, 0x44, 0x41, 0x54, 0x78, 0xDA, 0xED, 0xD6, 0xC1, 0x0D, 0xC0,
    0x20, 0x0C, 0x03, 0x40, 0x0F, 0xD6, 0x49, 0x3A, 0x27, 0xAB, 0xF5, 0x9D, 0xFE, 0x81, 0x54, 0x91,
    0xE2, 0xBA, 0x82, 0x06, 0xE5, 0xED, 0xFB, 0x00, 0x31, 0xEC, 0xE5, 0x83, 0x7F, 0x00, 0x57, 0xC3,
    0x74, 0x08, 0x80, 0x17, 0x1D, 0x67, 0x90, 0x89, 0x8E, 0x30, 0xA0, 0xA4, 0x3F, 0x18, 0x60, 0xA5,
    0x7B, 0x06, 0x88, 0xE9, 0x53, 0x43, 0x0B, 0xE4, 0xD3, 0x47, 0x43, 0x08, 0xB0, 0xD2, 0x3B, 0x23,
    0x0A, 0x9C, 0x47, 0x3F, 0x05, 0x14, 0xF0, 0xC1, 0x35, 0x5D, 0xFF, 0x25, 0x6F, 0xF1, 0x9B, 0x2A,
    0x16, 0x8E, 0x62, 0x65, 0x2A, 0x96, 0xBE, 0xA2, 0xB6, 0x88, 0x8A, 0x57, 0x95, 0x5F, 0xB3, 0x1B,
    0x08, 0x2A, 0x00, 0xAC, 0xD3, 0x60, 0x42, 0x37, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44,
    0xAE, 0x42, 0x60, 0x82,
];

/// 目录外的 `wave`：白底上橙色圆（猜图给的）。
const WAVE_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0x20, 0x08, 0x02, 0x00, 0x00, 0x00, 0xFC, 0x18, 0xED,
    0xA3, 0x00, 0x00, 0x00, 0x63, 0x49, 0x44, 0x41, 0x54, 0x78, 0xDA, 0xED, 0xD6, 0xC1, 0x0A, 0x00,
    0x20, 0x08, 0x03, 0xD0, 0xFD, 0x78, 0xDF, 0x6E, 0xE7, 0x48, 0xC1, 0x68, 0x0D, 0x2A, 0xA3, 0xF3,
    0xDE, 0xC5, 0x72, 0xB0, 0xC3, 0x07, 0x9F, 0x00, 0x0D, 0xFE, 0x25, 0x00, 0x51, 0x74, 0x9A, 0xC1,
    0x56, 0x74, 0x82, 0x01, 0x27, 0x3D, 0x36, 0x40, 0x4B, 0x0F, 0x0C, 0x30, 0xD3, 0x3D, 0x43, 0x0C,
    0xEC, 0xA7, 0x4F, 0x86, 0x12, 0x60, 0xA5, 0x8F, 0x46, 0x1A, 0x58, 0x1D, 0xB6, 0x02, 0x0A, 0x38,
    0x32, 0xA6, 0xF7, 0xBF, 0xE4, 0x27, 0x7E, 0x53, 0xC5, 0xC2, 0x51, 0xAC, 0x4C, 0xC5, 0xD2, 0x57,
    0xD4, 0x16, 0x51, 0xF1, 0xAA, 0xF2, 0x6B, 0x1D, 0x44, 0xAF, 0xFE, 0x81, 0xF2, 0x7D, 0x87, 0x2C,
    0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

/// 自己的应用方式里，假授权页交回的授权码。
const CODE: &str = "sample-code";
/// 续期间隔。
const RENEW_AFTER_MS: i64 = 60_000;
/// 用宿主账号登录这种方式的编号（`loginMethods` 里的 `id`）。
const HOST_METHOD: &str = "cindy";
/// 自己在群成员、消息里的编号（用宿主账号登录时也是它，只换名字）。
const ME: &str = "me";
/// 批准新设备时要对上的 4 个词。
const APPROVE_WORDS: &str = "松果 飞机 河流 积木";
/// 批准新设备最多输错几次，再错这次请求就作废。
const APPROVE_TRIES: u32 = 5;
/// 等批准的提醒上“查看”对应的操作编号（不在 `initialize.actions` 里）。
const APPROVE_ACTION: &str = "approve:1";
/// 会话菜单里“删除会话”的操作编号（v19，私聊与群都有）。
const DELETE_ACTION: &str = "delete-conversation";
/// 恢复密钥从这些词里挑 12 个（每次重新生成换一组）。
const KEY_WORDS: [&str; 24] = [
    "月光", "森林", "灯塔", "茶杯", "风筝", "雪山", "铅笔", "海豚", "草莓", "钟表", "云朵", "石桥",
    "竹林", "帆船", "蜡烛", "围巾", "橘子", "铃铛", "书架", "彩虹", "枫叶", "陶罐", "邮票", "星星",
];
/// 好友链接末尾的几种写法（“换一条链接”轮着换）。
const LINK_CODES: [&str; 3] = ["7K3P-Q9XD", "M2VA-8RTE", "C5WN-4HJB"];

type Out = Arc<Mutex<std::io::Stdout>>;
/// 插件发给宿主、正在等回复的请求：编号 → 收回复的一头（续期要等凭据存好再回宿主）。
type Waiting = Arc<Mutex<BTreeMap<i64, std::sync::mpsc::Sender<RpcMessage>>>>;

/// 一次正在进行的登录：宿主给的 `state` → 流程编号与回调地址（自己的应用方式才有）；登录窗口方式（v36）记下是它。
struct Flow {
    flow_id: String,
    callback: Option<String>,
    window: bool,
}

/// 登录窗口方式（契约 provider-protocol v36）的编号与要截的跳转开头；假登录页上“完成登录”跳到这个开头的链接。
const WINDOW_METHOD: &str = "window";
const WINDOW_CAPTURE: &str = "hhh-sample://";

/// 用宿主账号登录声明成什么样（`--host`、`--host-extra`）。
#[derive(Clone, Copy, PartialEq, Eq)]
enum HostMethod {
    /// 不声明（照旧两种）。
    Off,
    /// 只声明这一种。
    Only,
    /// 声明它，再加上原来的两种。
    Extra,
}

/// 一台登录着的设备（“我的设备”里的一行）。
struct Device {
    id: String,
    name: String,
    detail: String,
    /// 本机：没有“移除”。
    this: bool,
}

/// 一个群的群主与成员（会话名照 [`conversation`]）。
struct Group {
    owner: String,
    members: Vec<String>,
}

/// 正在走的宿主账号登录或插件操作（同一时间只有一个，协议 §6.1、§6.7）。中继、自己的应用两种登录照旧记在
/// `State::flows` 里（假授权页按 `state` 找它们）。
#[derive(Clone)]
struct Active {
    flow_id: String,
    /// `login` 或 `action`：步骤、成败用哪一组方法名。
    kind: &'static str,
    /// 停在哪一步。
    stage: Stage,
}

/// 流程停在哪一步、等用户做什么（会话级操作带会话编号）。
#[derive(Clone)]
enum Stage {
    /// 宿主账号登录：推了“正在开通…”，另一条线程接着做。
    HostOpening,
    /// 宿主账号登录：出示了恢复密钥，等“完成”（带这次要登录的账号）。
    HostKey(Value),
    /// 宿主账号登录：点了“完成”，正在存凭据。
    HostSaving,
    /// 我的设备：列表。
    Devices,
    /// 我的设备：确认移除这一台。
    DeviceRemove(String),
    /// 我的设备：确认重新生成恢复密钥。
    RegenConfirm,
    /// 我的设备：出示新的恢复密钥。
    RegenKey,
    /// 我的链接：出示链接。
    Link,
    /// 我的链接：确认换一条。
    LinkRenew,
    /// 加个朋友：填链接。
    AddInput,
    /// 加个朋友：核对是谁。
    AddConfirm,
    /// 关于示例。
    About,
    /// 群成员：列表。
    Members(String),
    /// 群成员：确认移出这个人（会话、人）。
    MemberRemove(String, String),
    /// 邀请成员：勾选好友。
    Invite(String),
    /// 邀请成员：出示群邀请链接。
    InviteLink(String),
    /// 退出群：确认。
    Leave(String),
    /// 删除会话：确认（v19）。
    Delete(String),
    /// 批准新设备：填 4 个词。
    Approve,
}

/// 回复宿主之后要做的事（步骤不能比 `login/start`、`actions/start` 的回复先到）。
enum Follow {
    /// 发一个通知。
    Notify(String, Value),
    /// 用宿主账号开通（见 [`host_login`]）。
    HostLogin {
        flow_id: String,
        account: Value,
        token_ok: bool,
    },
}

/// 用户在一步里做的事：`step/choose`，或表单提交（`login/submit`、`action/submit`）。
enum Input<'a> {
    Choose {
        choice: &'a str,
        row: &'a str,
        selected: Vec<&'a str>,
    },
    Submit(&'a Value),
}

/// 一步之后怎么走（见 [`advance`]）。
enum Next {
    /// 推下一步，停在那一步。
    Step(Stage, Value),
    /// 做完：`…/finished` 的参数（不含流程编号）。
    Finished(Value),
    /// 没做成：`…/failed` 的参数（不含流程编号）。
    Failed(Value),
    /// 宿主账号登录：存凭据后以 `login/finished` 结束（带这次要登录的账号）。
    HostSave(Value),
    /// 这一步不认这个选择：不动。
    Ignore,
}

struct State {
    reading: reading::Reading,
    /// 插件编号（`--reserved` 时就是那个保留编号）。
    id: String,
    plain: bool,
    /// `--callback-path` 给的回调路径；不给就写相对路径 `callback`（v14：宿主拼在这个 IM 自己的目录下，插件不用知道平台编号）。
    callback_path: Option<String>,
    /// 用宿主账号登录怎么声明（`--host`、`--host-extra`）。
    host: HostMethod,
    /// `--bad-capture`：登录窗口一步故意声明不合格的开头（v36，自检工具与宿主应当拒绝）。
    bad_capture: bool,
    /// `--login-window-url <网址>`：登录窗口一步的起始网址换成它（看真实网页在窗口里的样子，v36）。
    window_url: Option<String>,
    logged_in: bool,
    /// 当前账号（`session/open`、续期时从凭据里取回；交接时换成内置平台核验过的账号）。
    account: Value,
    flows: BTreeMap<String, Flow>,
    next_flow: u64,
    messages: Vec<Value>,
    /// 消息上的表情回应（第一条上有一个目录里的“赞”、一个目录外的 `wave`；宿主点的加在后面）。
    reactions: Vec<Value>,
    favorites: Vec<String>,
    /// 每个会话上次被读的时间（标准错误里写读取间隔用）。
    last_read: BTreeMap<String, i64>,
    /// 刚开始的登录要推的一步（“打开登录页”或“登录窗口”，回复 `login/start` 之后推）：流程编号与这一步。
    pending_step: Option<(String, Value)>,
    /// 正在走的宿主账号登录或插件操作。
    active: Option<Active>,
    /// 回复宿主之后要做的事（读标准输入的那条线程在回复之后做）。
    follow: Vec<Follow>,
    /// 登录着的设备（“我的设备”；批准新设备后多一台）。
    devices: Vec<Device>,
    /// 群：会话编号 → 群主与成员。
    groups: BTreeMap<String, Group>,
    /// 好友（邀请成员时勾选的就是不在群里的这些人）。
    friends: Vec<String>,
    /// 恢复密钥是第几版（重新生成一次加一）。
    key_round: usize,
    /// 好友链接是第几条（换一条加一）。
    link_round: usize,
    /// 缺权限的提醒在不在（`notice` / `notice-clear`）。
    notice_scope: bool,
    /// 等批准的新设备请求：在就是输错了几次（`approve` 开关来、处理完或作废了去掉）。
    approve_wrong: Option<u32>,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let value_of = |flag: &str| {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let id = value_of("--reserved")
        .or_else(|| value_of("--id"))
        .unwrap_or_else(|| "sample".into());
    let has = |flag: &str| args.iter().any(|a| a == flag);
    let host = if has("--host-extra") {
        HostMethod::Extra
    } else if has("--host") {
        HostMethod::Only
    } else {
        HostMethod::Off
    };
    let reading = reading::Reading {
        shared: value_of("--reading-state").map(Into::into),
        ..Default::default()
    };
    let mut messages = demo_messages();
    reading
        .refresh_messages(&mut messages)
        .expect("示例共享消息无法读取");
    let state = Arc::new(Mutex::new(State {
        reading,
        favorites: Vec::new(),
        id,
        plain: has("--plain"),
        callback_path: value_of("--callback-path"),
        host,
        bad_capture: has("--bad-capture"),
        window_url: value_of("--login-window-url"),
        logged_in: false,
        account: default_account(),
        flows: BTreeMap::new(),
        next_flow: 1,
        messages,
        reactions: demo_reactions(),
        last_read: BTreeMap::new(),
        pending_step: None,
        active: None,
        follow: Vec::new(),
        devices: demo_devices(),
        groups: demo_groups(),
        friends: ["u-amy", "u-bo", "u-lin", "u-dave"]
            .map(str::to_owned)
            .to_vec(),
        key_round: 0,
        link_round: 0,
        notice_scope: false,
        approve_wrong: None,
    }));
    let out: Out = Arc::new(Mutex::new(std::io::stdout()));
    let next_id = Arc::new(AtomicI64::new(1_000_000));
    let waiting: Waiting = Arc::new(Mutex::new(BTreeMap::new()));
    let port = serve_auth_pages(state.clone(), out.clone(), next_id.clone(), waiting.clone());
    watch_switches(state.clone(), out.clone());

    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(msg) = codec::decode_line(line) else {
            eprintln!("sample_provider: 收到非法 JSON-RPC，跳过");
            continue;
        };
        if msg.is_response() {
            // 有人在等这个回复（续期等凭据存好）就交给它，别的回复不看。
            if let Some(RpcId::Number(n)) = &msg.id
                && let Some(tx) = waiting.lock().unwrap().remove(n)
            {
                let _ = tx.send(msg);
            }
            continue;
        }
        if msg.is_notification() {
            // 宿主→插件的通知不用回：取消、表单提交、在一步里做的选择（协议 §6.1、§6.7）。
            if let Some(method) = msg.method.as_deref() {
                let params = msg.params.clone().unwrap_or(Value::Null);
                on_notification(method, &params, &state, &out, &next_id, &waiting);
            }
            continue;
        }
        let (Some(method), Some(id)) = (msg.method.clone(), msg.id.clone()) else {
            continue;
        };
        let params = msg.params.clone().unwrap_or(Value::Null);
        if method == "session/renew" {
            // 续期：经 credential/save 存新的、宿主回了“存好了”再回续期成功（协议 §6.3）；在另一条线程里等，
            // 这条线程接着读标准输入（宿主的回复也从这里来）。
            eprintln!("sample_provider: 宿主叫续期");
            adopt_account(&state, params["credential"].as_str());
            let (state, out, next_id, waiting) =
                (state.clone(), out.clone(), next_id.clone(), waiting.clone());
            std::thread::spawn(move || {
                let reply = match save_credential_and_wait(&state, &out, &next_id, &waiting) {
                    Ok(()) => RpcMessage::ok(id, json!({})),
                    Err(message) => RpcMessage::err(id, error("unavailable", &message)),
                };
                send(&out, reply);
            });
            continue;
        }
        if method == "session/open" && params.get("legacy").is_some_and(Value::is_object) {
            // 交接：先经 credential/save 存好自己的凭据、宿主回了“存好了”再回在线（协议附录 D，同 §6.3 先存后用；
            // 没等回复就回在线时，宿主可能还没存下，内置那份就留到下次再交）。同续期在另一条线程里等。
            let (state, out, next_id, waiting) =
                (state.clone(), out.clone(), next_id.clone(), waiting.clone());
            std::thread::spawn(move || {
                let reply = match adopt_legacy(&params, &state, &out, &next_id, &waiting) {
                    Ok(value) => RpcMessage::ok(id, value),
                    Err(message) => RpcMessage::err(id, error("unavailable", &message)),
                };
                send(&out, reply);
            });
            continue;
        }
        let reply = handle(&method, &params, &state, &out, &next_id, &waiting, port);
        send(
            &out,
            match reply {
                Ok(value) => RpcMessage::ok(id, value),
                Err(error) => RpcMessage::err(id, error),
            },
        );
        let pending = state.lock().unwrap().pending_step.take();
        if let Some((flow_id, step)) = pending {
            notify(
                &out,
                "login/step",
                json!({ "flowId": flow_id, "step": step }),
            );
        }
        let follow = std::mem::take(&mut state.lock().unwrap().follow);
        for next in follow {
            match next {
                Follow::Notify(method, params) => notify(&out, &method, params),
                Follow::HostLogin {
                    flow_id,
                    account,
                    token_ok,
                } => host_login(flow_id, account, token_ok, &state, &out, &next_id, &waiting),
            }
        }
    }
}

/// 处理一个请求；错误按协议的错误对象回。
fn handle(
    method: &str,
    params: &Value,
    state: &Arc<Mutex<State>>,
    out: &Out,
    next_id: &Arc<AtomicI64>,
    waiting: &Waiting,
    port: u16,
) -> Result<Value, RpcError> {
    match method {
        "initialize" => {
            let mut state = state.lock().unwrap();
            state.reading.events = params["readingEvents"] == true;
            Ok(initialize(&state))
        }
        "reading/receipts" | "reading/people" | "reading/sync" | "reading/mark" => {
            reading::handle(method, params, &mut state.lock().unwrap(), out)
        }
        "session/open" => Ok(open(params, state, out, next_id)),
        "session/logout" => {
            state.lock().unwrap().logged_in = false;
            Ok(json!({}))
        }
        "ping" | "shutdown" => Ok(json!({})),
        "login/start" => start_login(params, state, port),
        "login/callback" => finish_callback(params, state, out, next_id, waiting),
        "actions/start" => start_action(params, state),
        "conversations/get" => conversation(params["conversationId"].as_str().unwrap_or(""))
            .ok_or_else(|| error("notFound", "没有这个会话。")),
        "conversations/recentActivity" => Ok(recent_activity(params, state)),
        "messages/list" => Ok(list_messages(params, state)),
        "messages/get" => {
            let id = params["messageId"].as_str().unwrap_or("");
            let state = state.lock().unwrap();
            state
                .messages
                .iter()
                .find(|m| m["id"] == id)
                .map(translated)
                .ok_or_else(|| error("notFound", "没有这条消息。"))
        }
        "people/get" => {
            let found: Vec<Value> = params["ids"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(|id| json!({ "id": id, "kind": "user", "name": person(id) }))
                .collect();
            Ok(json!({ "found": found, "missing": [] }))
        }
        "reactions/list" => {
            if std::path::Path::new("deny-reactions").exists() {
                return Err(error(
                    "permissionDenied",
                    "示例 IM 没给读表情的权限：在示例开放平台打开“读表情”后重新登录。",
                ));
            }
            let message = params["messageId"].as_str().unwrap_or("");
            let state = state.lock().unwrap();
            let items: Vec<Value> = state
                .reactions
                .iter()
                .filter(|r| r["messageId"] == message)
                .cloned()
                .collect();
            Ok(json!({ "items": items }))
        }
        "reactions/add" => {
            let mut state = state.lock().unwrap();
            let n = state.reactions.len() + 1;
            let reaction = json!({
                "id": format!("r-{n}"), "conversationId": params["conversationId"], "messageId": params["messageId"],
                "operator": { "id": "me", "kind": "user", "name": "示例用户" },
                "emojiType": params["emojiType"], "atMs": now_ms()
            });
            state.reactions.push(reaction.clone());
            Ok(reaction)
        }
        "reactions/remove" => {
            let id = params["reactionId"].as_str().unwrap_or("").to_owned();
            state
                .lock()
                .unwrap()
                .reactions
                .retain(|r| r["id"] != id.as_str());
            Ok(json!({}))
        }
        "stickers/packs" => {
            if std::path::Path::new("deny-stickers").exists() {
                return Err(error("unavailable", "示例表情包暂时不可用"));
            }
            Ok(
                json!({"items":[{"id":"sample-pack","name":"示例表情包","cover":"smile"}],"next":null}),
            )
        }
        "stickers/list" | "stickers/favorites" => {
            if std::path::Path::new("deny-stickers").exists() {
                return Err(error("unavailable", "示例贴纸暂时不可用"));
            }
            let state = state.lock().unwrap();
            let from = params["cursor"]
                .as_str()
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0);
            let limit = params["limit"].as_u64().unwrap_or(48).min(100) as usize;
            let all: Vec<_> = (0..60).filter(|i| method == "stickers/list" || state.favorites.contains(&format!("s-{i}"))).map(|i| json!({"id":format!("s-{i}"),"name":format!("贴纸 {}",i+1),"imageKey":if i%2==0 {"smile"} else {"thumbsup"},"favorite":state.favorites.contains(&format!("s-{i}"))})).collect();
            let items: Vec<_> = all.iter().skip(from).take(limit).cloned().collect();
            let next = (from + items.len() < all.len()).then(|| (from + items.len()).to_string());
            Ok(json!({"items":items,"next":next}))
        }
        "stickers/favorite" => {
            if std::path::Path::new("deny-favorite").exists() {
                return Err(error("permissionDenied", "示例收藏操作被拒绝"));
            }
            let id = params["id"].as_str().unwrap_or("").to_owned();
            let mut state = state.lock().unwrap();
            state.favorites.retain(|v| v != &id);
            if params["favorite"].as_bool() == Some(true) {
                state.favorites.push(id);
            }
            Ok(json!({}))
        }
        "stickers/image" => Ok(chunk(
            params,
            if params["key"] == "smile" {
                SMILE_PNG
            } else {
                THUMBSUP_PNG
            },
        )),
        "emoji/catalog" => Ok(json!({
            "emojis": [ { "emojiType": "thumbsup", "name": "赞" }, { "emojiType": "smile", "name": "微笑" },
                { "emojiType": "thumbsup-sun", "name": "赞（晒黑）", "toneOf": { "base": "thumbsup", "tone": "sun" } } ],
            "tones": [{ "id": "sun", "name": "晒黑" }],
            "categories": [], "updatedAtMs": 2
        })),
        "emoji/image" => match params["emojiType"].as_str() {
            Some("thumbsup" | "thumbsup-sun") => Ok(chunk(params, THUMBSUP_PNG)),
            Some("smile") => Ok(chunk(params, SMILE_PNG)),
            Some("wave") => Ok(chunk(params, WAVE_PNG)),
            _ => Err(error("notFound", "示例 IM 没有这个表情的图。")),
        },
        "resources/read" if params["key"] == "link-cover-missing" => {
            Err(error("notFound", "示例封面已无法读取"))
        }
        "resources/read" => Ok(chunk(params, SMILE_PNG)),
        "messages/send" => {
            if std::path::Path::new("deny-send").exists() {
                return Err(error("permissionDenied", "示例：发送被拒绝"));
            }
            if std::path::Path::new("unknown-send").exists() {
                return Ok(json!({"state":"unknown","reason":"示例：发送结果尚未确认"}));
            }
            Ok(send_message(params, state))
        }
        "messages/edit" => edit_message(params, state),
        "messages/retranslate" => Ok(retranslate(params, state)),
        _ => Err(error("unsupported", "示例 IM 不支持这件事。")),
    }
}

fn initialize(state: &State) -> Value {
    let mut rules = vec![json!({ "name": "sample-v1" })];
    if std::path::Path::new("rule-v2").exists() {
        rules.push(json!({ "name": "sample-v2", "formats": ["sample-msg-v1"] }));
    }
    let mut platform = json!({
        "avatarShape": "rounded",
        "history": "server",
        "actions": { "reply": true, "thread": true, "react": true, "recall": null, "edit": { "own": true, "perMessage": true } },
        "pageMax": 50,
        "reads": { "perSecond": 5, "perMinute": 120 },
        "emoji": { "unlisted": { "placeholderLabel": "表情", "guess": true }, "textPattern": "\\{([^{}\\r\\n]{1,12})\\}", "whiteBackground": true, "textTemplate": "{{type}}" },
        "discovery": { "fastestSecs": 15, "pages": 2, "coldPages": 4 },
        "editsByContent": ["text"],
        "retranslateRules": rules,
    });
    if !state.plain {
        platform["userAdjustable"] = json!(["readIntervals", "readsPerMinute"]);
    }
    let settings = if state.plain {
        json!([])
    } else {
        json!([{ "id": "signature", "label": "消息签名", "type": "text", "default": "",
                 "help": "发消息时附在末尾；留空不加。" }])
    };
    let callback = state
        .callback_path
        .clone()
        .unwrap_or_else(|| "callback".to_owned());
    // 用宿主账号登录（v13）：`--host` 只声明它，`--host-extra` 排在原来两种前面，不带参数照旧。
    let mut login_methods = Vec::new();
    if state.host != HostMethod::Off {
        login_methods.push(
            json!({ "id": HOST_METHOD, "kind": "host", "label": "用宿主账号登录",
                                   "note": "用 Friend 现在登录的账号登录示例 IM，不用注册。" }),
        );
    }
    if state.host != HostMethod::Only {
        login_methods.push(json!({ "id": "relay", "kind": "relay", "label": "通过中继登录",
                                   "note": "经公网授权中继登录示例 IM，不用自己创建应用。", "placeholder": "https://relay.example.com" }));
        login_methods.push(json!({ "id": "own", "kind": "ownApp", "label": "使用自己的应用", "note": "用你在示例开放平台创建的应用登录。",
              "idPlaceholder": "cli_ 开头", "secretPlaceholder": "应用的 App Secret",
              "secretHelp": "示例开放平台 → 应用的凭证页。只保存在本机。", "callbackPath": callback,
              "callbackHelp": "在示例开放平台的应用后台，把这个地址登记为重定向地址。",
              "createLink": { "label": "前往示例开放平台创建", "url": "https://open.example.com/app",
                              "after": "，创建后把 App ID 与 App Secret 填在下面。" } }));
        // 登录窗口（v36）：直接登录，点“登录”后推“登录窗口”一步。
        login_methods.push(
            json!({ "id": WINDOW_METHOD, "kind": "direct", "label": "在登录窗口里登录",
                                   "note": "在 HHH 打开的登录窗口里完成登录，不用自己创建应用。" }),
        );
    }
    let mut result = json!({
        "protocolVersion": 1,
        "provider": { "id": state.id, "name": "示例 IM", "version": "0.1.0" },
        "platform": platform,
        "capabilities": {
            "stickers": {"packs": true, "favorites": true, "favoriteAdd": true, "favoriteRemove": true, "send": true},
            "conversations": { "list": false, "search": false, "members": false, "findDirect": false, "recentActivity": true },
            "messages": { "list": true, "get": true },
            "people": { "get": true, "search": false },
            "send": { "text": true, "mentions": false, "mentionAll": false, "image": false, "file": false, "lookup": false },
            "recall": false,
            "edit": true,
            "reactions": { "list": true, "add": true, "remove": true, "catalog": true },
            "resources": { "read": true, "avatars": false },
            "events": { "messages": true, "deletions": false, "reactions": false, "conversations": true, "people": true },
            "retranslate": true,
            "session": { "renew": true }
        },
        "settings": settings,
        "loginMethods": login_methods
    });
    reading::declare(&mut result, state.plain);
    // 用于核对旧插件未声明贴纸能力时，宿主仍只显示小表情。
    if std::path::Path::new("no-stickers").exists() {
        result["capabilities"]
            .as_object_mut()
            .unwrap()
            .remove("stickers");
    }
    // 插件操作（v13）：账号级的进管理菜单，会话级的进会话菜单——群成员、邀请、退出只给群，删除会话（v19）不写种类、
    // 私聊与群都有。`--plain` 不声明。
    if !state.plain {
        result["actions"] = json!([
            { "id": "devices", "label": "我的设备", "scope": "account" },
            { "id": "link", "label": "我的链接", "scope": "account" },
            { "id": "add", "label": "加个朋友", "scope": "account" },
            { "id": "about", "label": "关于示例", "scope": "account" },
            { "id": "members", "label": "群成员", "scope": "conversation", "conversationKinds": ["group"] },
            { "id": "invite", "label": "邀请成员", "scope": "conversation", "conversationKinds": ["group"] },
            { "id": "leave", "label": "退出群", "scope": "conversation", "conversationKinds": ["group"] },
            { "id": DELETE_ACTION, "label": "删除会话", "scope": "conversation" }
        ]);
    }
    result
}

/// 交接（`session/open` 带 `legacy`）：沿用内置授权里上次核验的账号，先存自己的凭据并等宿主回复，再回在线与要写进
/// 设置的东西；宿主没存上时回错误（内置那份留着，下次再交）。
fn adopt_legacy(
    params: &Value,
    state: &Arc<Mutex<State>>,
    out: &Out,
    next_id: &Arc<AtomicI64>,
    waiting: &Waiting,
) -> Result<Value, String> {
    let legacy = &params["legacy"];
    eprintln!(
        "sample_provider: 收到内置平台的交接（格式 {}）",
        legacy["format"].as_str().unwrap_or("?")
    );
    // 内置授权里有上次核验的账号：沿用它（同编号按账号分库，账号对上本机记录与关注才在）。
    let builtin: Value = legacy["credential"]
        .as_str()
        .and_then(|c| serde_json::from_str(c).ok())
        .unwrap_or(Value::Null);
    if let Some(id) = builtin["account"]["id"]
        .as_str()
        .filter(|id| !id.is_empty())
    {
        let name = builtin["account"]["name"].as_str().unwrap_or(id);
        state.lock().unwrap().account = json!({ "id": id, "name": name });
    }
    save_credential_and_wait(state, out, next_id, waiting)?;
    let account = state.lock().unwrap().account.clone();
    state.lock().unwrap().logged_in = true;
    push_conversations(out, next_id);
    let app_id = legacy["settings"]["appId"]
        .as_str()
        .unwrap_or("")
        .to_owned();
    let mut adopted = json!({ "parameters": { "signature": "" } });
    if !app_id.is_empty() {
        adopted["login"] = json!({ "method": "own", "values": { "appId": app_id } });
    }
    Ok(json!({ "state": "online", "account": account, "adoptedSettings": adopted }))
}

/// `session/open`：有凭据在线；都没有要登录（带 `legacy` 的交接见 [`adopt_legacy`]）。
fn open(params: &Value, state: &Arc<Mutex<State>>, out: &Out, next_id: &Arc<AtomicI64>) -> Value {
    let credential = params
        .get("credential")
        .and_then(Value::as_str)
        .filter(|c| !c.is_empty());
    adopt_account(state, credential);
    if credential.is_some() || state.lock().unwrap().logged_in {
        state.lock().unwrap().logged_in = true;
        push_conversations(out, next_id);
        json!({ "state": "online", "account": state.lock().unwrap().account.clone() })
    } else {
        json!({ "state": "needsLogin", "reason": "noCredential", "message": "需要登录示例 IM：点“登录”。" })
    }
}

/// `login/start`：记下这次的 state 与回调地址，回流程编号；“打开登录页”一步在回复之后推。选了用宿主账号登录的见
/// [`start_host_login`]。新的登录取消还没结束的旧流程。
fn start_login(params: &Value, state: &Arc<Mutex<State>>, port: u16) -> Result<Value, RpcError> {
    let method = params["method"].as_str().unwrap_or("relay");
    if method == HOST_METHOD {
        return Ok(start_host_login(params, state));
    }
    if method == "own" && params["values"]["appId"].as_str().unwrap_or("").trim() == "bad" {
        return Err(error(
            "invalidRequest",
            "示例开放平台说这个 App ID 不存在。",
        ));
    }
    let mut state = state.lock().unwrap();
    cancel_running(&mut state);
    let flow_id = new_flow_id(&mut state);
    // 旧宿主不给 state：插件自己编一个。
    let key = params["state"]
        .as_str()
        .map_or_else(|| format!("sample-{flow_id}"), str::to_owned);
    let callback = (method == "own")
        .then(|| params["callbackUrl"].as_str().map(str::to_owned))
        .flatten();
    let window = method == WINDOW_METHOD;
    let step = if window {
        window_step(&key, port, state.bad_capture, state.window_url.clone())
    } else {
        json!({ "kind": "openUrl", "label": "再打开一次",
            "url": format!("http://127.0.0.1:{port}/authorize?state={}", encode(&key)),
            "note": { "text": "演示插件的假授权页：点“同意”就算登录成功。", "tone": "info" } })
    };
    state.pending_step = Some((flow_id.clone(), step));
    state.flows.insert(
        key,
        Flow {
            flow_id: flow_id.clone(),
            callback,
            window,
        },
    );
    Ok(json!({ "flowId": flow_id }))
}

/// 登录窗口一步（契约 provider-protocol v36）：起始网址是自带的假登录页（参数 `--login-window-url` 或工作目录里 `login-window-url` 写了网址就用它，
/// 验收看真实网页在窗口里的样子用）；要截的开头是 `hhh-sample://`（有 `login-window-bad-capture` 时故意写成不合格的
/// `https://example.com/`，演示宿主与自检工具拒绝；参数 `--bad-capture` 同样）。
fn window_step(key: &str, port: u16, bad_capture: bool, given: Option<String>) -> Value {
    let url = given
        .or_else(|| std::fs::read_to_string("login-window-url").ok())
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| format!("http://127.0.0.1:{port}/window?state={}", encode(key)));
    let capture = if bad_capture || std::path::Path::new("login-window-bad-capture").exists() {
        "https://example.com/"
    } else {
        WINDOW_CAPTURE
    };
    json!({ "kind": "loginWindow", "url": url, "label": "打开登录窗口", "capture": [capture],
        "note": { "text": "演示插件的假登录页：点“完成登录”就算登录成功。", "tone": "info" } })
}

/// `login/callback`：宿主把回调地址交回来（自己的应用方式的本机回调，或 v36 登录窗口截下的 `hhh-sample://` 链接）；
/// `state` 与授权码对得上才算登录成功。登录窗口交回来的链接对不上这次登录或没有登录码时，这次登录失败并说原因。
fn finish_callback(
    params: &Value,
    state: &Arc<Mutex<State>>,
    out: &Out,
    next_id: &Arc<AtomicI64>,
    waiting: &Waiting,
) -> Result<Value, RpcError> {
    let url = params["url"].as_str().unwrap_or("");
    let flow_id = params["flowId"].as_str().unwrap_or("");
    let query = query_of(url);
    let given = query.get("state").cloned().unwrap_or_default();
    let (flow, matched) = {
        let mut state = state.lock().unwrap();
        match state.flows.get(&given) {
            Some(flow) if flow.flow_id == flow_id => (state.flows.remove(&given), true),
            _ => {
                // 登录窗口截下的链接带错了 state：按流程编号找到这次登录、结束它。
                let key = state
                    .flows
                    .iter()
                    .find(|(_, flow)| flow.window && flow.flow_id == flow_id)
                    .map(|(key, _)| key.clone());
                (key.and_then(|key| state.flows.remove(&key)), false)
            }
        }
    };
    let Some(flow) = flow else {
        return Err(error("invalidRequest", "回调对不上这次登录。"));
    };
    let problem = if !matched {
        Some("登录窗口交回来的链接对不上这次登录。")
    } else if query.get("code").map(String::as_str) != Some(CODE) {
        Some(if flow.window {
            "登录窗口交回来的链接里没有登录码。"
        } else {
            "示例开放平台没给授权码。"
        })
    } else {
        None
    };
    if let Some(message) = problem {
        notify(
            out,
            "login/failed",
            json!({ "flowId": flow.flow_id, "message": message, "retryable": true }),
        );
        return Ok(json!({}));
    }
    complete_login(&flow.flow_id, state, out, next_id, waiting);
    Ok(json!({}))
}

/// 登录成功：先存凭据（带续期时间）、等宿主回了“存好了”再说登录完成、推会话（协议 §6.3：`login/finished` 只能在
/// `credential/save` 成功之后发）。在另一条线程里等，调用方（读标准输入的那条、假授权页）照常往下走；宿主没存上时说登录失败。
fn complete_login(
    flow_id: &str,
    state: &Arc<Mutex<State>>,
    out: &Out,
    next_id: &Arc<AtomicI64>,
    waiting: &Waiting,
) {
    let (flow_id, state, out, next_id, waiting) = (
        flow_id.to_owned(),
        state.clone(),
        out.clone(),
        next_id.clone(),
        waiting.clone(),
    );
    std::thread::spawn(move || {
        if let Err(message) = save_credential_and_wait(&state, &out, &next_id, &waiting) {
            notify(
                &out,
                "login/failed",
                json!({ "flowId": flow_id, "message": message, "retryable": true }),
            );
            return;
        }
        let account = {
            let mut state = state.lock().unwrap();
            state.logged_in = true;
            state.account.clone()
        };
        notify(
            &out,
            "login/finished",
            json!({ "flowId": flow_id, "account": account }),
        );
        push_conversations(&out, &next_id);
    });
}

// ============================ v13：用宿主账号登录、插件操作、请你处理的事 ============================

fn new_flow_id(state: &mut State) -> String {
    let flow_id = format!("flow-{}", state.next_flow);
    state.next_flow += 1;
    flow_id
}

/// 新的 `login/start`、`actions/start` 取消还没结束的旧流程（协议 §6.1、§6.7：同一时间只走一个）。
fn cancel_running(state: &mut State) {
    if let Some(old) = state.active.take() {
        eprintln!(
            "sample_provider: 新流程开始，取消还没结束的 {}",
            old.flow_id
        );
    }
    state.flows.clear();
    state.pending_step = None;
}

fn is_active(state: &Arc<Mutex<State>>, flow_id: &str) -> bool {
    state
        .lock()
        .unwrap()
        .active
        .as_ref()
        .is_some_and(|a| a.flow_id == flow_id)
}

/// 这个流程结束了（还是它在走时）。
fn end_flow(state: &Arc<Mutex<State>>, flow_id: &str) {
    state
        .lock()
        .unwrap()
        .active
        .take_if(|a| a.flow_id == flow_id);
}

/// `login/start` 选了用宿主账号登录：回流程编号，开通在回复之后另起（[`host_login`]）。令牌不打印、不存，只写长度与账号名。
fn start_host_login(params: &Value, state: &Arc<Mutex<State>>) -> Value {
    let token = params["values"]["hostToken"].as_str().unwrap_or("");
    let name = params["hostAccount"]["name"]
        .as_str()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .unwrap_or("示例用户");
    eprintln!(
        "sample_provider: 用宿主账号登录（令牌 {} 字节，宿主账号「{name}」）",
        token.len()
    );
    let mut state = state.lock().unwrap();
    cancel_running(&mut state);
    let flow_id = new_flow_id(&mut state);
    state.active = Some(Active {
        flow_id: flow_id.clone(),
        kind: "login",
        stage: Stage::HostOpening,
    });
    state.follow.push(Follow::HostLogin {
        flow_id: flow_id.clone(),
        account: json!({ "id": ME, "name": name }),
        token_ok: !token.trim().is_empty(),
    });
    json!({ "flowId": flow_id })
}

/// 用宿主账号开通（`login/start` 的回复发出之后）：没令牌直接失败；有就先推“正在开通…”，再按开关文件 `host-fail` 失败、
/// `host-key` 出示恢复密钥，都没有就存凭据后静默结束。在另一条线程里做（存凭据要等宿主回复，回复从标准输入来）。
fn host_login(
    flow_id: String,
    account: Value,
    token_ok: bool,
    state: &Arc<Mutex<State>>,
    out: &Out,
    next_id: &Arc<AtomicI64>,
    waiting: &Waiting,
) {
    if !token_ok {
        eprintln!("sample_provider: 宿主没给令牌，用宿主账号登录失败");
        end_flow(state, &flow_id);
        notify(
            out,
            "login/failed",
            json!({ "flowId": flow_id, "message": "没收到宿主的令牌", "retryable": false }),
        );
        return;
    }
    let (state, out, next_id, waiting) =
        (state.clone(), out.clone(), next_id.clone(), waiting.clone());
    std::thread::spawn(move || {
        notify(
            &out,
            "login/step",
            json!({ "flowId": flow_id, "step": { "kind": "message", "text": "正在开通…", "tone": "info" } }),
        );
        // 停一会儿：手动点“登录”时看得见“正在开通…”。
        std::thread::sleep(std::time::Duration::from_millis(800));
        if !is_active(&state, &flow_id) {
            eprintln!("sample_provider: 开通途中流程 {flow_id} 被取消了");
            return;
        }
        if std::path::Path::new("host-fail").exists() {
            eprintln!("sample_provider: 开关 host-fail，开通失败");
            end_flow(&state, &flow_id);
            notify(
                &out,
                "login/failed",
                json!({ "flowId": flow_id, "message": "示例服务器暂时连不上。", "retryable": true,
                        "title": "没开通成", "retryLabel": "再试一次" }),
            );
        } else if std::path::Path::new("host-key").exists() {
            eprintln!("sample_provider: 开关 host-key，出示恢复密钥");
            let step = {
                let mut st = state.lock().unwrap();
                let step = key_step(
                    st.key_round,
                    "示例 IM 已开通。这是你的恢复密钥：所有设备都丢了时，只有它能找回聊天记录。它只显示这一次，请抄在纸上，或存进密码管理器。",
                );
                match st.active.as_mut().filter(|a| a.flow_id == flow_id) {
                    Some(active) => {
                        active.stage = Stage::HostKey(account);
                        Some(step)
                    }
                    None => None,
                }
            };
            if let Some(step) = step {
                notify(
                    &out,
                    "login/step",
                    json!({ "flowId": flow_id, "step": step }),
                );
            }
        } else {
            finish_host_login(
                &flow_id,
                account,
                json!({}),
                &state,
                &out,
                &next_id,
                &waiting,
            );
        }
    });
}

/// 宿主账号登录做完：换成这个账号、存凭据并等宿主回了“存好了”，再 `login/finished`（`extra` 是成功页的标题与一段话；
/// 都不给时自动登录静默结束）、推会话。要在读标准输入以外的线程里叫。没存上时换回原来的账号、说登录失败。
fn finish_host_login(
    flow_id: &str,
    account: Value,
    extra: Value,
    state: &Arc<Mutex<State>>,
    out: &Out,
    next_id: &Arc<AtomicI64>,
    waiting: &Waiting,
) {
    let previous = std::mem::replace(&mut state.lock().unwrap().account, account);
    if let Err(message) = save_credential_and_wait(state, out, next_id, waiting) {
        eprintln!("sample_provider: 用宿主账号登录时没存上凭据：{message}");
        state.lock().unwrap().account = previous;
        end_flow(state, flow_id);
        notify(
            out,
            "login/failed",
            json!({ "flowId": flow_id, "message": message, "retryable": true }),
        );
        return;
    }
    let account = {
        let mut st = state.lock().unwrap();
        st.logged_in = true;
        st.active.take_if(|a| a.flow_id == flow_id);
        st.account.clone()
    };
    let mut params = extra;
    params["flowId"] = json!(flow_id);
    params["account"] = account;
    notify(out, "login/finished", params);
    push_conversations(out, next_id);
}

/// `actions/start`：认出是哪项操作，回流程编号（提醒上的“查看”另给对话框标题）；第一步在回复之后推。会话级操作要给一个
/// 示例 IM 里有的群。
fn start_action(params: &Value, state: &Arc<Mutex<State>>) -> Result<Value, RpcError> {
    let action = params["actionId"].as_str().unwrap_or("");
    let conversation = params["conversationId"].as_str().unwrap_or("").to_owned();
    let mut st = state.lock().unwrap();
    let group = st.groups.get(&conversation);
    let first = match action {
        "devices" => Next::Step(Stage::Devices, devices_step(&st, None)),
        "link" => Next::Step(Stage::Link, link_step(&st, None)),
        "add" => Next::Step(Stage::AddInput, add_step(None)),
        "about" => Next::Step(Stage::About, about_step()),
        "members" | "invite" | "leave" if group.is_none() => {
            eprintln!("sample_provider: {action} 给的会话 {conversation} 不是示例 IM 的群");
            return Err(error("notFound", "示例 IM 里没有这个群。"));
        }
        "members" => Next::Step(
            Stage::Members(conversation.clone()),
            members_step(&st, &conversation, None),
        ),
        "invite" => Next::Step(
            Stage::Invite(conversation.clone()),
            invite_step(&st, &conversation, None),
        ),
        "leave" if group.is_some_and(|g| g.members.iter().any(|m| m == ME)) => Next::Step(
            Stage::Leave(conversation.clone()),
            leave_step(&conversation),
        ),
        "leave" => Next::Failed(json!({ "title": "不在这个群里",
            "message": format!("你已经退出「{}」了。", group_name(&conversation)), "retryable": false })),
        DELETE_ACTION if self::conversation(&conversation).is_none() => {
            eprintln!("sample_provider: {action} 给的会话 {conversation} 不是示例 IM 的会话");
            return Err(error("notFound", "示例 IM 里没有这个会话。"));
        }
        DELETE_ACTION => Next::Step(
            Stage::Delete(conversation.clone()),
            delete_step(&conversation),
        ),
        APPROVE_ACTION => match st.approve_wrong {
            Some(wrong) => Next::Step(Stage::Approve, approve_step(wrong)),
            None => Next::Failed(json!({ "title": "请求已失效",
                "message": "这个请求已经处理过或作废了。", "retryable": false })),
        },
        _ => {
            eprintln!("sample_provider: 没有操作 {action}");
            return Err(error("notFound", "示例 IM 没有这个操作。"));
        }
    };
    cancel_running(&mut st);
    let flow_id = new_flow_id(&mut st);
    eprintln!("sample_provider: 开始操作 {action}（{flow_id}）");
    match first {
        Next::Step(stage, step) => {
            st.active = Some(Active {
                flow_id: flow_id.clone(),
                kind: "action",
                stage,
            });
            st.follow.push(Follow::Notify(
                "action/step".into(),
                json!({ "flowId": flow_id, "step": step }),
            ));
        }
        Next::Failed(mut failed) => {
            failed["flowId"] = json!(flow_id);
            st.follow
                .push(Follow::Notify("action/failed".into(), failed));
        }
        Next::Finished(_) | Next::HostSave(_) | Next::Ignore => {}
    }
    let mut result = json!({ "flowId": flow_id });
    if action == APPROVE_ACTION {
        result["title"] = json!("批准新设备");
    }
    Ok(result)
}

/// 宿主发来的通知（不用回）：取消、表单提交、在一步里做的选择。对不上正在走的流程的不理。
fn on_notification(
    method: &str,
    params: &Value,
    state: &Arc<Mutex<State>>,
    out: &Out,
    next_id: &Arc<AtomicI64>,
    waiting: &Waiting,
) {
    let flow_id = params["flowId"].as_str().unwrap_or("");
    match method {
        "login/cancel" | "action/cancel" => {
            let mut st = state.lock().unwrap();
            st.flows.retain(|_, flow| flow.flow_id != flow_id);
            if st.active.take_if(|a| a.flow_id == flow_id).is_some() {
                eprintln!("sample_provider: {method}，流程 {flow_id} 结束");
            }
        }
        "step/choose" => {
            let input = Input::Choose {
                choice: params["choice"].as_str().unwrap_or(""),
                row: params["row"].as_str().unwrap_or(""),
                selected: params["selected"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .collect(),
            };
            on_flow_input(flow_id, input, state, out, next_id, waiting);
        }
        "login/submit" | "action/submit" => {
            on_flow_input(
                flow_id,
                Input::Submit(&params["values"]),
                state,
                out,
                next_id,
                waiting,
            );
        }
        _ => {}
    }
}

/// 用户在正在走的流程里做了一件事：按停在哪一步往下走，推下一步或结束；等批准的请求处理完、作废了时重发提醒（去掉那一条）。
fn on_flow_input(
    flow_id: &str,
    input: Input<'_>,
    state: &Arc<Mutex<State>>,
    out: &Out,
    next_id: &Arc<AtomicI64>,
    waiting: &Waiting,
) {
    let (kind, next, notices_now) = {
        let mut st = state.lock().unwrap();
        let Some(active) = st.active.clone().filter(|a| a.flow_id == flow_id) else {
            eprintln!("sample_provider: 收到的选择或提交对不上正在走的流程（{flow_id}），不理");
            return;
        };
        let approving = st.approve_wrong.is_some();
        let next = advance(&mut st, active.stage, input);
        match &next {
            Next::Step(stage, _) => {
                if let Some(a) = st.active.as_mut() {
                    a.stage = stage.clone();
                }
            }
            Next::HostSave(_) => {
                if let Some(a) = st.active.as_mut() {
                    a.stage = Stage::HostSaving;
                }
            }
            Next::Finished(_) | Next::Failed(_) => st.active = None,
            Next::Ignore => {}
        }
        let notices_now = (approving && st.approve_wrong.is_none()).then(|| notices(&st));
        (active.kind, next, notices_now)
    };
    match next {
        Next::Step(_, step) => notify(
            out,
            &format!("{kind}/step"),
            json!({ "flowId": flow_id, "step": step }),
        ),
        Next::Finished(mut done) => {
            done["flowId"] = json!(flow_id);
            notify(out, &format!("{kind}/finished"), done);
        }
        Next::Failed(mut failed) => {
            failed["flowId"] = json!(flow_id);
            notify(out, &format!("{kind}/failed"), failed);
        }
        Next::HostSave(account) => {
            let (flow_id, state, out, next_id, waiting) = (
                flow_id.to_owned(),
                state.clone(),
                out.clone(),
                next_id.clone(),
                waiting.clone(),
            );
            std::thread::spawn(move || {
                let done = json!({ "title": "示例 IM 已开通",
                    "message": "以后在别的电脑上用同一个账号登录，就能看到全部记录。" });
                finish_host_login(&flow_id, account, done, &state, &out, &next_id, &waiting);
            });
        }
        Next::Ignore => eprintln!("sample_provider: 这一步不认这个选择或提交（{flow_id}），不动"),
    }
    if let Some(params) = notices_now {
        notify(out, "session/notice", params);
    }
}

/// 按停在哪一步与用户做的事决定下一步（改动设备、群、好友、提醒都在这里）。
fn advance(st: &mut State, stage: Stage, input: Input<'_>) -> Next {
    use Input::{Choose, Submit};
    match (stage, input) {
        (
            Stage::HostKey(account),
            Choose {
                choice: "confirm", ..
            },
        ) => Next::HostSave(account),
        // 我的设备。
        (
            Stage::Devices,
            Choose {
                choice: "remove",
                row,
                ..
            },
        ) => match st.devices.iter().find(|d| d.id == row && !d.this) {
            Some(device) => Next::Step(
                Stage::DeviceRemove(device.id.clone()),
                json!({ "kind": "confirm", "caption": format!("移除「{}」？", device.name),
                        "text": "那台设备会立刻退出示例 IM，之后的新消息它读不到。它本机已有的聊天记录不会被远程删除。",
                        "confirmLabel": "移除", "danger": true, "cancelChoice": "back" }),
            ),
            None => Next::Ignore,
        },
        (
            Stage::Devices,
            Choose {
                choice: "regen", ..
            },
        ) => Next::Step(
            Stage::RegenConfirm,
            json!({ "kind": "confirm", "caption": "重新生成恢复密钥？",
                "text": "旧的恢复密钥会立即失效。已经登录着的设备不受影响。",
                "confirmLabel": "重新生成", "cancelChoice": "back" }),
        ),
        (
            Stage::DeviceRemove(id),
            Choose {
                choice: "confirm", ..
            },
        ) => {
            let name = st
                .devices
                .iter()
                .find(|d| d.id == id)
                .map(|d| d.name.clone())
                .unwrap_or_default();
            st.devices.retain(|d| d.id != id);
            let note = format!("已移除「{name}」。你的其它设备也会收到提醒。");
            Next::Step(Stage::Devices, devices_step(st, Some(&note)))
        }
        (Stage::DeviceRemove(_) | Stage::RegenConfirm, Choose { choice: "back", .. }) => {
            Next::Step(Stage::Devices, devices_step(st, None))
        }
        (
            Stage::RegenConfirm,
            Choose {
                choice: "confirm", ..
            },
        ) => {
            st.key_round += 1;
            Next::Step(
                Stage::RegenKey,
                key_step(
                    st.key_round,
                    "这是新的恢复密钥，只显示这一次。旧的已经失效。",
                ),
            )
        }
        (
            Stage::RegenKey,
            Choose {
                choice: "confirm", ..
            },
        ) => Next::Step(
            Stage::Devices,
            devices_step(st, Some("新的恢复密钥已生效，旧的已经失效。")),
        ),
        // 我的链接。
        (
            Stage::Link,
            Choose {
                choice: "renew", ..
            },
        ) => Next::Step(
            Stage::LinkRenew,
            json!({ "kind": "confirm", "caption": "换一条链接？",
                "text": "旧链接会立即失效，还没用它加你的人就加不了了；已经加了你的好友不受影响。",
                "confirmLabel": "换一条", "cancelChoice": "back" }),
        ),
        (
            Stage::LinkRenew,
            Choose {
                choice: "confirm", ..
            },
        ) => {
            st.link_round += 1;
            Next::Step(
                Stage::Link,
                link_step(
                    st,
                    Some("已换成新链接，旧链接失效了。已经加了你的人不受影响。"),
                ),
            )
        }
        (Stage::LinkRenew, Choose { choice: "back", .. }) => {
            Next::Step(Stage::Link, link_step(st, None))
        }
        // 加个朋友。
        (Stage::AddInput, Submit(values)) => {
            if values["link"]
                .as_str()
                .unwrap_or("")
                .trim()
                .starts_with("https://")
            {
                Next::Step(
                    Stage::AddConfirm,
                    json!({ "kind": "confirm", "person": { "name": "王小明", "detail": "身份指纹 3F2A 9C41" },
                        "caption": "加王小明为好友？对方不需要再点同意。", "confirmLabel": "加好友", "cancelChoice": "back" }),
                )
            } else {
                Next::Step(
                    Stage::AddInput,
                    add_step(Some(
                        "这不是好友链接。好友链接以 https:// 开头，由对方在“我的链接”里复制给你。",
                    )),
                )
            }
        }
        (Stage::AddConfirm, Choose { choice: "back", .. }) => {
            Next::Step(Stage::AddInput, add_step(None))
        }
        (
            Stage::AddConfirm,
            Choose {
                choice: "confirm", ..
            },
        ) => {
            if !st.friends.iter().any(|f| f == "u-xm") {
                st.friends.push("u-xm".into());
            }
            Next::Finished(json!({ "title": "已添加王小明",
                "message": "王小明已在你的好友里，“邀请成员”时能勾选他。演示插件不会新建私聊。" }))
        }
        (
            Stage::About,
            Choose {
                choice: "confirm", ..
            },
        ) => Next::Finished(json!({})),
        // 群成员：只有群主能移出别人。
        (
            Stage::Members(c),
            Choose {
                choice: "remove",
                row,
                ..
            },
        ) => {
            let allowed = st
                .groups
                .get(&c)
                .is_some_and(|g| g.owner == ME && row != ME && g.members.iter().any(|m| m == row));
            if allowed {
                let (who, group) = (person(row), group_name(&c));
                Next::Step(
                    Stage::MemberRemove(c, row.to_owned()),
                    json!({ "kind": "confirm", "caption": format!("把「{who}」移出「{group}」？"),
                        "text": format!("{who}读不到之后的新消息，在群期间的消息还留在他的设备上。"),
                        "confirmLabel": "移出", "danger": true, "cancelChoice": "back" }),
                )
            } else {
                Next::Ignore
            }
        }
        (
            Stage::MemberRemove(c, who),
            Choose {
                choice: "confirm", ..
            },
        ) => {
            if let Some(group) = st.groups.get_mut(&c) {
                group.members.retain(|m| *m != who);
            }
            let note = format!("已把「{}」移出群。", person(&who));
            let step = members_step(st, &c, Some(&note));
            Next::Step(Stage::Members(c), step)
        }
        (Stage::MemberRemove(c, _), Choose { choice: "back", .. }) => {
            let step = members_step(st, &c, None);
            Next::Step(Stage::Members(c), step)
        }
        // 邀请成员。
        (
            Stage::Invite(c),
            Choose {
                choice: "groupLink",
                ..
            },
        ) => {
            let step = json!({ "kind": "showText", "layout": "line", "copy": true,
                "text": format!("https://im.example.com/g/{c}-5TQ8"),
                "caption": "在示例 IM 里粘贴这条链接就能加入。链接带着群名，只有拿到链接的人看得到。",
                "links": [{ "id": "back", "label": "改回勾选好友" }] });
            Next::Step(Stage::InviteLink(c), step)
        }
        (Stage::InviteLink(c), Choose { choice: "back", .. }) => {
            let step = invite_step(st, &c, None);
            Next::Step(Stage::Invite(c), step)
        }
        (
            Stage::Invite(c),
            Choose {
                choice: "confirm",
                selected,
                ..
            },
        ) => {
            let Some(group) = st.groups.get(&c) else {
                return Next::Ignore;
            };
            let picked: Vec<String> = selected
                .into_iter()
                .filter(|id| st.friends.iter().any(|f| f == id))
                .filter(|id| !group.members.iter().any(|m| m == id))
                .map(str::to_owned)
                .collect();
            if picked.is_empty() {
                let step = invite_step(st, &c, Some("先勾选要邀请的好友。"));
                return Next::Step(Stage::Invite(c), step);
            }
            let names: Vec<&str> = picked.iter().map(|id| person(id)).collect();
            let message = format!(
                "已邀请{}加入「{}」。群里所有人都会看到这条提示。",
                names.join("、"),
                group_name(&c)
            );
            if let Some(group) = st.groups.get_mut(&c) {
                group.members.extend(picked);
            }
            Next::Finished(json!({ "title": "已邀请", "message": message }))
        }
        // 退出群：只在内存里记下（群主退出时群主交给下一个人）。
        (
            Stage::Leave(c),
            Choose {
                choice: "confirm", ..
            },
        ) => {
            if let Some(group) = st.groups.get_mut(&c) {
                group.members.retain(|m| m != ME);
                if group.owner == ME
                    && let Some(next) = group.members.first()
                {
                    group.owner = next.clone();
                }
            }
            Next::Finished(json!({ "title": format!("已退出「{}」", group_name(&c)),
                "message": "演示插件只在内存里记下这件事，重启插件后你会回到群里。" }))
        }
        // 删除会话（v19）：平台上什么都不动，只请宿主把它从列表里拿掉；不给标题与说明，对话框直接关掉。
        (
            Stage::Delete(c),
            Choose {
                choice: "confirm", ..
            },
        ) => {
            eprintln!("sample_provider: 删除会话 {c}，请宿主从列表里拿掉");
            Next::Finished(json!({ "removeFromList": true }))
        }
        // 批准新设备。
        (Stage::Approve, Submit(values)) => {
            let typed = values["words"]
                .as_str()
                .unwrap_or("")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            let Some(wrong) = st.approve_wrong else {
                return Next::Failed(json!({ "title": "请求已作废",
                    "message": "新设备取消了这次请求。", "retryable": false }));
            };
            if typed == APPROVE_WORDS {
                eprintln!("sample_provider: 批准了新设备「示例平板」");
                st.approve_wrong = None;
                if !st.devices.iter().any(|d| d.id == "d-tablet") {
                    st.devices
                        .push(device("d-tablet", "示例平板", "Android · 刚刚批准", false));
                }
                Next::Finished(json!({ "title": "已批准「示例平板」",
                    "message": "那台设备正在补齐聊天记录。你的其它设备也会收到提醒。" }))
            } else if wrong + 1 >= APPROVE_TRIES {
                eprintln!("sample_provider: 连续 {APPROVE_TRIES} 次不一致，请求作废");
                st.approve_wrong = None;
                Next::Failed(json!({ "title": "请求已作废",
                    "message": "连续 5 次和新设备上的不一致，这次请求已作废。需要的话，在新设备上重新请求。",
                    "retryable": false }))
            } else {
                st.approve_wrong = Some(wrong + 1);
                Next::Step(Stage::Approve, approve_step(wrong + 1))
            }
        }
        (
            Stage::Approve,
            Choose {
                choice: "reject", ..
            },
        ) => {
            eprintln!("sample_provider: 拒绝了新设备「示例平板」");
            st.approve_wrong = None;
            Next::Failed(json!({ "title": "已拒绝",
                "message": "那台设备不会加入。如果不是你本人在操作，建议尽快修改密码。", "retryable": false }))
        }
        _ => Next::Ignore,
    }
}

/// 给一步带上提示（`note`）；没有就不带。
fn with_note(mut step: Value, text: Option<&str>, tone: &str) -> Value {
    if let Some(text) = text {
        step["note"] = json!({ "text": text, "tone": tone });
    }
    step
}

/// 第几版的恢复密钥：12 个不重复的词。
fn recovery_key(round: usize) -> String {
    (0..12)
        .map(|i| KEY_WORDS[(i * 5 + round * 7) % KEY_WORDS.len()])
        .collect::<Vec<_>>()
        .join(" ")
}

/// 出示恢复密钥：12 个编号的词、能复制、勾上“我已保存”才能点“完成”，不能跳过。
fn key_step(round: usize, caption: &str) -> Value {
    json!({ "kind": "showText", "caption": caption, "text": recovery_key(round), "layout": "words", "copy": true,
        "ack": "我已保存恢复密钥", "confirmLabel": "完成", "noCancel": true,
        "note": { "text": "别发给任何人：拿到它的人能读你的全部消息。服务器上没有它，丢了谁也找不回。", "tone": "warn" } })
}

fn device(id: &str, name: &str, detail: &str, this: bool) -> Device {
    Device {
        id: id.into(),
        name: name.into(),
        detail: detail.into(),
        this,
    }
}

fn demo_devices() -> Vec<Device> {
    vec![
        device("d-this", "这台电脑", "Windows · 现在在用", true),
        device("d-office", "办公室电脑", "Windows · 3 天前在线", false),
        device("d-laptop", "旧笔记本", "macOS · 上个月在线", false),
    ]
}

/// 我的设备：每台一行（本机标“本机”、没有按钮，别的行尾“移除”），下面“重新生成恢复密钥”。
fn devices_step(st: &State, note: Option<&str>) -> Value {
    let rows: Vec<Value> = st
        .devices
        .iter()
        .map(|d| {
            let mut row =
                json!({ "id": d.id, "title": d.name, "detail": d.detail, "icon": "device" });
            if d.this {
                row["tag"] = json!("本机");
            } else {
                row["actions"] = json!([{ "id": "remove", "label": "移除", "danger": true }]);
            }
            row
        })
        .collect();
    with_note(
        json!({ "kind": "list", "caption": "登录着这个账号的设备。移除后，那台设备读不到之后的新消息。",
            "rows": rows, "empty": "没有登录着的设备。", "links": [{ "id": "regen", "label": "重新生成恢复密钥" }] }),
        note,
        "info",
    )
}

/// 我的链接：一行等宽、能复制，下面“换一条链接”。
fn link_step(st: &State, note: Option<&str>) -> Value {
    let link = format!(
        "https://im.example.com/u/{}",
        LINK_CODES[st.link_round % LINK_CODES.len()]
    );
    with_note(
        json!({ "kind": "showText", "layout": "line", "text": link, "copy": true,
            "caption": "把这条链接发给想加你的人。对方在示例 IM 里粘贴它就能加你为好友。",
            "links": [{ "id": "renew", "label": "换一条链接" }] }),
        note,
        "info",
    )
}

/// 加个朋友：填好友链接（填得不对时带警告）。
fn add_step(warn: Option<&str>) -> Value {
    with_note(
        json!({ "kind": "input", "caption": "粘贴对方发给你的好友链接。",
            "fields": [{ "id": "link", "label": "好友链接", "secret": false, "placeholder": "https://im.example.com/u/…" }],
            "submitLabel": "下一步" }),
        warn,
        "warn",
    )
}

/// 关于示例：几段带小标题的说明，只有“知道了”。
fn about_step() -> Value {
    json!({ "kind": "confirm", "confirmLabel": "知道了", "noCancel": true, "sections": [
        { "title": "这是什么", "text": "示例 IM 是 HHH 的演示插件：不联网，所有数据都在插件进程的内存里，重启插件就回到初始状态。" },
        { "title": "能演示什么", "text": "用宿主账号登录、管理菜单与会话菜单里的操作、请你处理的提醒，以及出示文字、列表、确认三种步骤。" },
        { "title": "怎么触发", "text": "在插件的工作目录里放 approve、host-key、host-fail 这些文件，见源码开头的说明。" }
    ] })
}

/// 群名（同 [`conversation`]）。
fn group_name(conversation_id: &str) -> String {
    conversation(conversation_id)
        .and_then(|c| c["name"].as_str().map(str::to_owned))
        .unwrap_or_default()
}

fn demo_groups() -> BTreeMap<String, Group> {
    let group = |owner: &str, members: &[&str]| Group {
        owner: owner.to_owned(),
        members: members.iter().map(|m| (*m).to_owned()).collect(),
    };
    BTreeMap::from([
        ("c-team".to_owned(), group(ME, &[ME, "u-amy", "u-bo"])),
        (
            "c-club".to_owned(),
            group("u-bo", &["u-bo", ME, "u-amy", "u-lin"]),
        ),
        ("c-news".to_owned(), group("u-bo", &["u-bo", ME])),
    ])
}

/// 群成员：群主标“群主”；你是群主时别人行尾有“移出”。
fn members_step(st: &State, conversation_id: &str, note: Option<&str>) -> Value {
    let Some(group) = st.groups.get(conversation_id) else {
        return json!({ "kind": "list", "rows": [], "empty": "示例 IM 里没有这个群。" });
    };
    let mine = group.owner == ME;
    let rows: Vec<Value> = group
        .members
        .iter()
        .map(|id| {
            let mut row = json!({ "id": id, "title": member_name(st, id), "icon": "person" });
            if id == ME {
                row["detail"] = json!("你自己");
            } else if st.friends.contains(id) {
                row["detail"] = json!("好友");
            }
            if *id == group.owner {
                row["tag"] = json!("群主");
            }
            if mine && id != ME {
                row["actions"] = json!([{ "id": "remove", "label": "移出", "danger": true }]);
            }
            row
        })
        .collect();
    let caption = if mine {
        format!("{} 位成员。你是群主，可以移出成员。", rows.len())
    } else {
        format!(
            "{} 位成员。只有群主{}能移出成员。",
            rows.len(),
            member_name(st, &group.owner)
        )
    };
    with_note(
        json!({ "kind": "list", "caption": caption, "rows": rows, "empty": "群里没有人了。" }),
        note,
        "info",
    )
}

/// 群成员、群主的名字：自己用当前账号的名字（用宿主账号登录时是宿主账号的）。
fn member_name(st: &State, id: &str) -> String {
    if id == ME {
        st.account["name"].as_str().unwrap_or("示例用户").to_owned()
    } else {
        person(id).to_owned()
    }
}

/// 邀请成员：勾选不在群里的好友（至少 1 个），下面“改用群邀请链接”。
fn invite_step(st: &State, conversation_id: &str, warn: Option<&str>) -> Value {
    let members = st
        .groups
        .get(conversation_id)
        .map(|g| g.members.as_slice())
        .unwrap_or_default();
    let rows: Vec<Value> = st
        .friends
        .iter()
        .filter(|f| !members.contains(f))
        .map(|id| json!({ "id": id, "title": person(id), "icon": "person" }))
        .collect();
    with_note(
        json!({ "kind": "list", "caption": "新成员能看到群里以前的全部消息。", "rows": rows,
            "select": { "min": 1, "confirmLabel": "邀请" }, "empty": "你的好友都已经在群里了。",
            "links": [{ "id": "groupLink", "label": "改用群邀请链接" }] }),
        warn,
        "warn",
    )
}

/// 退出群：危险的确认。
fn leave_step(conversation_id: &str) -> Value {
    json!({ "kind": "confirm", "caption": format!("退出「{}」？", group_name(conversation_id)), "danger": true,
        "confirmLabel": "退出群", "text": "退出后读不到群里之后的新消息；以前的消息还在这台设备上。要回来需要群里的人重新邀请。" })
}

/// 删除会话：可撤销的确认（不用危险色）。
fn delete_step(conversation_id: &str) -> Value {
    let name = conversation(conversation_id)
        .and_then(|c| c["name"].as_str().map(str::to_owned))
        .unwrap_or_default();
    json!({ "kind": "confirm", "caption": format!("删除「{name}」？"), "confirmLabel": "删除",
        "text": "只从你的列表里拿掉，聊天记录还在；有新消息时会再出现。" })
}

/// 批准新设备：填 4 个词（“拒绝 / 批准”，“拒绝”是一个选择）；输错过就带“还能试几次”的警告。
fn approve_step(wrong: u32) -> Value {
    let step = json!({ "kind": "input",
        "caption": "新设备「示例平板」想登录你的示例 IM 账号。如果是你本人在操作，输入那台设备上显示的 4 个词。",
        "fields": [{ "id": "words", "label": "新设备上的 4 个词", "secret": false, "placeholder": "用空格隔开" }],
        "submitLabel": "批准", "cancelLabel": "拒绝", "cancelChoice": "reject" });
    if wrong == 0 {
        with_note(
            step,
            Some("不是你本人在操作就点“拒绝”：批准后，那台设备能读你的全部消息。"),
            "info",
        )
    } else {
        let warn = format!(
            "和新设备上的不一致，再看一下（还能试 {} 次）。",
            APPROVE_TRIES - wrong
        );
        with_note(step, Some(&warn), "warn")
    }
}

/// 这个连接现在的全部提醒（`session/notice` 每次给全）：缺权限（`notice` 开关）与等批准的新设备（`approve` 开关，带
/// “查看”与 `prompt`）。
fn notices(st: &State) -> Value {
    let mut list = Vec::new();
    if st.notice_scope {
        list.push(json!({ "id": "scope", "text": "示例 IM 缺少读消息记录的权限：重新登录补上。", "relogin": true }));
    }
    if st.approve_wrong.is_some() {
        list.push(
            json!({ "id": "approve-1", "text": "新设备「示例平板」等你批准",
            "action": { "id": APPROVE_ACTION, "label": "查看" }, "prompt": true }),
        );
    }
    json!({ "notices": list })
}

/// 一份新凭据（签发时间与账号）的 `credential/save` 参数，60 秒后续期。
fn credential_params(state: &Arc<Mutex<State>>) -> Value {
    let issued = now_ms();
    let credential =
        json!({ "issuedAtMs": issued, "account": state.lock().unwrap().account.clone() });
    eprintln!("sample_provider: 存新凭据（签发于 {issued}）");
    json!({ "credential": credential.to_string(), "renewAtMs": issued + RENEW_AFTER_MS })
}

/// 存一份新凭据并等宿主回复（登录、交接、续期都是存好了才往下走，协议 §6.3 先存后用）。宿主 10 秒不回、回了错误都算没存上。
fn save_credential_and_wait(
    state: &Arc<Mutex<State>>,
    out: &Out,
    next_id: &Arc<AtomicI64>,
    waiting: &Waiting,
) -> Result<(), String> {
    let id = next_id.fetch_add(1, Ordering::SeqCst);
    let (tx, rx) = std::sync::mpsc::channel();
    waiting.lock().unwrap().insert(id, tx);
    send(
        out,
        RpcMessage::request(
            RpcId::Number(id),
            "credential/save",
            credential_params(state),
        ),
    );
    let reply = rx.recv_timeout(std::time::Duration::from_secs(10));
    waiting.lock().unwrap().remove(&id);
    match reply {
        Ok(reply) if reply.error.is_none() => Ok(()),
        Ok(reply) => Err(format!(
            "宿主没存上新凭据：{}",
            reply.error.map(|e| e.message).unwrap_or_default()
        )),
        Err(_) => Err("宿主 10 秒没回存凭据".into()),
    }
}

fn default_account() -> Value {
    json!({ "id": "me", "name": "示例用户", "tenant": "示例团队" })
}

/// 从凭据里取回账号；凭据不是这个插件写的 JSON（旧版写的、空的）就用默认账号。
fn adopt_account(state: &Arc<Mutex<State>>, credential: Option<&str>) {
    let account = credential
        .and_then(|c| serde_json::from_str::<Value>(c).ok())
        .map(|c| c["account"].clone())
        .filter(|a| a["id"].as_str().is_some_and(|id| !id.is_empty()))
        .unwrap_or_else(default_account);
    state.lock().unwrap().account = account;
}

/// 按现在的规则翻译一条消息：`rule-v2` 在时文字前加“【新版翻译】”（与 `messages/retranslate` 按 `sample-v2` 重译的结果一样）。
fn translated(message: &Value) -> Value {
    let mut m = message.clone();
    if std::path::Path::new("rule-v2").exists()
        && let Some(data) = m["raw"]["data"].as_str()
    {
        m["content"] = json!([{ "type": "text", "text": format!("【新版翻译】{data}") }]);
    }
    m
}

/// 会话与人推给宿主（宿主在本机替插件搜群、搜人）。
fn push_conversations(out: &Out, next_id: &Arc<AtomicI64>) {
    let events: Vec<Value> = ["c-team", "c-club", "c-direct"]
        .iter()
        .filter_map(|id| conversation(id))
        .map(|c| json!({ "type": "conversation", "conversation": c }))
        .collect();
    request(
        out,
        next_id,
        "events/push",
        json!({ "events": events, "syncState": "sample-1" }),
    );
}

fn conversation(id: &str) -> Option<Value> {
    Some(match id {
        "c-team" => json!({ "id": "c-team", "kind": "group", "name": "示例团队群" }),
        "c-club" => json!({ "id": "c-club", "kind": "group", "name": "示例读书会" }),
        "c-news" => json!({ "id": "c-news", "kind": "group", "name": "示例公告" }),
        "c-direct" => {
            json!({ "id": "c-direct", "kind": "direct", "name": "阿美", "peer": "u-amy", "private": true })
        }
        _ => return None,
    })
}

fn person(id: &str) -> &'static str {
    match id {
        "me" => "示例用户",
        "u-amy" => "阿美",
        "u-bo" => "阿博",
        "u-lin" => "小林",
        "u-dave" => "大卫",
        "u-xm" => "王小明",
        _ => "示例成员",
    }
}

static READ_POSITION_SERIAL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// 一条消息：文字、原始数据；本人发的、能编辑的带编辑框原文。
fn message(
    id: &str,
    conversation: &str,
    sender: &str,
    at_ms: i64,
    text: &str,
    editable: bool,
) -> Value {
    let mut m = json!({
        "id": id, "conversationId": conversation,
        "sender": { "id": sender, "kind": "user", "name": person(sender) },
        "readPosition": (u128::try_from(at_ms).unwrap_or(0)*1_000_000 + u128::from(READ_POSITION_SERIAL.fetch_add(1,Ordering::Relaxed))).to_string(),
        "createdAtMs": at_ms,
        "content": [ { "type": "text", "text": text } ],
        "raw": { "format": "sample-msg-v1", "data": text }
    });
    if editable {
        m["editDraft"] = json!([{ "type": "text", "text": text }]);
    }
    m
}

fn demo_messages() -> Vec<Value> {
    let base = now_ms() - 3_600_000;
    let mut thread_reply = message(
        "m-4",
        "c-team",
        "u-bo",
        base + 40_000,
        "话题里的回复，也发到了群里",
        false,
    );
    thread_reply["thread"] = json!({ "id": "t-1", "rootId": "m-1", "inThread": true,
        "copy": { "role": "thread", "peerId": "m-4c", "threadId": "t-1" } });
    let mut copy = message(
        "m-4c",
        "c-team",
        "u-bo",
        base + 40_000,
        "话题里的回复，也发到了群里",
        false,
    );
    copy["thread"] =
        json!({ "copy": { "role": "conversation", "peerId": "m-4", "threadId": "t-1" } });
    // 第一条是话题 t-1 的第一条（带话题编号与现状，宿主据此按话题读回复、给会话里那份副本找到话题）。
    let mut root = message(
        "m-1",
        "c-team",
        "u-amy",
        base,
        "欢迎来到示例 IM {赞}",
        false,
    );
    root["thread"] = json!({ "id": "t-1", "inThread": false,
        "activity": { "latestReplyId": "m-4", "latestReplyAtMs": base + 40_000, "replyCount": 1 } });
    let mut messages = vec![
        root,
        message("m-2", "c-team", "me", base + 10_000, "这条可以编辑", true),
        message(
            "m-3",
            "c-team",
            "me",
            base + 20_000,
            "这条平台说不能编辑",
            false,
        ),
        thread_reply,
        copy,
        message(
            "m-5",
            "c-direct",
            "u-amy",
            base + 50_000,
            "私聊里的一条",
            false,
        ),
        message(
            "m-6",
            "c-news",
            "u-bo",
            base - 86_400_000,
            "很早以前的公告",
            false,
        ),
        message(
            "m-7",
            "c-club",
            "u-bo",
            base + 30_000,
            "这周读《小王子》，周五晚上聊",
            false,
        ),
    ];
    // 显式的协议示例，只有任务目录放入标记才出现，不改变既有示例验收的数据。
    if std::path::Path::new("link-cards").exists()
        || std::env::args().any(|arg| arg == "--link-cards")
    {
        for (i, block) in [
            json!({"type":"linkCard","url":"https://example.com/article","title":"从一张卡片打开原文","description":"标题、封面和空白处都打开同一篇文章。","source":"示例来源","thumbnail":{"key":"link-cover","kind":"image"}}),
            json!({"type":"linkCard","url":"https://example.com/no-cover","title":"没有封面的文章","description":"文字和链接不依赖封面。"}),
            json!({"type":"linkCard","url":"https://example.com/missing-cover","title":"封面无法读取仍能打开原文","thumbnail":{"key":"link-cover-missing","kind":"image"}}),
            json!({"type":"linkCard","url":"https://example.com/long","title":"比较长的文章标题在窄窗口中最多显示两行，仍然可以通过整张卡片打开完整的原文进行阅读","description":"摘要在卡片中最多显示三行，不会把会话挤出水平方向的滚动条。".repeat(4)}),
            json!({"type":"linkCard","url":"https://example.com/only-url"}),
        ].into_iter().enumerate() {
            let mut m = message(&format!("link-{i}"), "c-team", "u-amy", base + 60_000 + i as i64, "", false);
            m["content"] = json!([block]);
            m.as_object_mut().unwrap().remove("raw");
            messages.push(m);
        }
    }
    messages
}

/// 第一条消息上的表情回应：目录里的“赞”与目录外的 `wave`（宿主按 `guess` 来要图）。
fn demo_reactions() -> Vec<Value> {
    vec![
        json!({ "id": "r-1", "conversationId": "c-team", "messageId": "m-1",
            "operator": { "id": "u-amy", "kind": "user", "name": "阿美" }, "emojiType": "thumbsup", "atMs": 1_700_000_100_000i64 }),
        json!({ "id": "r-2", "conversationId": "c-team", "messageId": "m-1",
            "operator": { "id": "u-bo", "kind": "user", "name": "阿博" }, "emojiType": "wave", "atMs": 1_700_000_200_000i64 }),
    ]
}

/// 会话发现：这段时间里别人发过消息的会话。
fn recent_activity(params: &Value, state: &Arc<Mutex<State>>) -> Value {
    let since = params["sinceMs"].as_i64().unwrap_or(0);
    let state = state.lock().unwrap();
    let mut items: BTreeMap<String, Value> = BTreeMap::new();
    for m in &state.messages {
        let at = m["createdAtMs"].as_i64().unwrap_or(0);
        let conversation = m["conversationId"].as_str().unwrap_or("");
        if at < since || m["sender"]["id"] == "me" || m["thread"]["inThread"] == true {
            continue;
        }
        let kind = if conversation == "c-direct" {
            "direct"
        } else {
            "group"
        };
        let entry = items.entry(conversation.to_owned()).or_insert_with(|| {
            json!({ "conversationId": conversation, "kind": kind, "firstMs": at, "latestMs": at })
        });
        entry["firstMs"] = json!(entry["firstMs"].as_i64().unwrap_or(at).min(at));
        entry["latestMs"] = json!(entry["latestMs"].as_i64().unwrap_or(at).max(at));
        if kind == "direct" {
            entry["peerId"] = m["sender"]["id"].clone();
        }
    }
    let mut items: Vec<Value> = items.into_values().collect();
    items.sort_by_key(|i| std::cmp::Reverse(i["latestMs"].as_i64().unwrap_or(0)));
    json!({ "items": items, "complete": true })
}

/// 读一页消息（会话里的或一个话题里的），并在标准错误里写距上次读这个会话多久。
fn list_messages(params: &Value, state: &Arc<Mutex<State>>) -> Value {
    let conversation = params["conversationId"].as_str().unwrap_or("");
    let thread = params["threadId"].as_str();
    let after = params["afterMs"].as_i64();
    let before = params["beforeMs"].as_i64();
    let limit = params["limit"].as_u64().unwrap_or(50) as usize;
    let mut state = state.lock().unwrap();
    let now = now_ms();
    let key = format!(
        "{conversation}{}",
        thread.map(|t| format!("/{t}")).unwrap_or_default()
    );
    match state.last_read.insert(key.clone(), now) {
        Some(last) => eprintln!(
            "sample_provider: messages/list {key}（距上次 {:.1} 秒）",
            (now - last) as f64 / 1000.0
        ),
        None => eprintln!("sample_provider: messages/list {key}（第一次）"),
    }
    let mut items: Vec<Value> = state
        .messages
        .iter()
        .filter(|m| m["conversationId"] == conversation)
        .filter(|m| match thread {
            Some(t) => m["thread"]["id"] == t || m["id"] == "m-1",
            None => m["thread"]["inThread"] != true,
        })
        .filter(|m| {
            let at = m["createdAtMs"].as_i64().unwrap_or(0);
            after.is_none_or(|a| at > a) && before.is_none_or(|b| at < b)
        })
        .map(translated)
        .collect();
    items.sort_by_key(|m| m["createdAtMs"].as_i64().unwrap_or(0));
    if params["newestFirst"].as_bool().unwrap_or(false) {
        items.reverse();
    }
    items.truncate(limit.max(1));
    json!({ "items": items })
}

fn send_message(params: &Value, state: &Arc<Mutex<State>>) -> Value {
    let op = params["operationId"].as_str().unwrap_or("op");
    let conversation = params["target"]["conversationId"]
        .as_str()
        .unwrap_or("c-team");
    let text: String = params["content"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|block| block["text"].as_str())
        .collect();
    let mut sent = message(
        &format!("sent-{op}"),
        conversation,
        "me",
        now_ms(),
        &text,
        true,
    );
    if let Some(id) = params["content"]
        .as_array()
        .and_then(|v| v.first())
        .filter(|v| v["type"] == "sticker")
        .and_then(|v| v["id"].as_str())
    {
        sent["content"] = json!([{"type":"resource","resource":{"key":id,"kind":"sticker"},"width":120,"height":120}]);
        sent.as_object_mut().unwrap().remove("editDraft");
    }
    let mut state = state.lock().unwrap();
    if params["target"]["inThread"] == true {
        let target = state
            .messages
            .iter()
            .find(|message| message["id"] == params["target"]["messageId"]);
        let root = target
            .and_then(|message| message["thread"]["rootId"].as_str())
            .or_else(|| params["target"]["messageId"].as_str())
            .unwrap_or("m-1");
        let thread = target
            .and_then(|message| message["thread"]["id"].as_str())
            .unwrap_or(root);
        sent["thread"] = json!({"id":thread,"rootId":root,"inThread":true});
    }
    state.messages.push(sent.clone());
    json!({ "state": "accepted", "message": sent })
}

fn edit_message(params: &Value, state: &Arc<Mutex<State>>) -> Result<Value, RpcError> {
    let id = params["messageId"].as_str().unwrap_or("");
    let text: String = params["content"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|block| block["text"].as_str())
        .collect();
    let mut state = state.lock().unwrap();
    let Some(m) = state.messages.iter_mut().find(|m| m["id"] == id) else {
        return Err(error("notFound", "没有这条消息。"));
    };
    if m.get("editDraft").is_none() {
        return Err(error("permissionDenied", "示例 IM 说这条不能编辑。"));
    }
    m["content"] = json!([{ "type": "text", "text": text }]);
    m["raw"] = json!({ "format": "sample-msg-v1", "data": text });
    m["editDraft"] = json!([{ "type": "text", "text": text }]);
    m["editedAtMs"] = json!(now_ms());
    Ok(m.clone())
}

/// 按规则名重译：`sample-v2` 在文字前加“【新版翻译】”，`sample-v1` 照原文。
fn retranslate(params: &Value, state: &Arc<Mutex<State>>) -> Value {
    let rule = params["rule"].as_str().unwrap_or("");
    eprintln!(
        "sample_provider: 按 {rule} 重译 {} 条",
        params["messages"].as_array().map_or(0, Vec::len)
    );
    let state = state.lock().unwrap();
    let messages: Vec<Value> = params["messages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| {
            let mut m = state
                .messages
                .iter()
                .find(|m| m["id"] == item["id"])?
                .clone();
            let data = item["raw"]["data"].as_str().unwrap_or("");
            let text = if rule == "sample-v2" {
                format!("【新版翻译】{data}")
            } else {
                data.to_owned()
            };
            m["content"] = json!([{ "type": "text", "text": text }]);
            Some(m)
        })
        .collect();
    json!({ "messages": messages })
}

fn chunk(params: &Value, png: &[u8]) -> Value {
    let offset = params["offset"].as_u64().unwrap_or(0) as usize;
    let length = params["length"].as_u64().unwrap_or(png.len() as u64) as usize;
    let end = (offset + length).min(png.len());
    let slice = if offset < png.len() {
        &png[offset..end]
    } else {
        &[]
    };
    json!({ "data": base64_encode(slice), "total": png.len(), "eof": end >= png.len() })
}

/// 每 300 毫秒看一次工作目录里的开关文件（见文件头）。
fn watch_switches(state: Arc<Mutex<State>>, out: Out) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(300));
            {
                let mut state = state.lock().unwrap();
                let State {
                    reading, messages, ..
                } = &mut *state;
                if let Err(error) = reading.refresh_messages(messages) {
                    eprintln!("sample_provider: 示例共享消息读取失败：{error}");
                }
            }
            if std::fs::remove_file("expire").is_ok() {
                eprintln!("sample_provider: 开关 expire，说登录失效");
                state.lock().unwrap().logged_in = false;
                notify(
                    &out,
                    "session/expired",
                    json!({ "reason": "expired", "message": "示例 IM 的登录过期了，请重新登录。" }),
                );
            }
            // 提醒每次给全（协议 §6.5）：缺权限的与等批准的各自开关，发的时候两条都按现状带上。
            if std::fs::remove_file("notice").is_ok() {
                let params = {
                    let mut st = state.lock().unwrap();
                    st.notice_scope = true;
                    notices(&st)
                };
                notify(&out, "session/notice", params);
            }
            if std::fs::remove_file("notice-clear").is_ok() {
                let params = {
                    let mut st = state.lock().unwrap();
                    st.notice_scope = false;
                    notices(&st)
                };
                notify(&out, "session/notice", params);
            }
            if std::fs::remove_file("approve").is_ok() {
                eprintln!("sample_provider: 开关 approve，新设备「示例平板」等你批准");
                let params = {
                    let mut st = state.lock().unwrap();
                    st.approve_wrong.get_or_insert(0);
                    notices(&st)
                };
                notify(&out, "session/notice", params);
            }
            if std::fs::remove_file("approve-clear").is_ok() {
                eprintln!("sample_provider: 开关 approve-clear，撤掉等批准的请求");
                let (params, open) = {
                    let mut st = state.lock().unwrap();
                    st.approve_wrong = None;
                    let open = st.active.take_if(|a| matches!(a.stage, Stage::Approve));
                    (notices(&st), open)
                };
                // 批准框正开着：这次请求作废了。
                if let Some(active) = open {
                    notify(
                        &out,
                        "action/failed",
                        json!({ "flowId": active.flow_id, "title": "请求已作废",
                                "message": "新设备取消了这次请求。", "retryable": false }),
                    );
                }
                notify(&out, "session/notice", params);
            }
            // 文件里写了示例 IM 的会话编号（如 `c-direct`）就来在那个会话里（删除会话后看它回到列表，v19），否则在示例公告。
            if let Ok(named) = std::fs::read_to_string("new-message")
                && std::fs::remove_file("new-message").is_ok()
            {
                let named = named.trim();
                let (chat, sender, text) = match conversation(named) {
                    Some(c) if named == "c-direct" => (
                        named,
                        "u-amy",
                        format!("{}：在吗？", c["name"].as_str().unwrap_or("")),
                    ),
                    Some(c) => (
                        named,
                        "u-bo",
                        format!("{}：有一条新消息", c["name"].as_str().unwrap_or("")),
                    ),
                    None => ("c-news", "u-bo", "示例公告：今天下午三点开会".to_owned()),
                };
                let mut state = state.lock().unwrap();
                let n = state.messages.len();
                let m = message(&format!("m-new-{n}"), chat, sender, now_ms(), &text, false);
                if let Err(error) = state.reading.publish_message(&m) {
                    eprintln!("sample_provider: 示例共享消息写入失败：{error}");
                    continue;
                }
                state.messages.push(m);
                eprintln!("sample_provider: 开关 new-message，{chat} 来了一条新消息");
            }
            if std::fs::remove_file("edit").is_ok() {
                let mut state = state.lock().unwrap();
                if let Some(m) = state.messages.iter_mut().find(|m| m["id"] == "m-1") {
                    let text = "欢迎来到示例 IM（已改过）{赞}";
                    m["content"] = json!([{ "type": "text", "text": text }]);
                    m["raw"] = json!({ "format": "sample-msg-v1", "data": text });
                    m["updatedAtMs"] = json!(now_ms());
                }
                eprintln!("sample_provider: 开关 edit，改了第一条");
            }
        }
    });
}

/// 本机回环的假授权页：`/authorize` 给“同意 / 拒绝”，`/approve` 完成（自己的应用方式跳回宿主的回调地址），`/deny` 失败。
fn serve_auth_pages(
    state: Arc<Mutex<State>>,
    out: Out,
    next_id: Arc<AtomicI64>,
    waiting: Waiting,
) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("假授权页监听本机端口");
    let port = listener.local_addr().map_or(0, |a| a.port());
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let (state, out, next_id, waiting) =
                (state.clone(), out.clone(), next_id.clone(), waiting.clone());
            std::thread::spawn(move || serve(stream, &state, &out, &next_id, &waiting));
        }
    });
    port
}

fn serve(
    mut stream: TcpStream,
    state: &Arc<Mutex<State>>,
    out: &Out,
    next_id: &Arc<AtomicI64>,
    waiting: &Waiting,
) {
    let mut first = String::new();
    let mut reader = BufReader::new(stream.try_clone().expect("复制连接"));
    if reader.read_line(&mut first).is_err() {
        return;
    }
    // 读完请求头（假授权页只收 GET）。
    let mut line = String::new();
    while reader.read_line(&mut line).is_ok_and(|n| n > 2) {
        line.clear();
    }
    let target = first.split_whitespace().nth(1).unwrap_or("/");
    let path = target.split('?').next().unwrap_or("/");
    let query = query_of(target);
    let key = query.get("state").cloned().unwrap_or_default();
    let flow = {
        let state = state.lock().unwrap();
        state
            .flows
            .get(&key)
            .map(|f| (f.flow_id.clone(), f.callback.clone()))
    };
    let (status, headers, body) = match (path, flow) {
        // 登录窗口方式（v36）的假登录页：“完成登录”跳到 `hhh-sample://`（宿主截下交回来），另有一个打开别的 App 的链接
        // （宿主应当拦下，验收用）。
        ("/window", Some(_)) => (
            "200 OK",
            String::new(),
            sample_page(
                "登录示例 IM",
                "演示插件的假登录页：点“完成登录”就算登录成功。",
                &format!(
                    "<div class=\"actions\"><a class=\"button\" id=\"finish\" href=\"{WINDOW_CAPTURE}done?state={0}&code={CODE}\">完成登录</a>\
                     <a class=\"quiet\" id=\"other-app\" href=\"other-app://open?state={0}\">打开别的 App</a></div>",
                    encode(&key)
                ),
            ),
        ),
        ("/authorize", Some(_)) => (
            "200 OK",
            String::new(),
            sample_page(
                "授权示例 IM",
                "演示插件的假授权页：点“同意”就算授权成功。",
                &format!(
                    "<div class=\"actions\"><a class=\"button\" href=\"/approve?state={0}\">同意</a>\
                     <a class=\"quiet\" href=\"/deny?state={0}\">拒绝</a></div>",
                    encode(&key)
                ),
            ),
        ),
        ("/approve", Some((_, Some(callback)))) => {
            let sep = if callback.contains('?') { '&' } else { '?' };
            (
                "302 Found",
                format!(
                    "Location: {callback}{sep}code={CODE}&state={}\r\n",
                    encode(&key)
                ),
                String::new(),
            )
        }
        ("/approve", Some((flow_id, None))) => {
            state.lock().unwrap().flows.remove(&key);
            complete_login(&flow_id, state, out, next_id, waiting);
            (
                "200 OK",
                String::new(),
                sample_page(
                    "示例 IM 授权已完成",
                    "请返回 HHH 等待连接完成，这个页面可以关掉了。",
                    "",
                ),
            )
        }
        ("/deny", Some((flow_id, _))) => {
            state.lock().unwrap().flows.remove(&key);
            notify(
                out,
                "login/failed",
                json!({ "flowId": flow_id, "message": "你在示例 IM 里拒绝了授权。", "retryable": true }),
            );
            (
                "200 OK",
                String::new(),
                sample_page(
                    "没有完成授权",
                    "请返回 HHH 查看原因后重试，这个页面可以关掉了。",
                    "",
                ),
            )
        }
        _ => (
            "404 Not Found",
            String::new(),
            sample_page("这次登录已经结束", "请回到 HHH 重新登录。", ""),
        ),
    };
    let _ = write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n{headers}\r\n{body}",
        body.len()
    );
    let _ = stream.flush();
}

/// 示例插件自己页面的样式（v37）：照抄中继收据页（hhh-im-callback-server `relay.mjs` 的 `RECEIPT_STYLE`）右侧一栏的版式，
/// 按钮照抄 Friend 登录页的 `.login-button` / `.login-primary` / `.login-quiet`。示例插件是独立程序，不带 HHH 的红花图；
/// 页面里没有脚本与外部资源。
const SAMPLE_PAGE_STYLE: &str = concat!(
    ":root{color-scheme:dark;--bg:#0d1117;--text:#d7dde4;--dim:#8b949e;--line-strong:#3a444f;--panel:#14181d;--red:#e5484d;",
    "--font-sans:'Inter Variable',Inter,-apple-system,BlinkMacSystemFont,'PingFang SC','Hiragino Sans GB','HarmonyOS Sans SC',",
    "'Microsoft YaHei UI','Microsoft YaHei','Noto Sans CJK SC','Source Han Sans SC','Segoe UI',system-ui,sans-serif}",
    "@media (prefers-color-scheme:light){:root{color-scheme:light;--bg:#f6f8fa;--text:#1f2328;--dim:#57606a;--line-strong:#afb8c1;--panel:#eaeef2;--red:#cf222e}}",
    "*{box-sizing:border-box;margin:0}html,body{height:100%}",
    "body{background:var(--bg);color:var(--text);font:400 14px/1.6 var(--font-sans);word-break:keep-all;overflow-wrap:anywhere}",
    ".screen{display:grid;grid-template-columns:minmax(0,1fr);height:100dvh;overflow:hidden}",
    ".stage{display:flex;min-width:0;min-height:0;overflow:auto}",
    "main{width:min(440px,calc(100% - 120px));margin:auto;padding:64px 0;animation:enter 360ms cubic-bezier(.4,0,.2,1) both}",
    ".eyebrow{display:flex;align-items:center;gap:9px;margin-bottom:20px;font-size:12px;font-weight:600;letter-spacing:.14em;color:var(--dim)}",
    ".eyebrow::before{content:'';width:6px;height:6px;border-radius:50%;background:var(--red)}",
    "h1{margin-bottom:10px;font-size:26px;font-weight:600;line-height:1.4;letter-spacing:-.025em}",
    ".lead{color:var(--dim);line-height:1.8}",
    ".actions{display:flex;flex-direction:column;align-items:stretch;gap:4px;margin-top:28px}",
    ".button{display:inline-flex;min-height:46px;align-items:center;justify-content:center;border-radius:999px;padding:0 17px;",
    "font:600 14px/1 var(--font-sans);border:1px solid var(--line-strong);background:transparent;color:var(--text);text-decoration:none;",
    "transition:background-color .16s,border-color .16s}",
    ".button:hover{background:color-mix(in srgb,var(--panel) 98%,var(--text));border-color:var(--dim)}",
    ".button:active{background:var(--panel);border-color:var(--text)}",
    ".quiet{display:inline-flex;min-height:44px;align-items:center;justify-content:center;color:var(--dim);font:400 13px/1 var(--font-sans);",
    "text-decoration:none;transition:color .16s}",
    ".quiet:hover{color:var(--text)}",
    ".button:focus-visible,.quiet:focus-visible{outline:2px solid var(--dim);outline-offset:3px}",
    "@keyframes enter{from{opacity:0}}",
    "@media(max-width:700px){main{width:calc(100% - 48px);max-width:440px;padding:32px 0}}",
    "@media(prefers-reduced-motion:reduce){main{animation:none}.button,.quiet{transition:none}}",
);

/// 示例插件自己的一页：眉标“示例 IM”、标题、说明，下面可以放按钮（`actions` 是现成的 HTML）。
fn sample_page(title: &str, lead: &str, actions: &str) -> String {
    format!(
        "<!doctype html><html lang=\"zh-CN\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
         <meta name=\"color-scheme\" content=\"dark light\"><title>{title}</title><style>{SAMPLE_PAGE_STYLE}</style></head>\
         <body><div class=\"screen bare\"><div class=\"stage\"><main><div class=\"eyebrow\">示例 IM</div><h1>{title}</h1>\
         <p class=\"lead\">{lead}</p>{actions}</main></div></div></body></html>"
    )
}

/// 地址查询串里的各项（百分号解码）。
fn query_of(url: &str) -> BTreeMap<String, String> {
    url.split_once('?')
        .map(|(_, q)| q)
        .unwrap_or("")
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .map(|(k, v)| (decode(k), decode(v)))
        .collect()
}

fn encode(text: &str) -> String {
    text.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(b) =
                u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""), 16)
        {
            out.push(b);
            i += 3;
            continue;
        }
        out.push(if bytes[i] == b'+' { b' ' } else { bytes[i] });
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// 协议的错误对象。
fn error(kind: &str, message: &str) -> RpcError {
    let code = match kind {
        "permissionDenied" => 1002,
        "notFound" => 1005,
        "invalidRequest" => 1006,
        _ => 1001,
    };
    serde_json::from_value(json!({ "code": code, "message": message, "data": { "kind": kind, "userMessage": message } }))
        .expect("错误对象")
}

fn send(out: &Out, msg: RpcMessage) {
    if let Ok(line) = codec::encode_line(&msg) {
        let mut guard = out.lock().unwrap();
        let _ = guard.write_all(line.as_bytes());
        let _ = guard.flush();
    }
}

fn notify(out: &Out, method: &str, params: Value) {
    send(out, RpcMessage::notification(method, params));
}

/// 发一个要宿主回的请求（回复不看）。
fn request(out: &Out, next_id: &Arc<AtomicI64>, method: &str, params: Value) {
    let id = next_id.fetch_add(1, Ordering::SeqCst);
    send(out, RpcMessage::request(RpcId::Number(id), method, params));
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

/// 最小 base64 编码（标准字母表），不引依赖。
fn base64_encode(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            T[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            T[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}
