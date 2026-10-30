//! 方法名，以及每个方法的参数/结果类型（protocol-v1 §4、§6、§8、§9、§10、§11）。
//!
//! 命名约定：宿主→插件的请求与插件→宿主的请求都在这里；通知（`login/step`、`status`、`log/write`、`$/cancel`）
//! 用各自的参数类型，没有结果。

use serde::{Deserialize, Serialize};

use crate::platform::{Capabilities, PlatformDesc, SettingDecl};
use crate::rpc::ReauthReason;
use crate::types::{
    Account, Conversation, ConversationKind, Emoji, EmojiCategory, EmojiTone, Event, Message,
    Outgoing, Person, Raw, ResourceKind, SendOutcome, SendTarget,
};

/// 协议里的方法名。字符串形式见 [`Method::as_str`]。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// 批量查询他人阅读摘要。
    ReadingReceipts,
    /// 点击后分页查询阅读名单。
    ReadingPeople,
    /// 只读查询本账号的阅读状态。
    ReadingSync,
    /// 显式写回本人阅读边界。
    ReadingMark,
    /// 协商后发送的本账号阅读通知。
    ReadingChanged,

    /// 分页读取已有联系人对应的私聊资料（参数沿用 `ConversationsListParams`，结果 `Page<Conversation>`）。
    ContactsList,
    /// Read the authoritative relationship for a group member.
    ContactsProfile,
    /// Explicitly add a group member as a contact.
    ContactsAdd,
    /// Query, send or explicitly resolve a friend request.
    ContactsRequests,
    // —— 生命周期（宿主→插件）——
    /// 协商版本，取回平台说明与能力。
    Initialize,
    /// 恢复账号。
    SessionOpen,
    /// 注销账号。
    SessionLogout,
    /// 关闭。
    Shutdown,
    /// 心跳。
    Ping,
    // —— 登录 ——
    /// 开始登录（宿主→插件）。
    LoginStart,
    /// 提交表单（宿主→插件）。
    LoginSubmit,
    /// 取消登录（宿主→插件）。
    LoginCancel,
    /// 登录步骤（插件→宿主通知）。
    LoginStep,
    /// 登录成功（插件→宿主通知）。
    LoginFinished,
    /// 登录失败（插件→宿主通知）。
    LoginFailed,
    /// 把本机回调交给插件（宿主→插件，契约 provider-protocol v12 缺口 6）。
    LoginCallback,
    // —— 插件操作（契约 provider-protocol v13）——
    /// 开始一个插件声明的操作（宿主→插件）。
    ActionsStart,
    /// 操作的一步（插件→宿主通知）。
    ActionStep,
    /// 提交操作里的表单（宿主→插件通知）。
    ActionSubmit,
    /// 取消操作（宿主→插件通知）。
    ActionCancel,
    /// 操作做完（插件→宿主通知）。
    ActionFinished,
    /// 操作没做成（插件→宿主通知）。
    ActionFailed,
    /// 用户在一步里做了选择（宿主→插件通知；登录与操作共用）。
    StepChoose,
    // —— 宿主公开资料（插件→宿主）——
    /// 请求当前用户已保存的显示名；不提供令牌或其他资料。
    HostUserName,
    // —— 凭据与授权（插件→宿主）——
    /// 保存凭据（插件→宿主）。
    CredentialSave,
    /// 每条平台读取发出前申请额度（插件→宿主）。
    RequestsAcquire,
    /// 插件确认收到读取许可（通知，握手后的逐请求可选回执）。
    RequestsReceived,
    /// 授权失效（插件→宿主通知）。
    SessionExpired,
    /// 这个连接现在有的问题（插件→宿主通知，契约 provider-protocol v12 缺口 5）。
    SessionNotice,
    /// 用存着的凭据续期（宿主→插件，契约 provider-protocol v12 缺口 7）。
    SessionRenew,
    /// 交本机用户头像（宿主→插件，契约 provider-protocol v35）。
    AccountAvatar,
    // —— 读取（宿主→插件）——
    /// 列会话。
    ConversationsList,
    /// 取会话。
    ConversationsGet,
    /// 搜会话。
    ConversationsSearch,
    /// 列群成员。
    ConversationsMembers,
    /// 定位已有私聊。
    ConversationsFindDirect,
    /// 一段时间里有人发过消息的会话（会话发现，契约 provider-protocol v12 缺口 1）。
    ConversationsRecentActivity,
    /// 批量读人。
    PeopleGet,
    /// 搜人。
    PeopleSearch,
    /// 列消息。
    MessagesList,
    /// 取单条消息。
    MessagesGet,
    /// 读表情。
    ReactionsList,
    /// 表情目录。
    EmojiCatalog,
    /// `stickers/packs`：贴纸目录、收藏与图片。
    StickersPacks,
    /// `stickers/list`：贴纸目录、收藏与图片。
    StickersList,
    /// `stickers/favorites`：贴纸目录、收藏与图片。
    StickersFavorites,
    /// `stickers/favorite`：贴纸目录、收藏与图片。
    StickersFavorite,
    /// `stickers/image`：贴纸目录、收藏与图片。
    StickersImage,

    // —— 资源（宿主→插件）——
    /// 读消息资源一段。
    ResourcesRead,
    /// 描述消息资源实际可用的预览与原件。
    ResourcesDescribe,
    /// 读头像一段。
    AvatarsRead,
    /// 读表情图一段。
    EmojiImage,
    // —— 上传（宿主→插件）——
    /// 开始上传。
    UploadsBegin,
    /// 传一段。
    UploadsChunk,
    /// 完成上传。
    UploadsFinish,
    /// 放弃上传。
    UploadsAbort,
    // —— 动作（宿主→插件）——
    /// 发送。
    MessagesSend,
    /// 按操作编号查发送结果。
    MessagesLookupSend,
    /// 撤回。
    MessagesRecall,
    /// 编辑。
    MessagesEdit,
    /// 重新翻译。
    MessagesRetranslate,
    /// 维护已存的统一消息，不联网。
    MessagesMaintain,
    /// 点表情。
    ReactionsAdd,
    /// 取消表情。
    ReactionsRemove,
    // —— 推送与状态（插件→宿主）——
    /// 推送事件。
    EventsPush,
    /// 连接状态（通知）。
    Status,
    /// 结构化日志（通知）。
    LogWrite,
    // —— 性能自报（契约 provider-protocol v32；只对声明了 `perf` 的插件）——
    /// 剖析开启：开始按间隔自报（宿主→插件请求）。
    PerfStart,
    /// 剖析关闭：停止自报（宿主→插件请求）。
    PerfStop,
    /// 一个间隔的自报（插件→宿主通知）。
    PerfReport,
    // —— 取消（宿主→插件通知）——
    /// 取消请求。
    Cancel,
}

impl Method {
    /// 线上方法名字符串。
    pub fn as_str(self) -> &'static str {
        match self {
            Method::ReadingReceipts => "reading/receipts",
            Method::ReadingPeople => "reading/people",
            Method::ReadingSync => "reading/sync",
            Method::ReadingMark => "reading/mark",
            Method::ReadingChanged => "reading/changed",
            Method::Initialize => "initialize",
            Method::SessionOpen => "session/open",
            Method::SessionLogout => "session/logout",
            Method::Shutdown => "shutdown",
            Method::Ping => "ping",
            Method::LoginStart => "login/start",
            Method::LoginSubmit => "login/submit",
            Method::LoginCancel => "login/cancel",
            Method::LoginStep => "login/step",
            Method::LoginFinished => "login/finished",
            Method::LoginFailed => "login/failed",
            Method::LoginCallback => "login/callback",
            Method::ActionsStart => "actions/start",
            Method::ActionStep => "action/step",
            Method::ActionSubmit => "action/submit",
            Method::ActionCancel => "action/cancel",
            Method::ActionFinished => "action/finished",
            Method::ActionFailed => "action/failed",
            Method::StepChoose => "step/choose",
            Method::HostUserName => "host/userName",
            Method::CredentialSave => "credential/save",
            Method::RequestsAcquire => "requests/acquire",
            Method::RequestsReceived => "requests/received",
            Method::SessionExpired => "session/expired",
            Method::SessionNotice => "session/notice",
            Method::SessionRenew => "session/renew",
            Method::AccountAvatar => "account/avatar",
            Method::ConversationsList => "conversations/list",
            Method::ContactsList => "contacts/list",
            Method::ContactsProfile => "contacts/profile",
            Method::ContactsAdd => "contacts/add",
            Method::ContactsRequests => "contacts/requests",
            Method::ConversationsGet => "conversations/get",
            Method::ConversationsSearch => "conversations/search",
            Method::ConversationsMembers => "conversations/members",
            Method::ConversationsFindDirect => "conversations/findDirect",
            Method::ConversationsRecentActivity => "conversations/recentActivity",
            Method::PeopleGet => "people/get",
            Method::PeopleSearch => "people/search",
            Method::MessagesList => "messages/list",
            Method::MessagesGet => "messages/get",
            Method::ReactionsList => "reactions/list",
            Method::EmojiCatalog => "emoji/catalog",
            Method::StickersPacks => "stickers/packs",
            Method::StickersList => "stickers/list",
            Method::StickersFavorites => "stickers/favorites",
            Method::StickersFavorite => "stickers/favorite",
            Method::StickersImage => "stickers/image",

            Method::ResourcesRead => "resources/read",
            Method::ResourcesDescribe => "resources/describe",
            Method::AvatarsRead => "avatars/read",
            Method::EmojiImage => "emoji/image",
            Method::UploadsBegin => "uploads/begin",
            Method::UploadsChunk => "uploads/chunk",
            Method::UploadsFinish => "uploads/finish",
            Method::UploadsAbort => "uploads/abort",
            Method::MessagesSend => "messages/send",
            Method::MessagesLookupSend => "messages/lookupSend",
            Method::MessagesRecall => "messages/recall",
            Method::MessagesEdit => "messages/edit",
            Method::MessagesRetranslate => "messages/retranslate",
            Method::MessagesMaintain => "messages/maintain",
            Method::ReactionsAdd => "reactions/add",
            Method::ReactionsRemove => "reactions/remove",
            Method::EventsPush => "events/push",
            Method::Status => "status",
            Method::LogWrite => "log/write",
            Method::PerfStart => "perf/start",
            Method::PerfStop => "perf/stop",
            Method::PerfReport => "perf/report",
            Method::Cancel => "$/cancel",
        }
    }

    /// 从字符串认方法名。
    ///
    /// # Examples
    ///
    /// ```
    /// use hhh_im_protocol::Method;
    /// assert_eq!(Method::from_name("conversations/list"), Some(Method::ConversationsList));
    /// assert_eq!(Method::from_name("nope"), None);
    /// assert_eq!(Method::ConversationsList.as_str(), "conversations/list");
    /// ```
    pub fn from_name(name: &str) -> Option<Method> {
        // 全集逐一比对；集合小、调用不频繁，直观优先。
        const ALL: &[Method] = &[
            Method::ReadingReceipts,
            Method::ReadingPeople,
            Method::ReadingSync,
            Method::ReadingMark,
            Method::ReadingChanged,
            Method::Initialize,
            Method::SessionOpen,
            Method::SessionLogout,
            Method::Shutdown,
            Method::Ping,
            Method::LoginStart,
            Method::LoginSubmit,
            Method::LoginCancel,
            Method::LoginStep,
            Method::LoginFinished,
            Method::LoginFailed,
            Method::LoginCallback,
            Method::ActionsStart,
            Method::ActionStep,
            Method::ActionSubmit,
            Method::ActionCancel,
            Method::ActionFinished,
            Method::ActionFailed,
            Method::StepChoose,
            Method::HostUserName,
            Method::CredentialSave,
            Method::RequestsAcquire,
            Method::RequestsReceived,
            Method::SessionExpired,
            Method::SessionNotice,
            Method::SessionRenew,
            Method::AccountAvatar,
            Method::ConversationsList,
            Method::ContactsList,
            Method::ContactsProfile,
            Method::ContactsAdd,
            Method::ContactsRequests,
            Method::ConversationsGet,
            Method::ConversationsSearch,
            Method::ConversationsMembers,
            Method::ConversationsFindDirect,
            Method::ConversationsRecentActivity,
            Method::PeopleGet,
            Method::PeopleSearch,
            Method::MessagesList,
            Method::MessagesGet,
            Method::ReactionsList,
            Method::EmojiCatalog,
            Method::StickersPacks,
            Method::StickersList,
            Method::StickersFavorites,
            Method::StickersFavorite,
            Method::StickersImage,
            Method::ResourcesRead,
            Method::ResourcesDescribe,
            Method::AvatarsRead,
            Method::EmojiImage,
            Method::UploadsBegin,
            Method::UploadsChunk,
            Method::UploadsFinish,
            Method::UploadsAbort,
            Method::MessagesSend,
            Method::MessagesLookupSend,
            Method::MessagesRecall,
            Method::MessagesEdit,
            Method::MessagesRetranslate,
            Method::MessagesMaintain,
            Method::ReactionsAdd,
            Method::ReactionsRemove,
            Method::EventsPush,
            Method::Status,
            Method::LogWrite,
            Method::PerfStart,
            Method::PerfStop,
            Method::PerfReport,
            Method::Cancel,
        ];
        ALL.iter().copied().find(|m| m.as_str() == name)
    }
}

// ============================ 生命周期 ============================

