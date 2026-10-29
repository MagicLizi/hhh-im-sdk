//! 领域线上类型（protocol-v1 §7、§11）：账号、会话、人员、消息、内容块、资源、表情、发出的内容、事件。
//!
//! 全部 camelCase；时间一律 Unix **毫秒**；编号是不透明字符串（账号内稳定）。
//! `rich` 与 `card` 内容块的树体用 [`serde_json::Value`] 承载：形状按 protocol-v1 §7.6 / 附录 B
//! （mdast 子集与卡片结构），由宿主的 provider 适配器翻译成 hhh-im 的内部类型，从而内部类型怎么改
//! 都不动这里。

use serde::{Deserialize, Serialize};

/// 通过凭据核验的本人账号。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    /// 平台用户编号。
    pub id: String,
    /// 编号的种类（v14，可选），如飞书的 `open_id`、`app_id`：宿主照样记进本机的身份里，不写时记成 `ext`。
    /// 顶替内置平台的插件要写成与内置时代相同，本机旧记录里的人与新消息才认得出是同一个。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_type: Option<String>,
    /// 显示名。
    pub name: String,
    /// 头像键（交给 `avatars/read`）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    /// 账号所在的工作区 / 租户的名字（v12 起宿主显示在账号行：“账号 · 工作区”）；没有不推断、不写编号。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,
}

/// 会话类别。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConversationKind {
    /// 多人群。
    Group,
    /// 双方单聊。
    Direct,
    /// 平台频道。
    Channel,
    /// 平台未提供可判定类别。
    Unknown,
}

/// 可展示的会话。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Conversation {
    /// 平台会话编号。
    pub id: String,
    /// 类别。
    pub kind: ConversationKind,
    /// 名称，可能为空。
    pub name: String,
    /// 头像键；没有时为空。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    /// 私聊对方的用户编号；群或认不出时为空。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub peer: Option<String>,
    /// 私有频道为真；其余为假。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub private: bool,
}

/// 人员种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PersonKind {
    /// 真人。
    User,
    /// 机器人。
    Bot,
}

/// 一个人或机器人的资料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    /// 平台用户编号。
    pub id: String,
    /// 编号的种类（v14，可选），如飞书的 `open_id`、`app_id`：宿主照样记进本机的身份里，不写时记成 `ext`。
    /// 顶替内置平台的插件要写成与内置时代相同，本机旧记录里的人与新消息才认得出是同一个。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_type: Option<String>,
    /// 种类。
    pub kind: PersonKind,
    /// 名字；读不到为空。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 头像键；没有为空。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
}

/// 消息发送者。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sender {
    /// 平台身份编号；系统通知可以为空。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub id: String,
    /// 编号的种类（v14，可选），如飞书的 `open_id`、`app_id`：宿主照样记进本机的身份里，不写时记成 `ext`。
    /// 顶替内置平台的插件要写成与内置时代相同，本机旧记录里的人与新消息才认得出是同一个。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_type: Option<String>,
    /// 身份种类。
    pub kind: SenderKind,
    /// 名字。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// 发送者种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SenderKind {
    /// 真人。
    User,
    /// 机器人 / 应用。
    Bot,
    /// 系统通知（可以没有人物身份）。
    System,
}

/// 平台消息资源种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ResourceKind {
    /// 图片。
    Image,
    /// 文件、音频或视频（v1 语音/视频/音频都按 file 给，用通用格式）。
    File,
    /// 表情包：单独一条消息的那张图。
    Sticker,
}

/// 消息里的资源引用。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    /// 平台资源键（只在它所在的消息里有意义）。
    pub key: String,
    /// 下载种类。
    pub kind: ResourceKind,
    /// MIME；宿主先存着。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime: Option<String>,
    /// 总字节数；能给就给。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bytes: Option<u64>,
    /// 语音/视频时长（毫秒）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// 平台明确标记的语音消息；只对 file 有意义，普通音频不填。
    /// 宿主不得根据文件名、时长或平台编号推断语音身份。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub voice: bool,
    /// 视频的封面（provider-protocol v9）：同一条消息里一张图片的资源键，宿主按 `resources/read`
    /// （`kind: "image"`）读它；只对视频有意义，旧插件不写。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover: Option<ResourceCover>,
}

