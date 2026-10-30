//! HHH-IM-Protocol：宿主（HHH）与外部 IM 插件之间的线上类型、JSON-RPC 编解码与传输。
//!
//! 规范正本：`doc/product-contracts/hhh-im/provider-protocol/protocol-v1.md`。
//!
//! 这些线上类型**独立于 hhh-im 的内部类型**定义、独立定版、独立冻结：hhh-im 的内部模型
//! 怎么改，都不破坏已发布的插件；两边之间的翻译由 hhh-im 的 provider 适配器负责。
//!
//! 分层：
//! - [`rpc`]：JSON-RPC 2.0 信封与一行一条的编解码。
//! - [`types`]：账号、会话、人员、消息、内容块、资源、表情等领域类型。
//! - [`platform`]：平台说明、能力清单、设置项、插件清单。
//! - [`method`]：方法名常量，以及每个方法的参数/结果类型。
//! - [`login_window`]：“登录窗口”一步的起始网址、要截的跳转开头与截取判断（宿主、自检工具与插件共用）。
//! - [`release`]：安装包的目标、更新来源、版本号与版本索引（发布与检查更新）。
//! - [`transport`]（feature `transport`）：子进程 stdio 传输与 [`transport::ProviderTransport`] trait。
//! - [`package`]（feature `package`）：安装包 `.hhhim` 的打包、读清单与解包。

#![forbid(unsafe_code)]

pub mod attribution;
pub mod contact_requests;
pub mod contacts;
pub mod content;
pub mod forwarded;
pub mod link_card;
pub mod login_window;
pub mod method;
pub mod platform;
pub mod reading;
pub mod release;
pub mod resources;
pub use resources::{ResourceVariant, ResourceVariants};
pub mod rpc;
mod stickers;
pub use stickers::*;
pub mod types;

#[cfg(feature = "transport")]
pub mod transport;

#[cfg(feature = "package")]
pub mod package;

#[cfg(feature = "runtime")]
pub mod runtime;

pub use attribution::AgentAttribution;
pub use method::Method;
pub use platform::{Capabilities, Manifest, PlatformDesc, SettingDecl};
pub use rpc::{ErrorKind, RpcError, RpcErrorData, RpcId, RpcMessage, codec};
pub use types::{
    Account, Block, Conversation, ConversationKind, Event, Message, Outgoing, Person, Reaction,
    Resource, ResourceKind, SendOutcome, SendTarget,
};

/// 本库实现的协议主版本号。只有不兼容的改动才升主版本。
pub const PROTOCOL_VERSION: u32 = 1;

/// 本库（IM SDK）的版本：编进宿主与插件程序。
///
/// 插件清单与版本索引的 `minHost` 写的是插件需要的最低 IM SDK 版本，宿主拿自己的这个版本跟它比
/// （见 [`release::Version::sdk_floor`]）。这个版本号只在发布 SDK 时升。
///
/// # Examples
///
/// ```
/// use hhh_im_protocol::release::Version;
/// assert!(Version::parse(hhh_im_protocol::SDK_VERSION).is_some());
/// ```
pub const SDK_VERSION: &str = env!("CARGO_PKG_VERSION");

/// 清单文件固定文件名。
pub const MANIFEST_FILE: &str = "hhh-im-provider.json";

/// 宿主启动插件时设置的环境变量（值固定为 `"1"`）。
pub const ENV_FLAG: &str = "HHH_IM_PROTOCOL";