/// `initialize` 参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
    /// 宿主能接收 reading/changed；省略时插件不得发这个通知。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub reading_events: bool,
    /// 宿主支持的协议主版本。
    pub protocol_versions: Vec<u32>,
    /// 宿主信息。
    pub host: HostInfo,
    /// 界面语言（如 zh-CN）。
    pub locale: String,
    /// 宿主给的临时目录（可随时被清空）。
    pub temp_dir: String,
    /// 网络设置。
    pub network: NetworkConfig,
}

/// 宿主信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostInfo {
    /// 宿主名。
    pub name: String,
    /// 宿主版本。
    pub version: String,
}

/// `host/userName` 的结果。无当前名字时为空对象；旧宿主也可能回复空对象或方法不存在。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostUserNameResult {
    /// 当前用户自己的已保存显示名；不是 IM 账号名或操作系统用户名。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// 网络设置。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// 出站代理；没有为空。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy: Option<String>,
}

/// `initialize` 结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    /// 选定的协议主版本。
    pub protocol_version: u32,
    /// 插件信息。
    pub provider: ProviderInfo,
    /// 平台说明。
    pub platform: PlatformDesc,
    /// 能力清单。
    pub capabilities: Capabilities,
    /// 每条平台读取都支持先向宿主申请额度（`requests/acquire`）；宿主在读取参数的 `_meta.readPermit` 中给出凭据。
    /// 默认关闭；旧插件继续用结果里的 `platformRequests` 结算。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub read_request_gating: bool,
    /// 程序设置项。
    #[serde(default)]
    pub settings: Vec<SettingDecl>,
    /// 支持的登录方式，按显示顺序（契约 provider-protocol v12 缺口 6）；不写就是只有一种、点登录直接走插件推来的步骤。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub login_methods: Vec<LoginMethod>,
    /// 插件的操作，按显示顺序（契约 provider-protocol v13）：账号级的进连接卡的“管理”菜单，会话级的进会话菜单；
    /// 不写就没有。至多 [`crate::budget::ACTIONS_MAX`] 项。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<ActionDecl>,
}

/// 插件声明的一项操作（`initialize.actions`，契约 provider-protocol v13）。
///
/// # Examples
///
/// ```
/// use hhh_im_protocol::method::{ActionDecl, ActionScope};
/// use hhh_im_protocol::ConversationKind;
/// let decl: ActionDecl = serde_json::from_value(serde_json::json!(
///     { "id": "members", "label": "群成员", "scope": "conversation", "conversationKinds": ["group"] })).unwrap();
/// assert_eq!(decl.scope, ActionScope::Conversation);
/// assert!(decl.applies_to(ConversationKind::Group) && !decl.applies_to(ConversationKind::Direct));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionDecl {
    /// 操作编号（插件内不重复；用户点了宿主照它叫 `actions/start`）。
    pub id: String,
    /// 菜单上的字（至多 [`crate::budget::ACTION_LABEL_MAX_CHARS`] 个字）。
    pub label: String,
    /// 放在哪个菜单。
    pub scope: ActionScope,
    /// 在会话列表的“＋”中显示；只适用于账号级操作，未声明时为假。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub shortcut: bool,
    /// 会话级操作只在这几种会话上有；不写就是每种会话都有。账号级操作不看它。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conversation_kinds: Vec<ConversationKind>,
}

impl ActionDecl {
    /// 会话级操作适不适用于这种会话（没限定就都适用）。
    pub fn applies_to(&self, kind: ConversationKind) -> bool {
        self.scope == ActionScope::Conversation
            && (self.conversation_kinds.is_empty() || self.conversation_kinds.contains(&kind))
    }
}

/// 操作放在哪个菜单。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ActionScope {
    /// 连接卡的“管理”菜单。
    Account,
    /// 会话菜单。
    Conversation,
    /// 宿主不认识的位置：这一项不登记。
    #[serde(other)]
    Unknown,
}

/// 插件信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    /// 程序编号（同清单 id）。
    pub id: String,
    /// 平台名。
    pub name: String,
    /// 程序版本。
    pub version: String,
}

/// `session/open` 参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionOpenParams {
    /// 宿主保存的不透明凭据；没有为空。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credential: Option<String>,
    /// 程序设置项的值。
    #[serde(default)]
    pub settings: std::collections::BTreeMap<String, String>,
    /// 上次保存的同步游标；没有为空。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sync_state: Option<String>,
    /// 这份凭据是借来的、只能用（隔离测试实例借用户的，契约 provider-protocol v12 缺口 7）：插件不得换新凭据、
    /// 不得调 `credential/save`，到期就发 `session/expired`。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub credential_read_only: bool,
    /// 同一编号的内置平台留下的授权与设置，交给插件一次（契约 provider-protocol v12 缺口 14，格式见规范附录 D）；
    /// 只在插件还没有自己的凭据时给。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legacy: Option<LegacyHandover>,
}

/// 内置平台交给插件的授权与设置（`session/open.legacy`）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyHandover {
    /// 格式名；本版只有 [`LEGACY_FORMAT_BUILTIN_V1`]。
    pub format: String,
    /// 内置授权（格式里定义的 JSON 文字）。
    pub credential: String,
    /// 内置平台的连接设置（格式里定义的 JSON 对象）。
    #[serde(default)]
    pub settings: serde_json::Value,
}

/// 内置飞书、Slack 交接用的格式名（规范附录 D）。
pub const LEGACY_FORMAT_BUILTIN_V1: &str = "hhh-builtin-v1";

/// 插件请宿主写进自身设置的交接结果（`session/open` 在线结果的 `adoptedSettings`）。
/// 普通打开也可从当前有效凭据返回同一结果，帮助宿主补完被中断的交接；借用凭据时不返回。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoptedSettings {
    /// 写进插件参数配置的值（键是插件声明的设置项编号）。
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub parameters: std::collections::BTreeMap<String, String>,
    /// 写成“现在在用”的登录方式与填的内容（键见 [`login_value`]）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub login: Option<AdoptedLogin>,
}

/// 交接后在用的登录方式。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoptedLogin {
    /// 登录方式编号（插件 `loginMethods` 里的 `id`）。
    pub method: String,
    /// 填的内容。
    #[serde(default)]
    pub values: std::collections::BTreeMap<String, String>,
}

/// `session/open` 结果（protocol-v1 §4.2）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SessionOpenResult {
    /// 已在线。
    Online {
        /// 已核验账号。
        account: Account,
        /// 交接或恢复未完成交接时，请宿主保存的插件设置；没有为空。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        adopted_settings: Option<AdoptedSettings>,
    },
    /// 需要登录。
    NeedsLogin {
        /// 原因。
        reason: NeedsLoginReason,
        /// 给用户看的说明。
        message: String,
    },
}

/// 需要登录的原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NeedsLoginReason {
    /// 没有凭据。
    NoCredential,
    /// 凭据过期。
    Expired,
    /// 平台撤销。
    Revoked,
    /// 凭据无效。
    CredentialInvalid,
}

// ============================ 登录 ============================

/// 登录方式的类型：由 IM 层定，各 IM 声明支持哪几种（契约 provider-protocol v12 缺口 6）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LoginKind {
    /// 经公网授权中继：登录前填中继地址（[`login_value::RELAY_URL`]）。
    Relay,
    /// 用户自己的应用：登录前填 App ID 与 App Secret（[`login_value::APP_ID`]、[`login_value::APP_SECRET`]）。
    OwnApp,
    /// 直接登录：不用填，照插件推来的步骤走。
    Direct,
    /// 用宿主账号登录（契约 provider-protocol v13）：不用填，宿主在 `login/start` 里给当前宿主账号的令牌
    /// （[`login_value::HOST_TOKEN`]）与账号（[`LoginStartParams::host_account`]）。只有宿主允许的插件（Friend：带官方签名）
    /// 登记这一类；登记了、还没登录也没被用户手动退出过时，宿主在刚装好、启动、切到这个身份时自动开始。
    Host,
    /// 宿主不认识的类型：这一种方式不登记。
    #[serde(other)]
    Unknown,
}

/// `login/start.values` 里各类型用的键。
pub mod login_value {
    /// 中继地址（`relay`）。
    pub const RELAY_URL: &str = "relayUrl";
    /// App ID（`ownApp`）。
    pub const APP_ID: &str = "appId";
    /// App Secret（`ownApp`）；只在用户这次填了时给，没填表示沿用插件凭据里的。
    pub const APP_SECRET: &str = "appSecret";
    /// 宿主账号的访问令牌（`host`，契约 provider-protocol v13）：只在这次 `login/start` 里给一次，宿主不存、不进日志；
    /// 插件要再用就自己存进凭据。
    pub const HOST_TOKEN: &str = "hostToken";
}

/// 用宿主账号登录时宿主现在的账号（`login/start.hostAccount`，契约 provider-protocol v13），给插件在步骤里显示用。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostAccount {
    /// 名字。
    pub name: String,
    /// 一行说明：个人还是组织（如“个人”“组织 · XD”）。
    pub detail: String,
    /// 这个账号的头像地址（https）；没有就不给。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
}

/// 一种登录方式（`initialize.loginMethods`）：类型由 IM 层定，叫法、说明、灰字、链接由 IM 写。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginMethod {
    /// 方式编号（插件内不重复，宿主据此记“现在在用哪种”）。
    pub id: String,
    /// 类型。
    pub kind: LoginKind,
    /// 叫法（页签上的字）。
    pub label: String,
    /// 一句说明。
    #[serde(default)]
    pub note: String,
    /// `relay`：中继地址框的灰字。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    /// `relay`：没保存过中继地址时框里预填的地址（https，或本机测试的回环 http；v14）。不写就空着让用户填。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,
    /// `ownApp`：App ID 的叫法（默认 App ID）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_label: Option<String>,
    /// `ownApp`：App ID 框的灰字。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_placeholder: Option<String>,
    /// `ownApp`：App Secret 的叫法（默认 App Secret）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret_label: Option<String>,
    /// `ownApp`：App Secret 框的灰字。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret_placeholder: Option<String>,
    /// `ownApp`：App Secret 去哪找、存在哪的说明（画在框旁的 ⓘ 里；v14）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret_help: Option<String>,
    /// `ownApp`：要宿主在本机收授权回调时的路径；不写就没有回调。写成相对路径（如 `callback`，v14）时宿主拼在这个 IM
    /// 自己的 `/hhh-friend-im/<平台编号>/` 下面（[`resolve_callback_path`]），插件不必知道宿主怎么给它编号；照 v12 写成
    /// 以 `/` 开头的完整路径时必须已经在这个目录下。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub callback_path: Option<String>,
    /// `ownApp`：回调地址的说明（画在 ⓘ 里）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub callback_help: Option<String>,
    /// 可选的“前往…创建”链接（只收 https）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub create_link: Option<CreateLink>,
}

/// 登录方式里的一个外部链接。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateLink {
    /// 链接文字。
    pub label: String,
    /// 地址（https）。
    pub url: String,
    /// 链接后面接的一句话。
    #[serde(default)]
    pub after: String,
}

/// 回调路径必须在的目录：`/hhh-friend-im/<平台编号>/`。
///
/// # Examples
///
/// ```
/// use hhh_im_protocol::method::callback_path_ok;
/// assert!(callback_path_ok("/hhh-friend-im/ext-sample/callback", "ext-sample"));
/// assert!(!callback_path_ok("/hhh-friend-im/feishu/callback", "ext-sample"));
/// assert!(!callback_path_ok("/hhh-friend-im/ext-sample/", "ext-sample"));
/// assert!(!callback_path_ok("/hhh-friend-im/ext-sample/../feishu/callback", "ext-sample"));
/// ```
pub fn callback_path_ok(path: &str, platform_id: &str) -> bool {
    let prefix = format!("/hhh-friend-im/{platform_id}/");
    let Some(rest) = path.strip_prefix(&prefix) else {
        return false;
    };
    !rest.is_empty()
        && path.len() <= 256
        && rest.split('/').all(|segment| {
            !segment.is_empty()
                && segment != "."
                && segment != ".."
                && segment
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
        })
}

/// 插件声明的回调路径在宿主上的完整路径（契约 provider-protocol v14）：相对路径拼在 `/hhh-friend-im/<平台编号>/` 下面，
/// 以 `/` 开头的照 v12 原样用；不在这个 IM 自己的目录下、或某一段不合规时为空（宿主不登记这种登录方式）。
///
/// # Examples
///
/// ```
/// use hhh_im_protocol::method::resolve_callback_path;
/// assert_eq!(resolve_callback_path("callback", "feishu").as_deref(), Some("/hhh-friend-im/feishu/callback"));
/// assert_eq!(resolve_callback_path("oauth/done", "ext-sample").as_deref(), Some("/hhh-friend-im/ext-sample/oauth/done"));
/// assert_eq!(resolve_callback_path("/hhh-friend-im/ext-sample/callback", "ext-sample").as_deref(), Some("/hhh-friend-im/ext-sample/callback"));
/// assert_eq!(resolve_callback_path("/hhh-friend-im/feishu/callback", "ext-sample"), None);
/// assert_eq!(resolve_callback_path("../feishu/callback", "ext-sample"), None);
/// assert_eq!(resolve_callback_path("", "ext-sample"), None);
/// ```
pub fn resolve_callback_path(path: &str, platform_id: &str) -> Option<String> {
    let full = if path.starts_with('/') {
        path.to_owned()
    } else {
        format!("/hhh-friend-im/{platform_id}/{path}")
    };
    callback_path_ok(&full, platform_id).then_some(full)
}