/// 视频封面（provider-protocol v9）：图片资源键与可选宽高。
///
/// # Examples
///
/// ```
/// use hhh_im_protocol::types::{Resource, ResourceCover, ResourceKind};
/// let video: Resource = serde_json::from_value(serde_json::json!({
///     "key": "v1", "kind": "file", "mime": "video/mp4", "durationMs": 3400,
///     "cover": { "key": "v1-cover", "width": 540, "height": 960 }
/// })).unwrap();
/// assert_eq!(video.kind, ResourceKind::File);
/// assert_eq!(video.cover, Some(ResourceCover { key: "v1-cover".into(), width: Some(540), height: Some(960) }));
/// // 旧插件不写封面：照常读，写回时也不出现。
/// let old: Resource = serde_json::from_value(serde_json::json!({ "key": "f1", "kind": "file" })).unwrap();
/// assert_eq!(old.cover, None);
/// assert!(serde_json::to_value(&old).unwrap().get("cover").is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCover {
    /// 封面图片的资源键（只在它所在的消息里有意义）。
    pub key: String,
    /// 封面宽（像素）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    /// 封面高（像素）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
}

/// 一条消息里的一个内容块（protocol-v1 §7.5），`type` 分辨。不认识的 `type` 宿主按 `unsupported` 显示。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Block {
    /// 纯文字。
    Text {
        /// 原文，不含任何标记。
        text: String,
    },
    /// 经平台结构确认的提及。
    Mention {
        /// 被提及用户编号；提及所有人时为空。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        user_id: Option<String>,
        /// 编号的种类（v14，可选），如飞书的 `open_id`、`app_id`：宿主照样记进本机的身份里，不写时记成 `ext`。
        /// 顶替内置平台的插件要写成与内置时代相同，本机旧记录里的人与新消息才认得出是同一个。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id_type: Option<String>,
        /// 被提及的是人还是机器人（v14，可选）；不写按人。宿主照它显示名字：机器人先用这里的名字，人先用人名册的。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        kind: Option<PersonKind>,
        /// 名字。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        /// 是否提及所有人。
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        everyone: bool,
    },
    /// 图片、文件、表情包。
    Resource {
        /// 资源引用。
        resource: Resource,
        /// 文件名。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        /// 图片宽（像素）。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        width: Option<u32>,
        /// 图片高（像素）。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        height: Option<u32>,
    },
    /// 格式正文：mdast 子集语法树（protocol-v1 §7.6）；树体由适配器翻译。
    Rich {
        /// `{"type":"root","children":[...]}`。
        root: serde_json::Value,
    },
    /// 平台明确标为 Markdown 的原文，由宿主解析。
    Markdown {
        /// Markdown 原文。
        text: String,
    },
    /// 卡片（protocol-v1 附录 B）；结构由适配器翻译。
    Card {
        /// 卡片结构（标题、色调、块）。
        card: serde_json::Value,
    },
    /// 一个原文链接的分享卡片；整张卡片打开 url，封面不作为独立图片打开。
    LinkCard {
        /// HTTP/HTTPS 原文地址。
        url: String,
        /// 纯文本标题；没有时宿主显示地址的主机名。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        /// 纯文本摘要。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        /// 来源显示名。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        source: Option<String>,
        /// 属于这条消息的图片资源；沿用 resources/read，缺失不影响链接。
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "crate::link_card::thumbnail"
        )]
        thumbnail: Option<Resource>,
    },
    /// 合并转发的只读快照；资源键仍属于外层消息，不把内部记录当成会话消息。
    Forwarded {
        /// 标题、消息顺序与原发送者的显示快照。
        record: Box<ForwardedRecord>,
    },
    /// 暂时翻译不了的内容（出现它时整条消息要标 `incomplete`）。
    Unsupported {
        /// 平台内容种类。
        kind: String,
        /// 平台里这类消息的叫法。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
    },
}