/// 宿主强制执行的预算（protocol-v1 §14）。超出的内容宿主按协议错误处理，不写进本地库。
pub mod budget {
    /// 一次阅读事实查询的消息上限。
    pub const READ_MESSAGES_MAX: usize = 100;
    /// 单页阅读名单的人员上限。
    pub const READ_PEOPLE_MAX: usize = 100;
    /// 平台精确阅读位置的十进制字符上限。
    pub const READ_POSITION_MAX_BYTES: usize = 64;
    /// 阅读名单不透明游标的字节上限。
    pub const READ_CURSOR_MAX_BYTES: usize = 4096;
    /// 伙伴署名名字的 UTF-8 字节预算。
    pub const AGENT_NAME_MAX_BYTES: usize = 512;
    /// 伙伴静态缩略头像的完整 data URL 预算；不随每条消息携带大图。
    pub const AGENT_AVATAR_MAX_BYTES: usize = 24 * 1024;
    /// 单条 JSON-RPC 消息最大字节数。
    pub const MESSAGE_MAX_BYTES: usize = 4 * 1024 * 1024;
    /// 每批推送事件最多几个。
    pub const EVENTS_PER_PUSH_MAX: usize = 500;
    /// 每条消息的内容块最多几个。
    pub const BLOCKS_PER_MESSAGE_MAX: usize = 200;
    /// 转发记录最多嵌套几层。
    pub const FORWARDED_DEPTH_MAX: usize = 8;
    /// 一份转发记录当前可保存的消息数上限。
    pub const FORWARDED_MESSAGES_PER_RECORD_MAX: usize = 200;
    /// 一条外层消息里全部转发记录合计的快照数上限。
    pub const FORWARDED_MESSAGES_MAX: usize = 500;
    /// 一条外层消息里转发快照合计的内容块数上限。
    pub const FORWARDED_BLOCKS_MAX: usize = 2000;
    /// 一条外层消息里转发快照的 JSON 字节预算，给普通内容与信封保留余量。
    pub const FORWARDED_BYTES_MAX: usize = 1024 * 1024;
    /// 单块文字最大字节数。
    pub const TEXT_BLOCK_MAX_BYTES: usize = 64 * 1024;
    /// 编号最大字节数（不含控制字符）。
    pub const ID_MAX_BYTES: usize = 256;
    /// 资源一段最大字节数。
    pub const RESOURCE_SEGMENT_MAX_BYTES: u64 = 1024 * 1024;
    /// 登录二维码图片最大字节数。
    pub const QR_IMAGE_MAX_BYTES: usize = 512 * 1024;
    /// 登录步骤里插件声明的提示最多几个字（按字符数，契约 provider-protocol v8）；超出的提示不显示。
    pub const LOGIN_NOTE_MAX_CHARS: usize = 200;
    /// logo 最大字节数。
    pub const LOGO_MAX_BYTES: usize = 256 * 1024;
    /// `account/avatar` 交给插件的头像最大字节数（契约 provider-protocol v35）。
    pub const HOST_AVATAR_MAX_BYTES: usize = 256 * 1024;
    /// `account/avatar` 交给插件的头像最大边长（像素）。
    pub const HOST_AVATAR_MAX_SIDE: u32 = 256;
    /// 凭据最大字节数。
    pub const CREDENTIAL_MAX_BYTES: usize = 64 * 1024;
    /// 单条消息的 `raw` 原始数据最大字节数。
    pub const RAW_MAX_BYTES: usize = 256 * 1024;
    /// 一次 `messages/retranslate` 最多几条。
    pub const RETRANSLATE_BATCH_MAX: usize = 100;
    /// 一次已存消息维护最多几条。
    pub const MAINTENANCE_BATCH_MAX: usize = 100;
    /// 一次维护请求的消息 JSON 预算，给信封与返回的修正留出空间。
    pub const MAINTENANCE_BYTES_MAX: usize = MESSAGE_MAX_BYTES / 2;
    /// 每种消息维护规则最多几条。
    pub const MAINTENANCE_RULES_MAX: usize = 64;
    /// 日志行、`log/write` 的 message 截断到多少字节。
    pub const LOG_LINE_MAX_BYTES: usize = 4 * 1024;
    /// 一个插件最多声明几种登录方式（契约 provider-protocol v12 缺口 6）。
    pub const LOGIN_METHODS_MAX: usize = 8;
    /// 登录方式里每段文字（叫法、说明、灰字、链接文字）最多几个字（按字符数）。
    pub const LOGIN_TEXT_MAX_CHARS: usize = 200;
    /// 登录时填的每个值、回调地址最多多少字节。
    pub const LOGIN_VALUE_MAX_BYTES: usize = 8 * 1024;
    /// `session/notice` 一次最多几条问题（多出的不显示）。
    pub const NOTICES_MAX: usize = 20;
    /// 一条问题最多几个字（按字符数）；超出的那条不显示。
    pub const NOTICE_MAX_CHARS: usize = 200;
    /// `conversations/recentActivity` 一次最多几项（多出的不用）。
    pub const RECENT_ACTIVITY_ITEMS_MAX: usize = 1000;
    /// `conversations/recentActivity` 一次结果里所有 `firstPage` 合起来最多多少字节（按 JSON 计，v43）：
    /// 给其余各项与信封留出一半，整条结果不超过 [`MESSAGE_MAX_BYTES`]。
    pub const RECENT_ACTIVITY_PAGES_BYTES_MAX: usize = MESSAGE_MAX_BYTES / 2;
    /// 最多声明几条重译规则。
    pub const RETRANSLATE_RULES_MAX: usize = 32;
    /// 平台说明里的正则（表情写法）最长多少字节。
    pub const PATTERN_MAX_BYTES: usize = 1024;
    /// 表情写进文字的写法（`emoji.textTemplate`，v31）去掉占位后最多几个字符。
    pub const TEXT_TEMPLATE_AFFIX_MAX_CHARS: usize = 16;
    /// 一个插件最多声明几项操作（契约 provider-protocol v13）；多出的不登记。
    pub const ACTIONS_MAX: usize = 16;
    /// 操作、提醒按钮的文字最多几个字（按字符数）。
    pub const ACTION_LABEL_MAX_CHARS: usize = 20;
    /// 出示文字一步的正文最多几个字（按字符数）。
    pub const STEP_TEXT_MAX_CHARS: usize = 4096;
    /// 列表一步最多几行。
    pub const STEP_ROWS_MAX: usize = 200;
    /// 一步里其余每段文字（说明、标题、细节、按钮文字、小标题与说明）最多几个字（按字符数）。
    pub const STEP_PIECE_MAX_CHARS: usize = 200;
    /// 一步最多几个“另一种做法”。
    pub const STEP_LINKS_MAX: usize = 8;
    /// 列表一行最多几颗按钮。
    pub const ROW_ACTIONS_MAX: usize = 4;
    /// “登录窗口”一步最多几个要截的跳转开头（契约 provider-protocol v36）。
    pub const LOGIN_WINDOW_CAPTURES_MAX: usize = 4;
    /// 一个要截的跳转开头最多几个字（按字符数）。
    pub const LOGIN_WINDOW_CAPTURE_MAX_CHARS: usize = 200;
    /// 确认一步最多几段带小标题的说明。
    pub const STEP_SECTIONS_MAX: usize = 16;
    /// 一次 `people/get` 问几个（`ids` 与 `bots` 合计）：程序没声明 `people.batchMax` 时（v15）。
    pub const PEOPLE_GET_DEFAULT: usize = 100;
    /// `people.batchMax` 的上限（v15）。
    pub const PEOPLE_GET_MAX: usize = 2000;
    /// 一条 `perf/report` 最多多少字节（契约 provider-protocol v32）：每秒一条，宿主内存与解析都可控。
    pub const PERF_REPORT_BYTES_MAX: usize = 64 * 1024;
    /// 一条 `perf/report` 最多几个区间：只报最慢与超过阈值的，其余进分布。
    pub const PERF_SPANS_MAX: usize = 256;
    /// 一条 `perf/report` 的计数与分布合计最多几个序列。
    pub const PERF_SERIES_MAX: usize = 128;
    /// `perf/report` 里的名字最多几个字（按字符数）：名字是方法名或固定标签，不是原文。
    pub const PERF_NAME_CHARS_MAX: usize = 128;
    /// `perf/start` 的报告间隔至少多少毫秒：防止插件刷屏。
    pub const PERF_INTERVAL_MIN_MS: u64 = 500;
}