/// `login/start` 参数（契约 provider-protocol v12 缺口 6）。旧宿主发 `{}`，旧插件忽略多出的字段。
/// 填的内容里有 App Secret、宿主令牌，`Debug` 不打印它们。
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginStartParams {
    /// 这次选的登录方式编号（插件声明了 `loginMethods` 时给）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    /// 这次填的内容（键见 [`login_value`]）。
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub values: std::collections::BTreeMap<String, String>,
    /// 宿主在本机收回调的完整地址（这种方式声明了 `callbackPath` 时给）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub callback_url: Option<String>,
    /// 一次性的 `state`：插件放进授权地址，回调带回来时宿主核对（给了 `callbackUrl` 时给）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    /// 用宿主账号登录（`host`，契约 provider-protocol v13）时宿主现在的账号；令牌在 `values` 的
    /// [`login_value::HOST_TOKEN`]。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_account: Option<HostAccount>,
}

impl std::fmt::Debug for LoginStartParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoginStartParams")
            .field("method", &self.method)
            .field("values", &self.values.keys().collect::<Vec<_>>())
            .field("callback_url", &self.callback_url)
            .field("has_state", &self.state.is_some())
            .field("host_account", &self.host_account)
            .finish()
    }
}

/// `login/callback` 参数（宿主→插件）：宿主在本机收到这次登录的回调、`state` 对上了，把整个地址交给插件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginCallbackParams {
    /// 登录流程编号。
    pub flow_id: String,
    /// 回调的完整地址（含查询串）。
    pub url: String,
}

/// `login/start` 结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginStartResult {
    /// 本次登录流程编号。
    pub flow_id: String,
}

/// `login/step` 通知参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginStepParams {
    /// 登录流程编号。
    pub flow_id: String,
    /// 这一步要显示什么。
    pub step: LoginStep,
}

/// 登录与插件操作共用的一步（protocol-v1 §6.1 步骤表；`showText`、`list`、`confirm` 与 `links` 是契约
/// provider-protocol v13 加的）。
///
/// 用户在一步里的选择——点了“另一种做法”（`links`）、列表行尾的按钮、多选或确认一步的确认、出示文字的确认、
/// 左边按钮代表的选择（`cancelChoice`）——宿主一律发 `step/choose`（[`StepChooseParams`]）。`noCancel` 的一步没有左边的
/// 取消按钮，点遮罩、按 Esc 都不关（不能跳过）。
///
/// # Examples
///
/// ```
/// use hhh_im_protocol::method::{LoginStep, TextLayout};
/// let step: LoginStep = serde_json::from_value(serde_json::json!({
///     "kind": "showText", "caption": "这是你的恢复密钥", "text": "a b c", "layout": "words",
///     "copy": true, "ack": "我已保存", "confirmLabel": "完成", "noCancel": true })).unwrap();
/// let LoginStep::ShowText { layout, no_cancel, .. } = step else { panic!() };
/// assert_eq!(layout, TextLayout::Words);
/// assert!(no_cancel);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum LoginStep {
    /// 二维码。
    QrCode {
        /// PNG 的 base64。
        image: String,
        /// 过期时间（毫秒）。
        expires_at_ms: i64,
        /// 提示。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        hint: Option<String>,
        /// 插件声明的提示（契约 provider-protocol v8）：画在这一步内容下面、按钮上面；不给就没有。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        note: Option<LoginNote>,
        /// 另一种做法（v13）：文字按钮，宿主画在底部一行的左边（v46）。
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        links: Vec<StepLink>,
    },
    /// 打开浏览器。
    OpenUrl {
        /// 地址（https 或本机回环 http）。
        url: String,
        /// 按钮文字。
        label: String,
        /// 插件声明的提示（契约 provider-protocol v8）：画在这一步内容下面、按钮上面；不给就没有。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        note: Option<LoginNote>,
        /// 另一种做法（v13）。
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        links: Vec<StepLink>,
    },
    /// 等待设备确认。
    ConfirmOnDevice {
        /// 提示。
        message: String,
        /// 插件声明的提示（契约 provider-protocol v8）：画在这一步内容下面、按钮上面；不给就没有。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        note: Option<LoginNote>,
        /// 另一种做法（v13）。
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        links: Vec<StepLink>,
    },
    /// 表单。提交时宿主调 `login/submit` 或 `action/submit`。
    Input {
        /// 字段。
        fields: Vec<LoginField>,
        /// 插件声明的提示（契约 provider-protocol v8）：画在这一步内容下面、按钮上面；不给就没有。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        note: Option<LoginNote>,
        /// 字段上面的一句说明（v13）。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        caption: Option<String>,
        /// 提交按钮的文字（v13）；不给写“提交”。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        submit_label: Option<String>,
        /// 左边按钮的文字（v13）；不给写“取消”。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cancel_label: Option<String>,
        /// 左边按钮代表的选择（v13，如批准框的“拒绝”）：给了点它发 `step/choose`，不是取消。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cancel_choice: Option<String>,
        /// 没有左边的按钮、不能跳过（v13）。
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        no_cancel: bool,
        /// 另一种做法（v13）。
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        links: Vec<StepLink>,
    },
    /// 一段说明（本身就是提示，不再带 `note`）。
    Message {
        /// 文字。
        text: String,
        /// 色调。
        tone: MessageTone,
        /// 另一种做法（v13）。
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        links: Vec<StepLink>,
    },
    /// 出示一段要保存或要对照的文字（v13）：恢复密钥、新电脑上的几个词、好友链接。
    ShowText {
        /// 文字上面的一句说明。
        caption: String,
        /// 正文（至多 [`crate::budget::STEP_TEXT_MAX_CHARS`] 个字）。
        text: String,
        /// 排法。
        layout: TextLayout,
        /// 有没有“复制”。
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        copy: bool,
        /// “我已保存”这类勾选：给了就要先勾上才能点确认。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ack: Option<String>,
        /// 确认按钮的文字（点了发 `step/choose {choice: "confirm"}`）；不给就没有确认按钮。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        confirm_label: Option<String>,
        /// 左边按钮的文字；不给时有确认写“取消”、没确认写“关闭”。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cancel_label: Option<String>,
        /// 左边按钮代表的选择：给了点它发 `step/choose`，不是取消。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cancel_choice: Option<String>,
        /// 没有左边的按钮、不能跳过。
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        no_cancel: bool,
        /// 给了就在文字下面转圈等，写这句话（如“等待批准…”）。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        waiting: Option<String>,
        /// 插件声明的提示。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        note: Option<LoginNote>,
        /// 另一种做法。
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        links: Vec<StepLink>,
    },
    /// 一列东西（v13）：每行可带按钮（我的电脑、群成员），或多选后确认（选好友）。
    List {
        /// 列表上面的一句说明。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        caption: Option<String>,
        /// 行（至多 [`crate::budget::STEP_ROWS_MAX`] 行）。
        rows: Vec<ListRow>,
        /// 给了就是多选。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        select: Option<ListSelect>,
        /// 一行都没有时写什么。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        empty: Option<String>,
        /// 左边按钮的文字；不给时多选写“取消”、否则写“关闭”。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cancel_label: Option<String>,
        /// 左边按钮代表的选择：给了点它发 `step/choose`，不是取消。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cancel_choice: Option<String>,
        /// 没有左边的按钮、不能跳过。
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        no_cancel: bool,
        /// 插件声明的提示。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        note: Option<LoginNote>,
        /// 另一种做法。
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        links: Vec<StepLink>,
    },
    /// 确认或说明（v13）：可带一个人、几段带小标题的说明；不可撤销的操作用危险色。
    Confirm {
        /// 粗体的一句话（如“移除「办公室电脑」？”）。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        caption: Option<String>,
        /// 要核对的人。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        person: Option<StepPerson>,
        /// 几段带小标题的说明。
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        sections: Vec<StepSection>,
        /// 一段说明。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        text: Option<String>,
        /// 确认按钮的文字（点了发 `step/choose {choice: "confirm"}`）。
        confirm_label: String,
        /// 危险操作：确认按钮用危险色。
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        danger: bool,
        /// 左边按钮的文字；不给写“取消”。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cancel_label: Option<String>,
        /// 左边按钮代表的选择：给了点它发 `step/choose`，不是取消。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cancel_choice: Option<String>,
        /// 没有左边的按钮、不能跳过。
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        no_cancel: bool,
        /// 插件声明的提示。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        note: Option<LoginNote>,
        /// 另一种做法。
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        links: Vec<StepLink>,
    },
    /// 登录窗口（契约 provider-protocol v36）：宿主开一个无痕窗口显示 `url`（平台自己的登录页），页面要跳到 `capture`
    /// 里任一开头时截下，用 `login/callback {flowId, url}` 把完整地址交给插件（每一步至多一次）；窗口里换成宿主的收据页，
    /// 登录有了结局再关（v37）。规则见 [`crate::login_window`]；开不了窗时宿主的登录框写“没能打开”，用户可以再试。
    LoginWindow {
        /// 起始网址（https，或本机回环 http）。
        url: String,
        /// 按钮文字：点了把窗口叫到前面，窗口关了就重新打开。
        label: String,
        /// 要截下的跳转开头，1 到 [`crate::budget::LOGIN_WINDOW_CAPTURES_MAX`] 项。
        capture: Vec<String>,
        /// 插件声明的提示。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        note: Option<LoginNote>,
        /// 另一种做法。
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        links: Vec<StepLink>,
    },
}

/// 一步里“另一种做法”（v13）：文字按钮，宿主画在底部一行的左边（v46），点了宿主发 `step/choose {choice: id}`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepLink {
    /// 选择编号。
    pub id: String,
    /// 按钮文字。
    pub label: String,
}

/// 出示文字的排法（v13）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TextLayout {
    /// 按空白分开的词，逐个编号（恢复密钥）。
    Words,
    /// 一行大字（新电脑上的几个词）。
    Code,
    /// 一行等宽（链接）。
    Line,
    /// 宿主不认识的排法：按一行等宽画。
    #[serde(other)]
    Unknown,
}

/// 列表的一行（v13）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListRow {
    /// 行编号（点了行尾按钮、多选确认时交回）。
    pub id: String,
    /// 标题。
    pub title: String,
    /// 一行细节。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// 标题后的小角标（如“本机”“群主”）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    /// 行首的图标；不给就没有。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<RowIcon>,
    /// 行尾的按钮（点了发 `step/choose {choice: 按钮编号, row: 行编号}`）。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<RowAction>,
}

/// 行首的图标（v13）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RowIcon {
    /// 一个人：圆标里画标题的第一个字。
    Person,
    /// 一台电脑。
    Device,
    /// 宿主不认识的图标：不画。
    #[serde(other)]
    Unknown,
}

/// 行尾的一颗按钮（v13）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RowAction {
    /// 选择编号。
    pub id: String,
    /// 按钮文字。
    pub label: String,
    /// 不可撤销：用危险色。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub danger: bool,
}

/// 列表的多选（v13）：选好后点确认，宿主发 `step/choose {choice: "confirm", selected: [行编号…]}`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListSelect {
    /// 至少选几行（不给按 1）。
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub min: u32,
    /// 至多选几行；不给不限。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<u32>,
    /// 确认按钮的文字。
    pub confirm_label: String,
}

fn one() -> u32 {
    1
}

fn is_one(value: &u32) -> bool {
    *value == 1
}

/// 确认一步里要核对的人（v13）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepPerson {
    /// 名字。
    pub name: String,
    /// 一行细节（如身份指纹）。
    #[serde(default)]
    pub detail: String,
}

/// 确认一步里一段带小标题的说明（v13）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepSection {
    /// 小标题。
    pub title: String,
    /// 正文。
    pub text: String,
}

/// `step/choose` 参数（宿主→插件通知，v13；登录与操作共用，同 `login/submit`）：用户在这一步做了选择。插件之后照常推下一步或结束。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepChooseParams {
    /// 流程编号（登录或操作的）。
    pub flow_id: String,
    /// 选择编号：`confirm`（确认）、`links` 里的编号、行尾按钮的编号或 `cancelChoice`。
    pub choice: String,
    /// 点的是哪一行的按钮。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row: Option<String>,
    /// 多选确认时选中的行。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selected: Vec<String>,
}

/// 一项操作声明哪里不合要求（v13，宿主与自检工具共用）：位置认不出、编号不合编号规则、文字是空的或超过
/// [`ACTION_LABEL_MAX_CHARS`](crate::budget::ACTION_LABEL_MAX_CHARS) 个字。编号重复与项数上限由调用方按整张表核对。
///
/// # Examples
///
/// ```
/// use hhh_im_protocol::method::{action_problem, ActionDecl, ActionScope};
/// let decl = |id: &str, label: &str| ActionDecl {
///     id: id.into(), label: label.into(), scope: ActionScope::Account, shortcut: false, conversation_kinds: vec![] };
/// assert_eq!(action_problem(&decl("devices", "我的电脑")), None);
/// assert!(action_problem(&decl("", "我的电脑")).is_some());
/// assert!(action_problem(&decl("devices", "  ")).is_some());
/// assert!(action_problem(&decl("devices", &"字".repeat(21))).is_some());
/// ```
pub fn action_problem(decl: &ActionDecl) -> Option<&'static str> {
    use crate::budget::ACTION_LABEL_MAX_CHARS;
    if decl.scope == ActionScope::Unknown {
        Some("放在哪个菜单认不出")
    } else if decl.shortcut && decl.scope != ActionScope::Account {
        Some("只有账号级操作能放在会话列表的加号中")
    } else if !crate::platform::Manifest::id_ok(&format!("a{}", decl.id)) || decl.id.is_empty() {
        Some("编号是空的或写法不对")
    } else if decl.label.trim().is_empty() || decl.label.chars().count() > ACTION_LABEL_MAX_CHARS {
        Some("文字是空的或太长")
    } else {
        None
    }
}