/// 转发记录中的发送者显示快照，不要求原会话或当前成员身份可访问。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ForwardedSender {
    /// 可知的原发送者编号；没有时不伪造。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// 编号种类。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_type: Option<String>,
    /// 平台给出的发送者种类。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<SenderKind>,
    /// 转发时保留的名字，不用当前群成员名字覆盖。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 可经 avatars/read 读取的头像键；没有时显示名字占位。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
}

/// 转发记录内部的一条快照，没有当前会话的操作、未读或话题语义。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForwardedMessage {
    /// 原消息编号，仅供识别快照，不能直接拿它请求 messages/get。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// 原发送者的显示快照。
    #[serde(default)]
    pub sender: ForwardedSender,
    /// 可知的原发送时间（毫秒）；未知时不拿外层时间代替。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at_ms: Option<i64>,
    /// 按顺序排列的内容；资源键由插件在整个外层消息内唯一化。
    pub content: Vec<Block>,
    /// 这条快照是否有无法读取的部分。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub incomplete: bool,
}

/// 合并转发的聊天记录。完整性、数量与树形预算见 forwarded 模块。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ForwardedRecord {
    /// 平台提供的标题；空标题由界面显示“聊天记录”。
    #[serde(default)]
    pub title: String,
    /// 转发时的原始顺序，不按时间重排。
    #[serde(default)]
    pub messages: Vec<ForwardedMessage>,
    /// 平台确认的总数；未知时省略，不能以当前可读条数冒充。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_count: Option<u32>,
    /// 部分内容未提供、不可读或超过预算。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub incomplete: bool,
}

/// 话题根消息上的话题现状。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadActivity {
    /// 最后一条回复编号。
    pub latest_reply_id: String,
    /// 最后一条回复时间（毫秒）。
    pub latest_reply_at_ms: i64,
    /// 回复数。
    pub reply_count: u32,
}

/// 消息的话题信息（平台没有话题功能时整个省略）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadInfo {
    /// 平台话题编号。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// 回复树根消息编号。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_id: Option<String>,
    /// 平台明说它是不是话题里的回复；没说为空。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub in_thread: Option<bool>,
    /// 话题回复同时发到了会话消息流。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub broadcast: bool,
    /// 根消息上的话题现状。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activity: Option<ThreadActivity>,
    /// 这条是“话题回复与它在会话里的那份”中的哪一边、另一边是哪条（契约 provider-protocol v12 缺口 13）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy: Option<ThreadCopy>,
}

/// 话题回复与它在会话消息流里那份副本的配对（飞书客户端“同时发送到群”的两条）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadCopy {
    /// 这一条是哪一边。
    pub role: ThreadCopyRole,
    /// 另一边那条的消息编号。
    pub peer_id: String,
    /// 平台的话题编号（会话里那条带着，用来找话题第一条）；不知道时为空。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
}

/// [`ThreadCopy`] 的哪一边。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ThreadCopyRole {
    /// 会话消息流里的那条。
    Conversation,
    /// 话题里的那条（原本的回复）。
    Thread,
}

/// 去凭据化的原始数据（声明 `retranslate` 能力时随消息带）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Raw {
    /// 程序自定的格式名（如 `wechat-addmsg-v1`）。
    pub format: String,
    /// 去凭据化的原始数据（字符串或 base64）；不含令牌、密钥、会话材料。
    pub data: String,
}

/// 一条表情回复。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reaction {
    /// 平台表情编号。
    pub id: String,
    /// 所属会话。
    pub conversation_id: String,
    /// 所属消息。
    pub message_id: String,
    /// 点表情的人。
    pub operator: Person,
    /// 表情类型。
    pub emoji_type: String,
    /// 操作时间（毫秒）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at_ms: Option<i64>,
}