/// 一步哪里超出宿主的上限（v13，宿主与自检工具共用）：正文超过
/// [`STEP_TEXT_MAX_CHARS`](crate::budget::STEP_TEXT_MAX_CHARS)、列表超过 [`STEP_ROWS_MAX`](crate::budget::STEP_ROWS_MAX)
/// 行、其余每段文字超过 [`STEP_PIECE_MAX_CHARS`](crate::budget::STEP_PIECE_MAX_CHARS) 个字、按钮与说明段数超限、编号空着
/// 或重复、多选的至少至多不成立。只核对 v13 加的内容（新的三种步骤、`links` 与表单的补充），旧步骤其余部分照旧。
///
/// # Examples
///
/// ```
/// use hhh_im_protocol::method::{step_problem, LoginStep};
/// let step = |v: serde_json::Value| serde_json::from_value::<LoginStep>(v).unwrap();
/// assert_eq!(step_problem(&step(serde_json::json!({ "kind": "list", "rows": [{ "id": "a", "title": "甲" }] }))), None);
/// assert!(step_problem(&step(serde_json::json!({ "kind": "list",
///     "rows": [{ "id": "a", "title": "甲" }, { "id": "a", "title": "乙" }] }))).is_some());
/// assert!(step_problem(&step(serde_json::json!({ "kind": "showText", "caption": "密钥",
///     "text": "x".repeat(4097), "layout": "line" }))).is_some());
/// ```
pub fn step_problem(step: &LoginStep) -> Option<&'static str> {
    use crate::budget::{
        ROW_ACTIONS_MAX, STEP_LINKS_MAX, STEP_PIECE_MAX_CHARS, STEP_ROWS_MAX, STEP_SECTIONS_MAX,
        STEP_TEXT_MAX_CHARS,
    };
    let piece = |text: &str| text.chars().count() <= STEP_PIECE_MAX_CHARS;
    let piece_opt = |text: &Option<String>| text.as_deref().is_none_or(piece);
    let filled = |text: &str| !text.trim().is_empty() && piece(text);
    let links_ok = |links: &[StepLink]| {
        links.len() <= STEP_LINKS_MAX
            && links
                .iter()
                .all(|link| !link.id.is_empty() && piece(&link.id) && filled(&link.label))
    };
    let common_ok =
        |links: &[StepLink], cancel_label: &Option<String>, cancel_choice: &Option<String>| {
            links_ok(links)
                && piece_opt(cancel_label)
                && cancel_choice
                    .as_deref()
                    .is_none_or(|choice| !choice.is_empty() && piece(choice))
        };
    let ok = match step {
        LoginStep::QrCode { links, .. }
        | LoginStep::OpenUrl { links, .. }
        | LoginStep::ConfirmOnDevice { links, .. }
        | LoginStep::Message { links, .. } => links_ok(links),
        LoginStep::Input {
            caption,
            submit_label,
            cancel_label,
            cancel_choice,
            links,
            ..
        } => {
            piece_opt(caption)
                && piece_opt(submit_label)
                && common_ok(links, cancel_label, cancel_choice)
        }
        LoginStep::ShowText {
            caption,
            text,
            ack,
            confirm_label,
            cancel_label,
            cancel_choice,
            waiting,
            links,
            ..
        } => {
            piece(caption)
                && !text.trim().is_empty()
                && text.chars().count() <= STEP_TEXT_MAX_CHARS
                && [ack, confirm_label, waiting].into_iter().all(piece_opt)
                && common_ok(links, cancel_label, cancel_choice)
        }
        LoginStep::List {
            caption,
            rows,
            select,
            empty,
            cancel_label,
            cancel_choice,
            links,
            ..
        } => {
            let mut ids = std::collections::HashSet::new();
            rows.len() <= STEP_ROWS_MAX
                && rows.iter().all(|row| {
                    !row.id.is_empty()
                        && piece(&row.id)
                        && ids.insert(row.id.as_str())
                        && filled(&row.title)
                        && piece_opt(&row.detail)
                        && piece_opt(&row.tag)
                        && row.actions.len() <= ROW_ACTIONS_MAX
                        && row
                            .actions
                            .iter()
                            .all(|a| !a.id.is_empty() && piece(&a.id) && filled(&a.label))
                })
                && select.as_ref().is_none_or(|s| {
                    filled(&s.confirm_label) && s.max.is_none_or(|max| max >= 1 && max >= s.min)
                })
                && piece_opt(caption)
                && piece_opt(empty)
                && common_ok(links, cancel_label, cancel_choice)
        }
        LoginStep::Confirm {
            caption,
            person,
            sections,
            text,
            confirm_label,
            cancel_label,
            cancel_choice,
            links,
            ..
        } => {
            filled(confirm_label)
                && piece_opt(caption)
                && piece_opt(text)
                && person
                    .as_ref()
                    .is_none_or(|p| filled(&p.name) && piece(&p.detail))
                && sections.len() <= STEP_SECTIONS_MAX
                && sections.iter().all(|s| filled(&s.title) && filled(&s.text))
                && common_ok(links, cancel_label, cancel_choice)
        }
        LoginStep::LoginWindow {
            url,
            label,
            capture,
            links,
            ..
        } => {
            if !crate::login_window::start_url_ok(url) {
                return Some("登录窗口的起始网址要是 https，或本机回环地址的 http");
            }
            if !crate::login_window::captures_ok(capture) {
                return Some(
                    "登录窗口要截的跳转开头要有 1 到 4 个，每个是自定义协议的 <协议>:// 或带路径的 https 开头",
                );
            }
            filled(label) && links_ok(links)
        }
    };
    (!ok).then_some("这一步的文字、行或按钮超出上限，或编号空着、重复")
}

/// 登录表单字段。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginField {
    /// 字段编号。
    pub id: String,
    /// 显示名。
    pub label: String,
    /// 密文。
    #[serde(default)]
    pub secret: bool,
    /// 占位符。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    /// 校验正则。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
}

/// 插件在一步登录里声明的提示（契约 provider-protocol v8）。每个插件自己决定在哪一步、说什么，
/// 宿主不认识提示的含义，只按字段画；文字不超过 [`crate::budget::LOGIN_NOTE_MAX_CHARS`] 个字、原样显示。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginNote {
    /// 文字。
    pub text: String,
    /// 色调（同“一段说明”）。
    pub tone: MessageTone,
}

/// 说明色调。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MessageTone {
    /// 普通。
    Info,
    /// 警告。
    Warn,
}

/// `login/submit` 参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginSubmitParams {
    /// 登录流程编号。
    pub flow_id: String,
    /// 表单值。
    pub values: std::collections::BTreeMap<String, String>,
}

/// `login/cancel` 参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginCancelParams {
    /// 登录流程编号。
    pub flow_id: String,
}

/// `login/finished` 通知参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginFinishedParams {
    /// 登录流程编号。
    pub flow_id: String,
    /// 核验账号。
    pub account: Account,
    /// 登录框开着时成功页的标题（v13）；不给写“<名字>已登录”。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// 成功页的一段话（v13）；不给写“开始同步会话。”。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// `login/failed` 通知参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginFailedParams {
    /// 登录流程编号。
    pub flow_id: String,
    /// 给用户看的说明。
    pub message: String,
    /// 能不能重试。
    #[serde(default)]
    pub retryable: bool,
    /// 失败页的标题（v13）；不给写“登录没完成”。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// 能重试时重试按钮的文字（v13）；不给写“重新登录”。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_label: Option<String>,
}

// ============================ 插件操作（契约 provider-protocol v13） ============================

/// `actions/start` 参数（宿主→插件）：用户点了一项操作。插件回 [`ActionsStartResult`]，之后发 `action/step` 一步步走，
/// 以 `action/finished` 或 `action/failed` 结束。新的 `actions/start` 或 `login/start` 取消还没结束的旧流程。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionsStartParams {
    /// 操作编号：`initialize.actions` 里的，或提醒的 `action.id`（不必出现在 `actions` 里）。
    pub action_id: String,
    /// 会话级操作作用在哪个会话（平台会话编号）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
}

/// `actions/start` 结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionsStartResult {
    /// 这次操作的流程编号。
    pub flow_id: String,
    /// 对话框的标题（如提醒上的“查看”点开后写“批准新电脑”）；不给时宿主用操作的名字（会话级的写“操作名 · 会话名”）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// `action/step` 通知参数（插件→宿主）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionStepParams {
    /// 流程编号。
    pub flow_id: String,
    /// 这一步要显示什么（与登录同一套）。
    pub step: LoginStep,
}

/// `action/submit` 参数（宿主→插件通知）：表单一步提交。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionSubmitParams {
    /// 流程编号。
    pub flow_id: String,
    /// 表单值。
    pub values: std::collections::BTreeMap<String, String>,
}

/// `action/cancel` 参数（宿主→插件通知）：用户关掉了对话框。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionCancelParams {
    /// 流程编号。
    pub flow_id: String,
}

/// `action/finished` 通知参数（插件→宿主）。给了 `title` 或 `message` 时对话框换成成功页，都不给就直接关掉。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionFinishedParams {
    /// 流程编号。
    pub flow_id: String,
    /// 成功页的标题。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// 成功页的一段话。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// 做完后把这个会话从列表里拿掉（契约 provider-protocol v19）：只对会话级操作（`actions/start` 带了
    /// `conversationId`）有效，宿主当作取消关注——不删记录、不动平台，有新消息时会话照样回来。账号级操作带了宿主不理会。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub remove_from_list: bool,
    /// 完成后打开这个会话；宿主先向当前账号核对资料，再加入列表。不与移出列表同时使用。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open_conversation_id: Option<String>,
}

/// `action/failed` 通知参数（插件→宿主）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionFailedParams {
    /// 流程编号。
    pub flow_id: String,
    /// 给用户看的说明。
    pub message: String,
    /// 能不能重试（重试 = 再开始一次这项操作）。
    #[serde(default)]
    pub retryable: bool,
    /// 失败页的标题；不给写“没有完成”。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// 重试按钮的文字；不给写“再试一次”。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_label: Option<String>,
}

// ============================ 凭据与授权 ============================

/// `credential/save` 参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialSaveParams {
    /// 不透明凭据（≤64 KiB）。
    pub credential: String,
    /// 下次要续期的时间（毫秒，契约 provider-protocol v12 缺口 7）：到了宿主叫 `session/renew`，插件没在跑也临时起来续。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub renew_at_ms: Option<i64>,
}

/// `session/renew` 参数（宿主→插件，需要能力 `session.renew`）：用这份凭据续期，经 `credential/save` 存新的再回空结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionRenewParams {
    /// 宿主存着的凭据。
    pub credential: String,
}

/// `account/avatar` 参数（宿主→插件，需要能力 `account.hostAvatar`，契约 provider-protocol v35）：本机用户现在的头像。
///
/// 会话上线后交一次，用户换头像、或用默认头像时改名字后再交；插件回空结果表示收到。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountAvatarParams {
    /// 头像；`null` 表示这个用户没有任何头像可交。
    #[serde(default)]
    pub avatar: Option<HostAvatar>,
}

/// 宿主交给插件的一张头像：缩到 [`crate::budget::HOST_AVATAR_MAX_SIDE`] 以内、不超过
/// [`crate::budget::HOST_AVATAR_MAX_BYTES`] 字节，只用 PNG 或 JPEG。
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostAvatar {
    /// `image/png` 或 `image/jpeg`。
    pub mime: String,
    /// 图片字节的 base64（标准字母表、带补齐）。
    pub data: String,
    /// 图片字节的 sha256（64 位小写十六进制），插件据此判断是不是同一张。
    pub sha256: String,
    /// 图片字节数。
    pub size: u64,
}

impl std::fmt::Debug for HostAvatar {
    // 不把整张图打进日志：只要类型、大小与摘要。
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HostAvatar")
            .field("mime", &self.mime)
            .field("size", &self.size)
            .field("sha256", &self.sha256)
            .finish_non_exhaustive()
    }
}

impl HostAvatar {
    /// 形状不对时的原因：类型不是 PNG / JPEG、超过上限、摘要不是 64 位小写十六进制、base64 长度与字节数对不上。
    ///
    /// # Examples
    ///
    /// ```
    /// use hhh_im_protocol::method::HostAvatar;
    /// let ok = HostAvatar { mime: "image/png".into(), data: "AAEC".into(), sha256: "a".repeat(64), size: 3 };
    /// assert_eq!(ok.problem(), None);
    /// let gif = HostAvatar { mime: "image/gif".into(), ..ok.clone() };
    /// assert!(gif.problem().is_some());
    /// ```
    pub fn problem(&self) -> Option<&'static str> {
        if !matches!(self.mime.as_str(), "image/png" | "image/jpeg") {
            return Some("头像只收 PNG 或 JPEG");
        }
        if self.size == 0 || self.size > crate::budget::HOST_AVATAR_MAX_BYTES as u64 {
            return Some("头像字节数为零或超过上限");
        }
        if self.sha256.len() != 64
            || !self
                .sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Some("头像摘要不是 64 位小写十六进制");
        }
        if self.data.len() as u64 != self.size.div_ceil(3) * 4 {
            return Some("头像 base64 长度与字节数对不上");
        }
        None
    }
}

/// `session/notice` 通知参数（插件→宿主）：这个连接现在有的问题，每次给全，空数组就是都没有了。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionNoticeParams {
    /// 问题。
    #[serde(default)]
    pub notices: Vec<Notice>,
}