/// 保留发送者、引用、话题关系的统一消息（protocol-v1 §7.4）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    /// 同一会话／话题内的精确阅读序数，规范十进制字符串；未知时省略。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_position: Option<String>,
    /// 伙伴代发来源的显示快照；无效的附注不影响正文与同批消息。
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::attribution::receive"
    )]
    pub agent_attribution: Option<Box<crate::AgentAttribution>>,
    /// 平台消息编号。
    pub id: String,
    /// 所属会话。
    pub conversation_id: String,
    /// 发送者。
    pub sender: Sender,
    /// 创建时间（毫秒）。
    pub created_at_ms: i64,
    /// 更新时间（毫秒）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at_ms: Option<i64>,
    /// 编辑时间（毫秒）；只在平台明说改过时给。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edited_at_ms: Option<i64>,
    /// 是否已撤回（为真时 content 为空）。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub deleted: bool,
    /// 是否有翻译不了的内容。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub incomplete: bool,
    /// 内容块。
    pub content: Vec<Block>,
    /// 被回复的消息编号。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
    /// 话题信息。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread: Option<ThreadInfo>,
    /// 这条消息此刻的全部表情；不知道全部就省略。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reactions: Option<Vec<Reaction>>,
    /// 去凭据化的原始数据（声明 retranslate 时给）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw: Option<Raw>,
    /// 本人能编辑这条时，编辑框里的原文（表情、链接按平台的写法，契约 provider-protocol v12 缺口 12）；不能编辑就不带。
    /// 只对声明了 `actions.edit.perMessage` 的插件有意义。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edit_draft: Option<Vec<Outgoing>>,
}

/// 要发出的一段内容（protocol-v1 §7.8）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Outgoing {
    /// 文字。
    Text {
        /// 原文。
        text: String,
    },
    /// 提及某人。
    Mention {
        /// 用户编号。
        user_id: String,
        /// 名字。
        name: String,
    },
    /// 提及所有人。
    MentionAll,
    /// 图片（`key` 来自 uploads/finish）。
    Image {
        /// 资源键。
        key: String,
    },
    /// 目录中的原生贴纸；独立一条，不与文字、附件混发。
    Sticker {
        /// 当前账号内插件给出的贴纸编号。
        id: String,
    },
    /// 文件（`key` 来自 uploads/finish）。
    File {
        /// 资源键。
        key: String,
    },
    /// 格式正文（契约 provider-protocol v40）：`{"type":"root","children":[...]}`，节点同收消息的 [`Block::Rich`]
    /// （protocol-v1 §7.6），用 [`crate::content::rich_outgoing`] 搭、[`crate::content::rich_nodes`] 读。
    /// 宿主只交给在 `capabilities.richSend` 里声明了格式的插件，是这一条的全部内容，树里只有声明过的格式
    /// （段落、文字、换行、提及、表情总可以有）；提及写成 `{"type":"mention","userId":…,"name":…}` 或 `everyone`。
    Rich {
        /// 语法树的根。
        root: serde_json::Value,
    },
}

/// 发送目标（protocol-v1 §10）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SendTarget {
    /// 发到会话。
    Conversation {
        /// 会话编号。
        conversation_id: String,
    },
    /// 回复某条消息。
    Reply {
        /// 会话编号。
        conversation_id: String,
        /// 被回复消息编号。
        message_id: String,
        /// 进原生话题。
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        in_thread: bool,
        /// 话题根消息编号（平台只收根消息的用它）。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        thread_root: Option<String>,
        /// 同时发到会话。
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        also_to_conversation: bool,
    },
}

/// 发送结果（protocol-v1 §10）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SendOutcome {
    /// 平台接受了，给出平台记下的这条消息。
    Accepted {
        /// 平台记下的消息。
        message: Box<Message>,
    },
    /// 拿不准平台有没有收到；宿主不会自动重发。
    Unknown {
        /// 原因。
        reason: String,
    },
}

/// 表情目录里的一个表情。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Emoji {
    /// 表情类型（交给 emoji/image 取图）。
    pub emoji_type: String,
    /// 中文/本地名。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 是否进选择面板。
    #[serde(default = "crate::types::yes")]
    pub panel: bool,
    /// Unicode 码位（标准表情）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unicode: Option<String>,
    /// 分类编号。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// 别名。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    /// 肤色版本的肤色名字（如“深色”；v14）；不是肤色版本时不写。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tone: Option<String>,
    /// 肤色版本：它是哪个表情的哪种肤色（v14）；面板按它把肤色版本收到原表情下面。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tone_of: Option<ToneOf>,
    /// 能不能按平台说明的 `emoji.textTemplate` 写进文字（v31）：假时输入框的表情面板不列它。与 [`Self::panel`]
    /// （能不能点成回应）互相独立。不写为真。
    #[serde(
        default = "crate::types::yes",
        skip_serializing_if = "crate::types::is_yes"
    )]
    pub in_text: bool,
}

/// 肤色版本对应的原表情与肤色。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToneOf {
    /// 原表情的类型。
    pub base: String,
    /// 肤色编号（目录 `tones` 里的）。
    pub tone: String,
}

/// 表情目录里的一种肤色。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmojiTone {
    /// 编号。
    pub id: String,
    /// 名字。
    pub name: String,
}

pub(crate) fn yes() -> bool {
    true
}

pub(crate) fn is_yes(value: &bool) -> bool {
    *value
}

/// 表情分类。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmojiCategory {
    /// 编号。
    pub id: String,
    /// 名字。
    pub name: String,
    /// 分类标签画哪个图标（v31），取 [`EMOJI_CATEGORY_ICONS`] 之一；不写或不认识时宿主画圆。分类编号本身不决定图标。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

/// 宿主图标库里分类标签能用的图标（v31）：笑脸、自然、食物、出行、活动、物品、符号、旗帜、自定义。
pub const EMOJI_CATEGORY_ICONS: [&str; 9] = [
    "smileys",
    "nature",
    "food",
    "travel",
    "activities",
    "objects",
    "symbols",
    "flags",
    "custom",
];

/// 推送事件（protocol-v1 §11.2），`type` 分辨。宿主遇到不认识的事件记日志、跳过、仍确认整批。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Event {
    /// 新到或变了的消息（含编辑、撤回成标记、话题根变化）。
    Messages {
        /// 会话编号。
        conversation_id: String,
        /// 消息。
        messages: Vec<Message>,
    },
    /// 消息从平台消失了（deletedMessagesVanish 的平台）。
    Deleted {
        /// 会话编号。
        conversation_id: String,
        /// 消息编号。
        message_id: String,
    },
    /// 这条消息此刻的全部表情。
    Reactions {
        /// 会话编号。
        conversation_id: String,
        /// 消息编号。
        message_id: String,
        /// 全部表情。
        reactions: Vec<Reaction>,
    },
    /// 单个表情增加。
    ReactionAdded {
        /// 表情。
        reaction: Reaction,
    },
    /// 单个表情取消。
    ReactionRemoved {
        /// 会话编号。
        conversation_id: String,
        /// 消息编号。
        message_id: String,
        /// 表情编号。
        reaction_id: String,
    },
    /// 新出现的会话，或资料变了。
    Conversation {
        /// 会话。
        conversation: Conversation,
    },
    /// 会话在平台没有了（删好友、退群）。宿主只记状态，不删本地。
    ConversationRemoved {
        /// 会话编号。
        conversation_id: String,
    },
    /// 有人名字或头像变了。
    People {
        /// 人员。
        people: Vec<Person>,
    },
}

#[cfg(test)]
mod tests {

    // 契约 provider-protocol v31：表情能不能写进文字、分类图标都可选；不写时 inText 读成真、写回不多出键。
    #[test]
    fn v31_in_text_defaults_to_true_and_category_icons_are_optional() {
        use super::{Emoji, EmojiCategory};
        let old = serde_json::json!({ "emojiType": "OK", "name": "OK", "panel": true });
        let emoji: Emoji = serde_json::from_value(old.clone()).unwrap();
        assert!(emoji.in_text);
        assert_eq!(
            serde_json::to_value(&emoji).unwrap(),
            old,
            "旧形状写回不多出键"
        );
        let only_reaction = serde_json::json!({ "emojiType": "2021", "name": "2021年", "panel": true, "inText": false });
        let emoji: Emoji = serde_json::from_value(only_reaction.clone()).unwrap();
        assert!(
            emoji.panel && !emoji.in_text,
            "能点成回应、不能写进文字，两个标记互相独立"
        );
        assert_eq!(serde_json::to_value(&emoji).unwrap(), only_reaction);

        let plain: EmojiCategory =
            serde_json::from_value(serde_json::json!({ "id": "people", "name": "人物" })).unwrap();
        assert!(plain.icon.is_none());
        let with_icon = serde_json::json!({ "id": "people", "name": "人物", "icon": "smileys" });
        let category: EmojiCategory = serde_json::from_value(with_icon.clone()).unwrap();
        assert_eq!(category.icon.as_deref(), Some("smileys"));
        assert_eq!(serde_json::to_value(&category).unwrap(), with_icon);
    }