/// 连接上的一个问题（画在卡片下面的问题行）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Notice {
    /// 编号（同一个问题前后不变）。
    pub id: String,
    /// 给用户看的一句话（不超过 [`crate::budget::NOTICE_MAX_CHARS`] 个字）。
    pub text: String,
    /// 要重新登录才能好：卡片右边多一颗“重新登录”。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub relogin: bool,
    /// 请用户处理的一件事（v13）：这一行画成待处理行（品牌黄），右边一颗按钮写 `label`，点了宿主叫
    /// `actions/start {actionId: id}`（这个编号不必出现在 `initialize.actions` 里）。处理完、作废了，插件在下一次
    /// `session/notice` 里去掉这一条。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<NoticeAction>,
    /// 要立刻请用户看（v13）：带 `action` 的新提醒出现时，宿主窗口在前台、也没有别的对话框开着，就直接开始这个操作。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub prompt: bool,
}

/// 提醒上的处理按钮（v13）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoticeAction {
    /// 操作编号。
    pub id: String,
    /// 按钮文字（至多 [`crate::budget::ACTION_LABEL_MAX_CHARS`] 个字）。
    pub label: String,
}

/// `session/expired` 通知参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionExpiredParams {
    /// 原因。
    pub reason: ReauthReason,
    /// 给用户看的说明。
    pub message: String,
}

// ============================ 分页 ============================

/// 一页结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page<T> {
    /// 本页项目。
    pub items: Vec<T>,
    /// 下一页游标；没有为空。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
}

/// 一页搜索结果（带不完整提示）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchPage<T> {
    /// 本页项目。
    pub items: Vec<T>,
    /// 下一页游标。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
    /// 范围/完整性提示。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notice: Option<String>,
}

// ============================ 读取参数/结果 ============================

/// `conversations/list` 参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationsListParams {
    /// 每页 1–100。
    pub limit: u16,
    /// 游标。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// `conversations/get` 参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationsGetParams {
    /// 会话编号。
    pub conversation_id: String,
}

/// 关键词搜索参数（会话与人员共用）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchParams {
    /// 关键词 1–50 字。
    pub text: String,
    /// 每页 1–20。
    pub limit: u16,
    /// 游标。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// `conversations/members` 参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MembersParams {
    /// 会话编号。
    pub conversation_id: String,
    /// 每页上限。
    pub limit: u16,
    /// 游标。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// `conversations/findDirect` 参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FindDirectParams {
    /// 对方用户编号。
    pub user_id: String,
}

/// `conversations/findDirect` 结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FindDirectResult {
    /// 找到的会话编号；没找到为空（不代表从未有过会话）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
    /// 提示。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notice: Option<String>,
}

/// `people/get` 参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeopleGetParams {
    /// 用户编号（与 `bots` 合计 ≤100 个）。
    pub ids: Vec<String>,
    /// 机器人与应用的编号（v15，可选）：只在程序声明 `people.bots` 时给；读到的放进 `found`、种类写 `bot`。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bots: Vec<String>,
}

/// `people/get` 结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeopleGetResult {
    /// 读到的。
    pub found: Vec<Person>,
    /// 平台明确说不存在的；这次没读到的两边都不放。
    #[serde(default)]
    pub missing: Vec<String>,
}

/// `messages/list` 参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessagesListParams {
    /// 会话编号。
    pub conversation_id: String,
    /// 话题编号。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
    /// 只要这个时间之后的（毫秒）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_ms: Option<i64>,
    /// 只要这个时间之前的（毫秒）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before_ms: Option<i64>,
    /// 从最新往更早翻。
    #[serde(default)]
    pub newest_first: bool,
    /// 每页上限。
    pub limit: u16,
    /// 游标。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// `messages/get` 参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessagesGetParams {
    /// 会话编号。
    pub conversation_id: String,
    /// 消息编号。
    pub message_id: String,
}

/// `reactions/list` 参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReactionsListParams {
    /// 会话编号。
    pub conversation_id: String,
    /// 消息编号。
    pub message_id: String,
    /// 每页 ≤50。
    pub limit: u16,
    /// 游标。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// `emoji/catalog` 结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmojiCatalogResult {
    /// 表情。
    pub emojis: Vec<Emoji>,
    /// 分类。
    #[serde(default)]
    pub categories: Vec<EmojiCategory>,
    /// 肤色，按面板里的顺序（v14）；没有肤色选择时不写。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tones: Vec<EmojiTone>,
    /// 目录更新时间（毫秒）。
    pub updated_at_ms: i64,
}

// ============================ 资源 ============================

/// `resources/read` 参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourcesReadParams {
    /// 会话编号。
    pub conversation_id: String,
    /// 消息编号。
    pub message_id: String,
    /// 资源键。
    pub key: String,
    /// 资源种类。
    pub kind: ResourceKind,
    /// 起始字节。
    pub offset: u64,
    /// 最多读多少（≤1 MiB）。
    pub length: u64,
    /// `resources/describe` 给出的实际版本编号；旧宿主省略时保留插件原来的取法。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
}

impl ResourcesReadParams {
    /// 分段流的身份包含版本，不能在预览与原件之间复用读流。
    pub fn segment_key(&self) -> String {
        serde_json::to_string(&(
            &self.conversation_id,
            &self.message_id,
            &self.key,
            self.kind,
            &self.variant,
        ))
        .expect("资源编号与类型总能序列化")
    }
}

/// `resources/describe` 的参数；不读取文件字节。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourcesDescribeParams {
    pub conversation_id: String,
    pub message_id: String,
    pub key: String,
    pub kind: ResourceKind,
}

/// `avatars/read` 参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvatarsReadParams {
    /// 头像键。
    pub key: String,
    /// 起始字节。
    pub offset: u64,
    /// 最多读多少。
    pub length: u64,
}

/// `emoji/image` 参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmojiImageParams {
    /// 表情类型。
    pub emoji_type: String,
    /// 起始字节。
    pub offset: u64,
    /// 最多读多少。
    pub length: u64,
}

/// 读资源一段的结果（protocol-v1 §9.1）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceChunk {
    /// 这一段的字节（base64）。
    pub data: String,
    /// 整个资源总字节；能给就给。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    /// 平台给的内容指纹（小写十六进制）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    /// 是不是读到末尾了。
    pub eof: bool,
}

// ============================ 上传 ============================

/// `uploads/begin` 参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadsBeginParams {
    /// 资源种类。
    pub kind: ResourceKind,
    /// 文件名。
    pub name: String,
    /// 总字节数。
    pub bytes: u64,
}

/// `uploads/begin` 结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadsBeginResult {
    /// 上传编号。
    pub upload_id: String,
}

/// `uploads/chunk` 参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadsChunkParams {
    /// 上传编号。
    pub upload_id: String,
    /// 起始字节。
    pub offset: u64,
    /// 这一段的字节（base64，≤1 MiB）。
    pub data: String,
}

/// `uploads/finish` 参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadsFinishParams {
    /// 上传编号。
    pub upload_id: String,
    /// 全文件 SHA-256（小写十六进制）。
    pub sha256: String,
}

/// `uploads/finish` 结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadsFinishResult {
    /// 资源键（填进发送内容的 image/file）。
    pub key: String,
}

/// `uploads/abort` 参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadsAbortParams {
    /// 上传编号。
    pub upload_id: String,
}

// ============================ 动作 ============================

/// `messages/send` 参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessagesSendParams {
    /// 伙伴代发来源；只向声明了 agentAttribution.send 的插件提供。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_attribution: Option<Box<crate::AgentAttribution>>,
    /// 操作编号（同一个只发一次）。
    pub operation_id: String,
    /// 发送目标。
    pub target: SendTarget,
    /// 内容。
    pub content: Vec<Outgoing>,
}

/// `messages/lookupSend` 参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessagesLookupSendParams {
    /// 操作编号。
    pub operation_id: String,
}

/// `messages/recall` 参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessagesRecallParams {
    /// 会话编号。
    pub conversation_id: String,
    /// 消息编号。
    pub message_id: String,
}

/// `messages/edit` 参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessagesEditParams {
    /// 会话编号。
    pub conversation_id: String,
    /// 消息编号。
    pub message_id: String,
    /// 新内容。
    pub content: Vec<Outgoing>,
}

/// `reactions/add` 参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReactionsAddParams {
    /// 会话编号。
    pub conversation_id: String,
    /// 消息编号。
    pub message_id: String,
    /// 表情类型。
    pub emoji_type: String,
}

/// `reactions/remove` 参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReactionsRemoveParams {
    /// 会话编号。
    pub conversation_id: String,
    /// 消息编号。
    pub message_id: String,
    /// 表情编号。
    pub reaction_id: String,
}

/// `conversations/recentActivity` 参数（会话发现，契约 provider-protocol v12 缺口 1）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentActivityParams {
    /// 从这个时间（毫秒）起，含这一刻。
    pub since_ms: i64,
    /// 一次最多翻几页。
    pub pages_max: u16,
    /// 宿主已经在同步的会话（v43，可选）：程序可以不核对、不报它们；宿主照旧滤掉报来的。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skip: Vec<String>,
    /// 宿主要的第一页至多几条（v43，可选）：给了，程序才可以在结果项里附上 [`ActivityItem::first_page`]。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_page_limit: Option<u16>,
}

/// `conversations/recentActivity` 结果。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentActivityResult {
    /// 这段时间里有人（不算机器人、应用、经接口发的）发过消息的会话，每个会话一项，按最新一条从新到旧。
    #[serde(default)]
    pub items: Vec<ActivityItem>,
    /// 翻到了起点；翻满页数还没到时为假。
    #[serde(default)]
    pub complete: bool,
}

/// 一段时间里有人发过消息的一个会话。消息内容只经 [`ActivityItem::first_page`] 交出。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityItem {
    /// 会话编号。
    pub conversation_id: String,
    /// 会话类别。
    pub kind: ConversationKind,
    /// 这段时间里最早一条的时间（毫秒）。
    pub first_ms: i64,
    /// 最新一条的时间（毫秒）。
    pub latest_ms: i64,
    /// 私聊的对方（不是本人的发送者）；群或只有本人发过时为空。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub peer_id: Option<String>,
    /// 群与频道的资料（v15，可选）：程序手边正好有（如本机记着的会话列表）时给，宿主就不再为它读一次
    /// `conversations/get`；私聊不给（名字与头像是对方本人的，宿主另读）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversation: Option<Conversation>,
    /// 这个会话最新的一页消息（v43，可选）：程序核对时正好读到了会话最新的消息才给，宿主第一次同步它时就用这页，
    /// 不再读一遍。宿主没给 `firstPageLimit` 时不给；一次结果里所有 `firstPage` 合起来不超过
    /// [`crate::budget::RECENT_ACTIVITY_PAGES_BYTES_MAX`]，装不下的项不附（按从新到旧的顺序先附前面的）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_page: Option<FirstPage>,
}

/// 会话最新的一页消息（[`ActivityItem::first_page`]，v43）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirstPage {
    /// 会话主消息流里最新的至多 `firstPageLimit` 条，不带时间条件：与 `messages/list`（`newestFirst`）读到的同一批、
    /// 同样翻译，页内从旧到新。
    #[serde(default)]
    pub items: Vec<Message>,
    /// 平台上还有比这页更早的消息。
    #[serde(default)]
    pub more: bool,
}

/// `messages/retranslate` 参数（protocol-v1 §10.1）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetranslateParams {
    /// 规则名。
    pub rule: String,
    /// 要重译的旧消息（带原始数据）。
    pub messages: Vec<RetranslateItem>,
}

/// 要重译的一条旧消息。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetranslateItem {
    /// 消息编号。
    pub id: String,
    /// 会话编号。
    pub conversation_id: String,
    /// 去凭据化的原始数据。
    pub raw: Raw,
}

/// `messages/retranslate` 结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetranslateResult {
    /// 重译后的消息（宿主按编号覆盖本地）。
    pub messages: Vec<Message>,
}

/// 已存消息维护的工作种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MaintenanceMode {
    /// 纯本地修正正文与编辑标记。
    Repair,
    /// 纯本地判断哪些消息需要宿主稍后重新读取。
    Reread,
}

/// `messages/maintain` 参数；一次只执行一个声明过的规则。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaintenanceParams {
    /// 工作种类。
    pub mode: MaintenanceMode,
    /// 规则名；沿用已发布名字以保留完成记录。
    pub rule: String,
    /// 当前统一消息，不需要 raw；条数与字节数都受预算约束。
    pub messages: Vec<Message>,
}

/// 同一账号内的一条消息。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaintenanceRef {
    /// 会话编号。
    pub conversation_id: String,
    /// 消息编号。
    pub id: String,
}

/// 本地修正只改变正文及翻译状态；其余身份、话题、资源来源与原始数据由宿主保留。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaintenanceUpdate {
    /// 消息位置。
    #[serde(flatten)]
    pub message: MaintenanceRef,
    /// 修正后的完整正文。
    pub content: Vec<crate::types::Block>,
    /// 修正后的编辑标记；null 表示清除旧的错误标记。
    pub edited_at_ms: Option<i64>,
    /// 是否仍有翻译不了的内容。
    pub incomplete: bool,
}

/// `messages/maintain` 结果，只能引用请求里的编号且每条最多一次。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaintenanceResult {
    /// Repair 模式下发生变化的消息；不变的省略。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub updates: Vec<MaintenanceUpdate>,
    /// Reread 模式下需要重新读取的消息；不符合条件的不写永久完成记录。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reread: Vec<MaintenanceRef>,
}

/// `reactions/list` / `messages/send` 等用到的发送结果别名。
pub type SendResult = SendOutcome;

// ============================ 推送与状态 ============================

/// `events/push` 参数（protocol-v1 §11.1）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventsPushParams {
    /// 这一批事件（≤500）。
    pub events: Vec<Event>,
    /// 平台同步位置（history=none 必带）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sync_state: Option<String>,
}

/// 连接状态（§11.3）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConnState {
    /// 在线。
    Online,
    /// 连接中。
    Connecting,
    /// 离线。
    Offline,
}

/// `status` 通知参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusParams {
    /// 状态。
    pub state: ConnState,
    /// 原因。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// 日志级别。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LogLevel {
    /// 错误。
    Error,
    /// 关注。
    Warn,
    /// 关键节点。
    Info,
    /// 过程。
    Debug,
}

/// `log/write` 通知参数（§11.5）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogWriteParams {
    /// 级别。
    pub level: LogLevel,
    /// 消息（不含凭据、消息正文）。
    pub message: String,
    /// 结构化字段。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fields: Option<std::collections::BTreeMap<String, serde_json::Value>>,
}

/// `$/cancel` 通知参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelParams {
    /// 要取消的请求编号。
    pub id: crate::rpc::RpcId,
}

// ============================ 性能自报（契约 provider-protocol v32） ============================

/// 自报分布的桶边界（毫秒）：第 i 个桶装 `(边界[i-1], 边界[i]]`，最后一个桶装超过最大边界的。由 SDK 固定，宿主按它还原。
pub const PERF_HISTOGRAM_BOUNDS_MS: [f64; 12] = [
    1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0, 200.0, 500.0, 1000.0, 2000.0, 5000.0,
];

/// `perf/start` 参数：剖析开启，插件开始按间隔自报。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerfStartParams {
    /// 宿主的剖析会话编号；报告带回它，宿主丢掉别的会话的报告。
    pub session: u64,
    /// 报告间隔（毫秒），不小于 [`crate::budget::PERF_INTERVAL_MIN_MS`]。
    pub interval_ms: u64,
}

/// `perf/stop` 参数：剖析关闭，插件停止自报。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerfStopParams {
    /// 要停的会话编号。
    pub session: u64,
}

/// 自报的一个区间。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerfSpan {
    /// 方法名或插件声明的固定标签（不含编号、正文、令牌）。
    pub name: String,
    /// 开始时刻：相对本窗口起点（`fromMs`）的偏移，毫秒。
    pub start_ms: f64,
    /// 时长，毫秒。
    pub dur_ms: f64,
}

/// 自报的一个计数（本窗口里的增量）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerfCounter {
    /// 方法名或固定标签。
    pub name: String,
    /// 本窗口的增量。
    pub value: u64,
}

/// 自报的一个分布（本窗口里同一名字的全部耗时）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerfHistogram {
    /// 方法名或固定标签。
    pub name: String,
    /// 条数。
    pub count: u64,
    /// 合计毫秒。
    pub sum_ms: f64,
    /// 最长一条毫秒。
    pub max_ms: f64,
    /// 各桶条数，按 [`PERF_HISTOGRAM_BOUNDS_MS`]，比边界多一个（最后一个装超过最大边界的）。
    pub buckets: Vec<u64>,
}

/// `perf/report` 通知参数：一个间隔里插件内部的耗时。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerfReportParams {
    /// `perf/start` 给的会话编号。
    pub session: u64,
    /// 本会话里第几条（从 0 起）。
    pub seq: u64,
    /// 本窗口起点：插件自己的单调时钟（毫秒），宿主只用 `toMs - fromMs` 与区间偏移，不信绝对时刻。
    pub from_ms: f64,
    /// 本窗口终点。
    pub to_ms: f64,
    /// 区间（只报最慢与超过阈值的）。
    #[serde(default)]
    pub spans: Vec<PerfSpan>,
    /// 计数。
    #[serde(default)]
    pub counters: Vec<PerfCounter>,
    /// 分布。
    #[serde(default)]
    pub histograms: Vec<PerfHistogram>,
}

impl PerfReportParams {
    /// 按 [`crate::budget`] 的自报上限核对（字节数由收到这一行的一方另核）；超出任何一项整条不收。
    ///
    /// # Errors
    /// 返回超出的是哪一项。
    ///
    /// # Examples
    ///
    /// ```
    /// use hhh_im_protocol::method::{PerfCounter, PerfReportParams};
    /// let mut report = PerfReportParams {
    ///     session: 1, seq: 0, from_ms: 0.0, to_ms: 1000.0,
    ///     spans: vec![], counters: vec![PerfCounter { name: "平台请求".into(), value: 3 }], histograms: vec![],
    /// };
    /// assert!(report.check().is_ok());
    /// report.counters[0].name = "x".repeat(200);
    /// assert!(report.check().is_err());
    /// ```
    pub fn check(&self) -> Result<(), &'static str> {
        use crate::budget::{PERF_NAME_CHARS_MAX, PERF_SERIES_MAX, PERF_SPANS_MAX};
        let name_ok = |name: &str| {
            !name.is_empty()
                && name.chars().count() <= PERF_NAME_CHARS_MAX
                && !name.chars().any(char::is_control)
        };
        let time_ok = |ms: f64| ms.is_finite() && ms >= 0.0;
        if !time_ok(self.from_ms) || !time_ok(self.to_ms) || self.to_ms < self.from_ms {
            return Err("窗口起止时刻不对");
        }
        if self.spans.len() > PERF_SPANS_MAX {
            return Err("区间太多");
        }
        if self.counters.len() + self.histograms.len() > PERF_SERIES_MAX {
            return Err("计数与分布太多");
        }
        let window = self.to_ms - self.from_ms;
        for span in &self.spans {
            if !name_ok(&span.name) {
                return Err("区间的名字不对");
            }
            if !time_ok(span.start_ms) || !time_ok(span.dur_ms) || span.start_ms > window {
                return Err("区间的时刻不对");
            }
        }
        if self.counters.iter().any(|c| !name_ok(&c.name)) {
            return Err("计数的名字不对");
        }
        for hist in &self.histograms {
            if !name_ok(&hist.name) {
                return Err("分布的名字不对");
            }
            if hist.buckets.len() != PERF_HISTOGRAM_BOUNDS_MS.len() + 1
                || hist.buckets.iter().sum::<u64>() != hist.count
                || !time_ok(hist.sum_ms)
                || !time_ok(hist.max_ms)
            {
                return Err("分布的形状不对");
            }
        }
        Ok(())
    }
}

/// 一个耗时落在自报分布的第几个桶。
///
/// # Examples
///
/// ```
/// use hhh_im_protocol::method::{perf_bucket_of, PERF_HISTOGRAM_BOUNDS_MS};
/// assert_eq!(perf_bucket_of(0.5), 0);
/// assert_eq!(perf_bucket_of(1.0), 0);
/// assert_eq!(perf_bucket_of(1.5), 1);
/// assert_eq!(perf_bucket_of(9_999.0), PERF_HISTOGRAM_BOUNDS_MS.len());
/// ```
pub fn perf_bucket_of(ms: f64) -> usize {
    PERF_HISTOGRAM_BOUNDS_MS
        .iter()
        .position(|bound| ms <= *bound)
        .unwrap_or(PERF_HISTOGRAM_BOUNDS_MS.len())
}

#[cfg(test)]
mod tests {
    // 契约 provider-protocol v32：perf/* 的线上形状是驼峰、可选字段缺省为空；不声明 perf 的能力清单写回不多出键。
    #[test]
    fn perf_shapes_round_trip_and_are_bounded() {
        use super::*;
        use serde_json::json;
        let start: PerfStartParams =
            serde_json::from_value(json!({ "session": 3, "intervalMs": 1000 })).unwrap();
        assert_eq!(start.interval_ms, 1000);
        let report: PerfReportParams = serde_json::from_value(json!({
            "session": 3, "seq": 0, "fromMs": 10.0, "toMs": 1010.0,
            "spans": [{ "name": "处理 · messages/list", "startMs": 5.0, "durMs": 120.0 }],
            "histograms": [{ "name": "处理 · messages/list", "count": 2, "sumMs": 130.0, "maxMs": 120.0,
                "buckets": [0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0] }]
        }))
        .unwrap();
        assert!(report.counters.is_empty());
        assert_eq!(report.check(), Ok(()));
        let mut bad = report.clone();
        bad.histograms[0].buckets.pop();
        assert_eq!(bad.check(), Err("分布的形状不对"));
        let mut bad = report.clone();
        bad.spans = vec![report.spans[0].clone(); crate::budget::PERF_SPANS_MAX + 1];
        assert_eq!(bad.check(), Err("区间太多"));
        let mut bad = report.clone();
        bad.spans[0].start_ms = 5_000.0;
        assert_eq!(bad.check(), Err("区间的时刻不对"));
        let mut bad = report;
        bad.counters = (0..crate::budget::PERF_SERIES_MAX)
            .map(|i| PerfCounter {
                name: format!("c{i}"),
                value: 1,
            })
            .collect();
        assert_eq!(bad.check(), Err("计数与分布太多"));

        let caps = crate::Capabilities::default();
        assert!(serde_json::to_value(&caps).unwrap().get("perf").is_none());
        let declared: crate::Capabilities =
            serde_json::from_value(json!({ "perf": true })).unwrap();
        assert!(declared.perf);
        for (m, name) in [
            (Method::PerfStart, "perf/start"),
            (Method::PerfStop, "perf/stop"),
            (Method::PerfReport, "perf/report"),
        ] {
            assert_eq!(m.as_str(), name);
            assert_eq!(Method::from_name(name), Some(m));
        }
    }