    // 契约 provider-protocol v14：账号与提及可带编号种类，提及还可带人还是机器人；不写时写回不多出键。
    #[test]
    fn v14_identities_carry_an_optional_id_type_and_mentions_a_kind() {
        let mention = serde_json::json!({ "type": "mention", "userId": "cli_1", "idType": "app_id", "kind": "bot", "name": "日报机器人" });
        let block: Block = serde_json::from_value(mention.clone()).unwrap();
        assert_eq!(serde_json::to_value(&block).unwrap(), mention);
        let old = serde_json::json!({ "type": "mention", "userId": "ou_1", "name": "小李" });
        let block: Block = serde_json::from_value(old.clone()).unwrap();
        assert!(matches!(
            block,
            Block::Mention {
                id_type: None,
                kind: None,
                ..
            }
        ));
        assert_eq!(
            serde_json::to_value(&block).unwrap(),
            old,
            "旧形状写回不多出键"
        );
        let account = serde_json::json!({ "id": "ou_1", "idType": "open_id", "name": "我", "tenant": "示例团队" });
        let parsed: Account = serde_json::from_value(account.clone()).unwrap();
        assert_eq!(serde_json::to_value(&parsed).unwrap(), account);
    }
    use super::*;

    #[test]
    fn a_text_block_round_trips_with_a_type_tag() {
        let b = Block::Text {
            text: "你好".into(),
        };
        let v = serde_json::to_value(&b).unwrap();
        assert_eq!(v["type"], "text");
        assert_eq!(v["text"], "你好");
        let back: Block = serde_json::from_value(v).unwrap();
        assert_eq!(back, b);
    }

    #[test]
    fn a_message_omits_empty_optionals() {
        let m = Message {
            read_position: None,
            agent_attribution: None,
            id: "m1".into(),
            conversation_id: "c1".into(),
            sender: Sender {
                id: "u1".into(),
                id_type: None,
                kind: SenderKind::User,
                name: None,
            },
            created_at_ms: 1_700_000_000_000,
            updated_at_ms: None,
            edited_at_ms: None,
            deleted: false,
            incomplete: false,
            content: vec![Block::Text { text: "hi".into() }],
            reply_to: None,
            thread: None,
            reactions: None,
            raw: None,
            edit_draft: None,
        };
        let v = serde_json::to_value(&m).unwrap();
        assert_eq!(v["id"], "m1");
        assert_eq!(v["createdAtMs"], 1_700_000_000_000_i64);
        assert!(v.get("deleted").is_none(), "false 不序列化");
        assert!(v.get("raw").is_none());
        assert!(v.get("thread").is_none());
    }

    #[test]
    fn send_outcome_is_tagged_by_state() {
        let acc = SendOutcome::Accepted {
            message: Box::new(Message {
                read_position: None,
                agent_attribution: None,
                id: "m".into(),
                conversation_id: "c".into(),
                sender: Sender {
                    id: "me".into(),
                    id_type: None,
                    kind: SenderKind::User,
                    name: None,
                },
                created_at_ms: 1,
                updated_at_ms: None,
                edited_at_ms: None,
                deleted: false,
                incomplete: false,
                content: vec![],
                reply_to: None,
                thread: None,
                reactions: None,
                raw: None,
                edit_draft: None,
            }),
        };
        assert_eq!(serde_json::to_value(&acc).unwrap()["state"], "accepted");
        let unk = SendOutcome::Unknown {
            reason: "超时".into(),
        };
        assert_eq!(serde_json::to_value(&unk).unwrap()["state"], "unknown");
    }
}