    #[test]
    fn maintenance_shapes_round_trip_and_updates_explicitly_replace_flags() {
        use serde_json::json;
        let params = json!({"mode": "repair", "rule": "old-text-v1", "messages": []});
        let parsed: MaintenanceParams = serde_json::from_value(params.clone()).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), params);
        let result = json!({"updates": [{"conversationId": "c", "id": "m", "content": [{"type": "text", "text": "fixed"}], "editedAtMs": null, "incomplete": false}]});
        let parsed: MaintenanceResult = serde_json::from_value(result.clone()).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), result);
        assert_eq!(
            serde_json::to_value(MaintenanceResult::default()).unwrap(),
            json!({})
        );
        assert!(
            serde_json::from_value::<MaintenanceParams>(
                json!({"mode": "future", "rule": "r", "messages": []})
            )
            .is_err()
        );
    }

    // 契约 provider-protocol v14：登录方式的默认中继、App Secret 的说明、相对的回调路径都可选、原样往返。
    #[test]
    fn v14_login_method_fields_are_optional_and_round_trip() {
        let relay = serde_json::json!({ "id": "relay", "kind": "relay", "label": "通过中继登录", "note": "说明",
            "defaultValue": "https://relay.example.com" });
        let own = serde_json::json!({ "id": "own", "kind": "ownApp", "label": "使用自己的应用", "note": "",
            "secretHelp": "开放平台 → 凭证与基础信息。只保存在本机。", "callbackPath": "callback" });
        for declared in [relay, own] {
            let method: LoginMethod = serde_json::from_value(declared.clone()).unwrap();
            assert_eq!(serde_json::to_value(&method).unwrap(), declared);
        }
        let old: LoginMethod = serde_json::from_value(
            serde_json::json!({ "id": "own", "kind": "ownApp", "label": "自己的应用" }),
        )
        .unwrap();
        assert!(old.default_value.is_none() && old.secret_help.is_none());
    }
    use super::*;

    #[test]
    fn method_names_round_trip() {
        for m in [
            Method::Initialize,
            Method::ConversationsFindDirect,
            Method::MessagesRetranslate,
            Method::EventsPush,
            Method::MessagesMaintain,
            Method::Cancel,
        ] {
            assert_eq!(Method::from_name(m.as_str()), Some(m));
        }
        assert_eq!(
            Method::ConversationsFindDirect.as_str(),
            "conversations/findDirect"
        );
        assert_eq!(Method::Cancel.as_str(), "$/cancel");
        assert!(Method::from_name("does/not-exist").is_none());
        for (m, name) in [
            (
                Method::ConversationsRecentActivity,
                "conversations/recentActivity",
            ),
            (Method::SessionNotice, "session/notice"),
            (Method::SessionRenew, "session/renew"),
            (Method::LoginCallback, "login/callback"),
            (Method::AccountAvatar, "account/avatar"),
        ] {
            assert_eq!(m.as_str(), name);
            assert_eq!(Method::from_name(name), Some(m));
        }
    }

    // 契约 provider-protocol v35：`account/avatar` 的参数原样往返；没有头像时是 null；形状不对的认得出；日志里不打整张图。
    #[test]
    fn v35_account_avatar_round_trips_and_checks_its_shape() {
        use serde_json::json;
        let wire = json!({ "avatar": { "mime": "image/png", "data": "iVBORw==", "sha256": "0".repeat(64), "size": 5 } });
        let params: AccountAvatarParams = serde_json::from_value(wire.clone()).unwrap();
        let avatar = params.avatar.clone().unwrap();
        assert_eq!((avatar.mime.as_str(), avatar.size), ("image/png", 5));
        assert_eq!(avatar.problem(), None);
        assert_eq!(serde_json::to_value(&params).unwrap(), wire);
        let none: AccountAvatarParams = serde_json::from_value(json!({ "avatar": null })).unwrap();
        assert_eq!(none.avatar, None);
        let missing: AccountAvatarParams = serde_json::from_value(json!({})).unwrap();
        assert_eq!(missing, none, "没写 avatar 与 null 一样");
        let bad = [
            HostAvatar {
                mime: "image/webp".into(),
                ..avatar.clone()
            },
            HostAvatar {
                size: crate::budget::HOST_AVATAR_MAX_BYTES as u64 + 1,
                ..avatar.clone()
            },
            HostAvatar {
                size: 0,
                data: String::new(),
                ..avatar.clone()
            },
            HostAvatar {
                sha256: "A".repeat(64),
                ..avatar.clone()
            },
            HostAvatar {
                sha256: "0".repeat(63),
                ..avatar.clone()
            },
            HostAvatar {
                data: "iVBORw==AAAA".into(),
                ..avatar.clone()
            },
        ];
        for one in bad {
            assert!(one.problem().is_some(), "{one:?}");
        }
        let shown = format!("{avatar:?}");
        assert!(!shown.contains("iVBORw"), "日志里不该有图片字节：{shown}");
    }

    #[test]
    fn host_user_name_has_only_the_optional_public_name() {
        use serde_json::json;
        assert_eq!(Method::HostUserName.as_str(), "host/userName");
        assert_eq!(
            Method::from_name("host/userName"),
            Some(Method::HostUserName)
        );
        let absent: HostUserNameResult = serde_json::from_value(json!({})).unwrap();
        assert_eq!(serde_json::to_value(absent).unwrap(), json!({}));
        let named: HostUserNameResult =
            serde_json::from_value(json!({"name": "梨子 小明"})).unwrap();
        assert_eq!(
            serde_json::to_value(named).unwrap(),
            json!({"name": "梨子 小明"})
        );
    }

    // 契约 provider-protocol v12：登录、续期、交接相关的新字段都是可选的——旧插件、旧宿主的形状照读，写回不多出键。
    #[test]
    fn v12_session_and_login_fields_are_optional_and_round_trip() {
        use serde_json::json;
        // login/start：旧宿主发 {}，读成全空；新宿主给方式、内容、回调地址与 state。
        let old: LoginStartParams = serde_json::from_value(json!({})).unwrap();
        assert_eq!(old, LoginStartParams::default());
        assert_eq!(serde_json::to_value(&old).unwrap(), json!({}));
        let start = json!({ "method": "own", "values": { "appId": "cli_1", "appSecret": "s" },
            "callbackUrl": "http://localhost:29527/hhh-friend-im/ext-sample/callback", "state": "st" });
        let parsed: LoginStartParams = serde_json::from_value(start.clone()).unwrap();
        assert_eq!(parsed.values[login_value::APP_ID], "cli_1");
        assert_eq!(serde_json::to_value(&parsed).unwrap(), start);

        // session/open：不借用、没有交接时不多出键；在线结果不带 adoptedSettings 时照旧。
        let open: SessionOpenParams = serde_json::from_value(json!({ "settings": {} })).unwrap();
        assert!(!open.credential_read_only && open.legacy.is_none());
        let v = serde_json::to_value(&open).unwrap();
        assert!(v.get("credentialReadOnly").is_none() && v.get("legacy").is_none());
        let online: SessionOpenResult = serde_json::from_value(
            json!({ "state": "online", "account": { "id": "u1", "name": "我" } }),
        )
        .unwrap();
        assert!(matches!(
            online,
            SessionOpenResult::Online {
                adopted_settings: None,
                ..
            }
        ));
        let adopted = json!({ "state": "online", "account": { "id": "u1", "name": "我" },
            "adoptedSettings": { "parameters": { "topic": "on" }, "login": { "method": "relay", "values": { "relayUrl": "https://r.example" } } } });
        let back: SessionOpenResult = serde_json::from_value(adopted.clone()).unwrap();
        assert_eq!(serde_json::to_value(&back).unwrap(), adopted);

        // credential/save 的续期时间、session/notice 每次给全。
        let save: CredentialSaveParams =
            serde_json::from_value(json!({ "credential": "c" })).unwrap();
        assert!(save.renew_at_ms.is_none());
        let notice: SessionNoticeParams = serde_json::from_value(
            json!({ "notices": [{ "id": "scope", "text": "缺少权限 xxx", "relogin": true }] }),
        )
        .unwrap();
        assert!(notice.notices[0].relogin);
        assert_eq!(
            serde_json::from_value::<SessionNoticeParams>(json!({})).unwrap(),
            SessionNoticeParams::default()
        );

        // 登录方式：不认识的类型读成 Unknown，不让整个握手失败。
        let method: LoginMethod =
            serde_json::from_value(json!({ "id": "x", "kind": "passkey", "label": "通行密钥" }))
                .unwrap();
        assert_eq!(method.kind, LoginKind::Unknown);

        // 会话发现的结果。
        let activity: RecentActivityResult = serde_json::from_value(json!({
            "items": [{ "conversationId": "c1", "kind": "direct", "firstMs": 1, "latestMs": 2, "peerId": "u2" }],
            "complete": true
        }))
        .unwrap();
        assert_eq!(activity.items[0].peer_id.as_deref(), Some("u2"));
        // v43 的新字段都可选：旧插件的结果没有 firstPage，旧宿主的参数没有 skip / firstPageLimit，没给的不写出去。
        assert!(activity.items[0].first_page.is_none());
        let old: RecentActivityParams =
            serde_json::from_value(json!({ "sinceMs": 1, "pagesMax": 2 })).unwrap();
        assert!(old.skip.is_empty() && old.first_page_limit.is_none());
        assert_eq!(
            serde_json::to_value(&old).unwrap(),
            json!({ "sinceMs": 1, "pagesMax": 2 })
        );
        let paged: RecentActivityResult = serde_json::from_value(json!({
            "items": [{ "conversationId": "c1", "kind": "group", "firstMs": 1, "latestMs": 2,
                        "firstPage": { "items": [], "more": true } }],
            "complete": true
        }))
        .unwrap();
        let page = paged.items[0].first_page.as_ref().unwrap();
        assert_eq!((page.items.len(), page.more), (0, true));
    }

    #[test]
    fn session_open_result_is_tagged_by_state() {
        let needs = SessionOpenResult::NeedsLogin {
            reason: NeedsLoginReason::NoCredential,
            message: "请扫码".into(),
        };
        let v = serde_json::to_value(&needs).unwrap();
        assert_eq!(v["state"], "needsLogin");
        assert_eq!(v["reason"], "noCredential");
    }

    #[test]
    fn login_step_qr_round_trips() {
        let step = LoginStep::QrCode {
            image: "iVBOR".into(),
            expires_at_ms: 1,
            hint: Some("扫码".into()),
            note: None,
            links: vec![],
        };
        let v = serde_json::to_value(&step).unwrap();
        assert_eq!(v["kind"], "qrCode");
        assert_eq!(v["expiresAtMs"], 1);
        let back: LoginStep = serde_json::from_value(v).unwrap();
        matches!(back, LoginStep::QrCode { .. });
    }

    /// 提示是可选的（契约 provider-protocol v8）：旧插件不带 `note` 照常读、写回时也不多出这个键；
    /// 二维码、打开链接、在手机上确认、表单四种步骤带了就原样往返。
    #[test]
    fn login_notes_are_optional_and_round_trip_on_every_step_that_takes_one() {
        use serde_json::json;
        let old = json!({ "kind": "confirmOnDevice", "message": "请在手机上确认" });
        let step: LoginStep = serde_json::from_value(old.clone()).unwrap();
        assert!(matches!(
            step,
            LoginStep::ConfirmOnDevice { note: None, .. }
        ));
        assert_eq!(
            serde_json::to_value(&step).unwrap(),
            old,
            "没带提示的写回时不多出 note"
        );

        let note = json!({ "text": "扫码后勾选同步", "tone": "info" });
        for mut shape in [
            json!({ "kind": "qrCode", "image": "iVBOR", "expiresAtMs": 1, "hint": "扫码" }),
            json!({ "kind": "openUrl", "url": "https://example.com/login", "label": "打开" }),
            json!({ "kind": "confirmOnDevice", "message": "请在手机上确认" }),
            json!({ "kind": "input", "fields": [{ "id": "code", "label": "验证码", "secret": false }] }),
        ] {
            shape["note"] = note.clone();
            let step: LoginStep = serde_json::from_value(shape.clone()).unwrap();
            assert_eq!(
                serde_json::to_value(&step).unwrap(),
                shape,
                "带提示的原样往返：{shape}"
            );
        }
        let warn: LoginNote =
            serde_json::from_value(json!({ "text": "注意", "tone": "warn" })).unwrap();
        assert_eq!(warn.tone, MessageTone::Warn);
    }

    // 契约 provider-protocol v15：会话发现的一项可带群与频道的资料；不带时写回不多出键。
    #[test]
    fn v15_activity_items_may_carry_the_conversation() {
        use serde_json::json;
        let with = json!({ "conversationId": "C1", "kind": "channel", "firstMs": 1, "latestMs": 2,
            "conversation": { "id": "C1", "kind": "channel", "name": "general", "private": true } });
        let item: ActivityItem = serde_json::from_value(with.clone()).unwrap();
        assert_eq!(
            item.conversation.as_ref().map(|c| c.name.as_str()),
            Some("general")
        );
        assert_eq!(serde_json::to_value(&item).unwrap(), with);
        let old = json!({ "conversationId": "D1", "kind": "direct", "firstMs": 1, "latestMs": 2, "peerId": "U1" });
        let item: ActivityItem = serde_json::from_value(old.clone()).unwrap();
        assert_eq!(item.conversation, None);
        assert_eq!(
            serde_json::to_value(&item).unwrap(),
            old,
            "旧形状写回不多出键"
        );
    }

    // 契约 provider-protocol v13：新方法名原样往返。
    #[test]
    fn v13_method_names_round_trip() {
        for (m, name) in [
            (Method::ActionsStart, "actions/start"),
            (Method::ActionStep, "action/step"),
            (Method::ActionSubmit, "action/submit"),
            (Method::ActionCancel, "action/cancel"),
            (Method::ActionFinished, "action/finished"),
            (Method::ActionFailed, "action/failed"),
            (Method::StepChoose, "step/choose"),
        ] {
            assert_eq!(m.as_str(), name);
            assert_eq!(Method::from_name(name), Some(m));
        }
    }

    // 契约 provider-protocol v13：用宿主账号登录、操作声明、提醒的处理按钮、成败页的文字都可选——旧形状照读、
    // 写回不多出键；新形状原样往返；不认识的位置读成 Unknown。
    #[test]
    fn v13_host_login_actions_and_notice_fields_are_optional_and_round_trip() {
        use serde_json::json;
        let host: LoginMethod = serde_json::from_value(
            json!({ "id": "cindy", "kind": "host", "label": "用 Cindy 账号登录" }),
        )
        .unwrap();
        assert_eq!(host.kind, LoginKind::Host);

        let start = json!({ "method": "cindy", "values": { "hostToken": "tok" },
            "hostAccount": { "name": "李子", "detail": "个人", "avatarUrl": "https://example.com/a.png" } });
        let parsed: LoginStartParams = serde_json::from_value(start.clone()).unwrap();
        assert_eq!(parsed.values[login_value::HOST_TOKEN], "tok");
        assert_eq!(serde_json::to_value(&parsed).unwrap(), start);
        let shown = format!("{parsed:?}");
        assert!(!shown.contains("tok\""), "Debug 不打印令牌：{shown}");
        let no_avatar: HostAccount =
            serde_json::from_value(json!({ "name": "XD", "detail": "组织" })).unwrap();
        assert!(no_avatar.avatar_url.is_none());
        assert_eq!(
            serde_json::to_value(&no_avatar).unwrap(),
            json!({ "name": "XD", "detail": "组织" })
        );

        let decls = json!([
            { "id": "devices", "label": "我的电脑", "scope": "account" },
            { "id": "members", "label": "群成员", "scope": "conversation", "conversationKinds": ["group"] }
        ]);
        let parsed: Vec<ActionDecl> = serde_json::from_value(decls.clone()).unwrap();
        assert_eq!(serde_json::to_value(&parsed).unwrap(), decls);
        assert!(
            !parsed[0].applies_to(ConversationKind::Group),
            "账号级不进会话菜单"
        );
        assert!(parsed[1].applies_to(ConversationKind::Group));
        assert!(!parsed[1].applies_to(ConversationKind::Direct));
        let odd: ActionDecl =
            serde_json::from_value(json!({ "id": "x", "label": "X", "scope": "toolbar" })).unwrap();
        assert_eq!(odd.scope, ActionScope::Unknown);

        // 提醒：旧形状不多出键；带处理按钮与 prompt 的原样往返。
        let plain = json!({ "id": "scope", "text": "缺少权限" });
        let notice: Notice = serde_json::from_value(plain.clone()).unwrap();
        assert!(notice.action.is_none() && !notice.prompt);
        assert_eq!(serde_json::to_value(&notice).unwrap(), plain);
        let ask = json!({ "id": "approve-1", "text": "新电脑「Mac」等你批准",
            "action": { "id": "approve:1", "label": "查看" }, "prompt": true });
        let notice: Notice = serde_json::from_value(ask.clone()).unwrap();
        assert_eq!(serde_json::to_value(&notice).unwrap(), ask);

        // 成败页的文字可选。
        let finished: LoginFinishedParams = serde_json::from_value(
            json!({ "flowId": "f", "account": { "id": "u", "name": "我" } }),
        )
        .unwrap();
        assert!(finished.title.is_none() && finished.message.is_none());
        let failed = json!({ "flowId": "f", "message": "那台电脑拒绝了", "retryable": true,
            "title": "没有批准", "retryLabel": "重新请求" });
        let parsed: LoginFailedParams = serde_json::from_value(failed.clone()).unwrap();
        assert_eq!(serde_json::to_value(&parsed).unwrap(), failed);
        let action_failed: ActionFailedParams = serde_json::from_value(failed.clone()).unwrap();
        assert_eq!(serde_json::to_value(&action_failed).unwrap(), failed);
        let done: ActionFinishedParams = serde_json::from_value(json!({ "flowId": "f" })).unwrap();
        assert!(!done.remove_from_list, "旧形状照读");
        assert_eq!(
            serde_json::to_value(&done).unwrap(),
            json!({ "flowId": "f" })
        );
        // v19：做完后让会话离开列表；`false` 写回不多出键。
        let removed = json!({ "flowId": "f", "removeFromList": true });
        let parsed: ActionFinishedParams = serde_json::from_value(removed.clone()).unwrap();
        assert!(parsed.remove_from_list);
        assert_eq!(serde_json::to_value(&parsed).unwrap(), removed);
        let kept: ActionFinishedParams =
            serde_json::from_value(json!({ "flowId": "f", "removeFromList": false })).unwrap();
        assert_eq!(
            serde_json::to_value(&kept).unwrap(),
            json!({ "flowId": "f" })
        );

        // actions/start 与 step/choose。
        let start = json!({ "actionId": "members", "conversationId": "g1" });
        let parsed: ActionsStartParams = serde_json::from_value(start.clone()).unwrap();
        assert_eq!(serde_json::to_value(&parsed).unwrap(), start);
        let choose = json!({ "flowId": "f", "choice": "confirm", "selected": ["wang", "chen"] });
        let parsed: StepChooseParams = serde_json::from_value(choose.clone()).unwrap();
        assert_eq!(serde_json::to_value(&parsed).unwrap(), choose);
        let row = json!({ "flowId": "f", "choice": "remove", "row": "mac" });
        let parsed: StepChooseParams = serde_json::from_value(row.clone()).unwrap();
        assert_eq!(serde_json::to_value(&parsed).unwrap(), row);
    }

    // 契约 provider-protocol v13：三种新步骤与各步的补充原样往返；旧步骤不带新字段时写回不多出键。
    #[test]
    fn v13_steps_round_trip_and_old_steps_stay_the_same() {
        use serde_json::json;
        for shape in [
            json!({ "kind": "showText", "caption": "恢复密钥", "text": "a b c", "layout": "words", "copy": true,
                "ack": "我已保存", "confirmLabel": "完成", "noCancel": true,
                "note": { "text": "别发给任何人", "tone": "warn" } }),
            json!({ "kind": "showText", "caption": "输入下面 4 个词", "text": "w x y z", "layout": "code",
                "cancelLabel": "取消", "waiting": "等待批准…", "links": [{ "id": "useKey", "label": "改用恢复密钥" }] }),
            json!({ "kind": "list", "caption": "登录着的电脑", "rows": [
                { "id": "home", "title": "台式机", "detail": "在线", "tag": "本机", "icon": "device" },
                { "id": "mac", "title": "MacBook", "icon": "device", "actions": [{ "id": "remove", "label": "移除", "danger": true }] }
              ], "links": [{ "id": "regen", "label": "重新生成恢复密钥" }] }),
            json!({ "kind": "list", "rows": [{ "id": "wang", "title": "王小明", "icon": "person" }],
                "select": { "min": 2, "max": 9, "confirmLabel": "创建" }, "empty": "还没有好友。" }),
            json!({ "kind": "confirm", "person": { "name": "王小明", "detail": "指纹 3F2A" },
                "caption": "加王小明为好友？", "confirmLabel": "加好友" }),
            json!({ "kind": "confirm", "sections": [{ "title": "服务器看不到", "text": "消息" }],
                "confirmLabel": "知道了", "noCancel": true }),
            json!({ "kind": "confirm", "caption": "移除？", "text": "那台电脑会退出。", "confirmLabel": "移除", "danger": true }),
            json!({ "kind": "input", "fields": [{ "id": "words", "label": "4 个词", "secret": false }],
                "caption": "新电脑想登录", "submitLabel": "批准", "cancelLabel": "拒绝", "cancelChoice": "reject" }),
            json!({ "kind": "message", "text": "正在核对…", "tone": "info", "links": [{ "id": "back", "label": "返回" }] }),
        ] {
            let step: LoginStep = serde_json::from_value(shape.clone()).unwrap();
            assert_eq!(
                serde_json::to_value(&step).unwrap(),
                shape,
                "原样往返：{shape}"
            );
            assert_eq!(step_problem(&step), None, "在上限以内：{shape}");
        }
        // 多选的 min 不写按 1、写回不多出。
        let step: LoginStep = serde_json::from_value(json!({ "kind": "list", "rows": [],
            "select": { "confirmLabel": "邀请" } }))
        .unwrap();
        let LoginStep::List {
            select: Some(select),
            ..
        } = &step
        else {
            panic!("应当是多选列表")
        };
        assert_eq!(select.min, 1);
        assert_eq!(
            serde_json::to_value(&step).unwrap()["select"],
            json!({ "confirmLabel": "邀请" })
        );
        // 不认识的排法、图标读成 Unknown，不让这一步读不出。
        let step: LoginStep =
            serde_json::from_value(json!({ "kind": "showText", "caption": "c", "text": "t",
            "layout": "grid" }))
            .unwrap();
        assert!(matches!(
            step,
            LoginStep::ShowText {
                layout: TextLayout::Unknown,
                ..
            }
        ));
        let old = json!({ "kind": "input", "fields": [{ "id": "code", "label": "验证码", "secret": false }] });
        let step: LoginStep = serde_json::from_value(old.clone()).unwrap();
        assert_eq!(serde_json::to_value(&step).unwrap(), old, "旧表单不多出键");
    }

    // 契约 provider-protocol v13 §14：超出上限、编号空着或重复的一步不合要求。
    #[test]
    fn v36_login_window_round_trips_and_omits_empty_optionals() {
        use serde_json::json;
        let shape = json!({ "kind": "loginWindow", "url": "https://xd.slack.com/sso/saml/start",
            "label": "打开 Slack 登录窗口", "capture": ["slack://", "https://app.slack.com/t/"] });
        let step: LoginStep = serde_json::from_value(shape.clone()).unwrap();
        let LoginStep::LoginWindow {
            url,
            label,
            capture,
            note,
            links,
        } = &step
        else {
            panic!("{step:?}")
        };
        assert_eq!(url, "https://xd.slack.com/sso/saml/start");
        assert_eq!(label, "打开 Slack 登录窗口");
        assert_eq!(capture.len(), 2);
        assert!(note.is_none() && links.is_empty());
        assert_eq!(serde_json::to_value(&step).unwrap(), shape);
        assert_eq!(step_problem(&step), None);
        // 带提示与另一种做法也照原样往返。
        let full = json!({ "kind": "loginWindow", "url": "http://127.0.0.1:9/x", "label": "打开",
            "capture": ["hhh-sample://"], "note": { "text": "用公司账号", "tone": "info" },
            "links": [{ "id": "password", "label": "用密码登录" }] });
        let step: LoginStep = serde_json::from_value(full.clone()).unwrap();
        assert_eq!(serde_json::to_value(&step).unwrap(), full);
        // 原有步骤的写法不变。
        let open = json!({ "kind": "openUrl", "url": "https://a.test/x", "label": "打开" });
        let step: LoginStep = serde_json::from_value(open.clone()).unwrap();
        assert_eq!(serde_json::to_value(&step).unwrap(), open);
    }

    #[test]
    fn v36_bad_login_windows_are_refused_with_the_reason() {
        use crate::budget::{LOGIN_WINDOW_CAPTURES_MAX, STEP_PIECE_MAX_CHARS};
        use serde_json::json;
        let check =
            |v: serde_json::Value| step_problem(&serde_json::from_value::<LoginStep>(v).unwrap());
        let good = |capture: serde_json::Value| json!({ "kind": "loginWindow", "url": "https://a.test/s", "label": "打开", "capture": capture });
        assert_eq!(check(good(json!(["a://"]))), None);
        assert_eq!(
            check(good(json!(vec!["a://"; LOGIN_WINDOW_CAPTURES_MAX]))),
            None
        );
        for bad_url in [
            "http://a.test/s",
            "file:///C:/x",
            "javascript:alert(1)",
            "slack://x",
        ] {
            let reason = check(
                json!({ "kind": "loginWindow", "url": bad_url, "label": "打开", "capture": ["a://"] }),
            );
            assert!(reason.is_some_and(|r| r.contains("起始网址")), "{bad_url}");
        }
        for bad in [
            json!([]),
            json!(vec!["a://"; LOGIN_WINDOW_CAPTURES_MAX + 1]),
            json!(["https://a.test/"]),
            json!(["http://a.test/x"]),
            json!(["file:///"]),
            json!(["javascript://x"]),
            json!([""]),
            json!([format!("a://{}", "x".repeat(200))]),
        ] {
            let reason = check(good(bad.clone()));
            assert!(reason.is_some_and(|r| r.contains("跳转开头")), "{bad}");
        }
        assert!(check(json!({ "kind": "loginWindow", "url": "https://a.test/s", "label": " ", "capture": ["a://"] })).is_some());
        assert!(check(json!({ "kind": "loginWindow", "url": "https://a.test/s", "label": "字".repeat(STEP_PIECE_MAX_CHARS + 1), "capture": ["a://"] })).is_some());
    }

    #[test]
    fn v13_steps_over_the_limits_are_refused() {
        use crate::budget::{
            STEP_LINKS_MAX, STEP_PIECE_MAX_CHARS, STEP_ROWS_MAX, STEP_TEXT_MAX_CHARS,
        };
        use serde_json::json;
        let step = |v: serde_json::Value| serde_json::from_value::<LoginStep>(v).unwrap();
        let long = "字".repeat(STEP_PIECE_MAX_CHARS + 1);
        let at_limit = "字".repeat(STEP_PIECE_MAX_CHARS);
        let rows = |n: usize| {
            (0..n)
                .map(|i| json!({ "id": format!("r{i}"), "title": "行" }))
                .collect::<Vec<_>>()
        };
        let links = |n: usize| {
            (0..n)
                .map(|i| json!({ "id": format!("l{i}"), "label": "做法" }))
                .collect::<Vec<_>>()
        };
        for (fine, shape) in [
            (
                true,
                json!({ "kind": "showText", "caption": at_limit, "text": "x".repeat(STEP_TEXT_MAX_CHARS), "layout": "line" }),
            ),
            (
                false,
                json!({ "kind": "showText", "caption": "c", "text": "x".repeat(STEP_TEXT_MAX_CHARS + 1), "layout": "line" }),
            ),
            (
                false,
                json!({ "kind": "showText", "caption": "c", "text": "  ", "layout": "line" }),
            ),
            (
                false,
                json!({ "kind": "showText", "caption": long, "text": "t", "layout": "line" }),
            ),
            (true, json!({ "kind": "list", "rows": rows(STEP_ROWS_MAX) })),
            (
                false,
                json!({ "kind": "list", "rows": rows(STEP_ROWS_MAX + 1) }),
            ),
            (
                false,
                json!({ "kind": "list", "rows": [{ "id": "", "title": "行" }] }),
            ),
            (
                false,
                json!({ "kind": "list", "rows": [{ "id": "a", "title": "行", "actions": [
                { "id": "x", "label": "1" }, { "id": "x", "label": "2" }, { "id": "x", "label": "3" },
                { "id": "x", "label": "4" }, { "id": "x", "label": "5" }] }] }),
            ),
            (
                false,
                json!({ "kind": "list", "rows": [], "select": { "min": 3, "max": 2, "confirmLabel": "选" } }),
            ),
            (false, json!({ "kind": "confirm", "confirmLabel": " " })),
            (
                false,
                json!({ "kind": "confirm", "confirmLabel": "好", "sections": [{ "title": "t", "text": long }] }),
            ),
            (
                true,
                json!({ "kind": "message", "text": "等", "tone": "info", "links": links(STEP_LINKS_MAX) }),
            ),
            (
                false,
                json!({ "kind": "message", "text": "等", "tone": "info", "links": links(STEP_LINKS_MAX + 1) }),
            ),
            (
                false,
                json!({ "kind": "input", "fields": [], "cancelChoice": "" }),
            ),
        ] {
            assert_eq!(
                step_problem(&step(shape.clone())).is_none(),
                fine,
                "{shape}"
            );
        }
    }
}

/// stickers/packs、stickers/favorites 的分页参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StickerPageParams {
    /// 1 到 STICKER_PAGE_MAX。
    pub limit: u16,
    /// 上次返回的游标。
    #[serde(default)]
    pub cursor: Option<String>,
}
/// stickers/list 的参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StickersListParams {
    /// 包编号。
    pub pack_id: String,
    /// 分页。
    #[serde(flatten)]
    pub page: StickerPageParams,
}
/// stickers/favorite：设置目标状态，重复设置应幂等；成功回空对象。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StickersFavoriteParams {
    /// 贴纸编号。
    pub id: String,
    /// true 收藏，false 取消收藏。
    pub favorite: bool,
}
/// stickers/image：结果复用 ResourceChunk；只接受目录中图片/封面键。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StickersImageParams {
    /// 图片键。
    pub key: String,
    /// 字节偏移。
    pub offset: u64,
    /// 分段长度。
    pub length: u64,
}
