//! 平台说明、能力清单、设置项、插件清单（protocol-v1 §2、§5、§6.4）。

use serde::{Deserialize, Serialize};

/// 插件包清单 `hhh-im-provider.json`（protocol-v1 §2）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    /// 清单格式版本，当前 1。
    pub manifest_version: u32,
    /// 程序编号：小写字母开头，`a-z0-9-`，2–32 字符。内部平台编号为 `ext-<id>`。
    pub id: String,
    /// 给用户看的平台名，1–20 字符。
    pub name: String,
    /// 程序版本（semver）。
    pub version: String,
    /// 启动命令（相对清单目录，或 PATH 里的程序）。
    pub command: String,
    /// 启动参数。
    #[serde(default)]
    pub args: Vec<String>,
    /// 支持的系统：windows / macos / linux。
    pub os: Vec<String>,
    /// logo 文件（相对清单目录），SVG/PNG ≤256 KiB；没有就用名字首字。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logo: Option<String>,
    /// 发布者。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
    /// 主页。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    /// IM 类型（可选）：同一种 IM 在宿主里同一时间只用一个。内置飞书是 `feishu`、内置 Slack 是 `slack`，
    /// 写成同一个值就和它们是同一种；不写时自成一类。写法同编号（见 [`Self::id_ok`]）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub im_type: Option<String>,
    /// 这个安装包给哪种电脑（[`crate::release::Target`] 的名字，契约 provider-protocol v10 §4.2）。从安装包安装时必须有、
    /// 且必须是本机的目标；从文件夹安装（开发用）时可以不写，照旧按 `os` 判断。打包工具写进包里的清单。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// 更新来源：宿主只从这里检查、下载新版本；不写就没有更新来源。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<crate::release::Source>,
    /// 宿主需要的最低 IM SDK 版本（`主.次.修订`，见 [`crate::SDK_VERSION`]）：打包工具按插件编进去的 SDK 自动写进包里的
    /// 清单；宿主的 SDK 低于它就不装、不启动。不写的（老版本、开发时直接用文件夹装的）按兼容处理。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_host: Option<String>,
    /// 标志的画法（契约 provider-protocol v12 缺口 10）：`mark` 是透明底的标志图形、宿主垫自己的底；`icon`（默认）是
    /// 自带底色的完整应用图标、原样铺满。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logo_style: Option<LogoStyle>,
    /// `mark` 画法时图形在画布里的外框（SVG 的 viewBox 坐标），宿主据此核对标志的视觉尺寸；不写按整张画布。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logo_ink: Option<LogoInk>,
}

/// 标志的画法（清单 `logoStyle`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LogoStyle {
    /// 透明底的标志图形：宿主垫上自己的底。
    Mark,
    /// 自带底色的完整应用图标：原样铺满标志位。
    Icon,
}

/// 标志图形在自己画布里的外框（viewBox 坐标）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LogoInk {
    /// 左上角横坐标。
    pub x: f64,
    /// 左上角纵坐标。
    pub y: f64,
    /// 宽。
    pub width: f64,
    /// 高。
    pub height: f64,
}

// 外框只用来核对，合法的外框不会是 NaN（`Manifest::logo_ok` 拒绝非有限值）。
impl Eq for LogoInk {}

impl Manifest {
    /// 本轮新加的发布字段是否合规（契约 provider-protocol v10 §4.2）：目标认得出、来源写法对、最低宿主版本是三段数字、
    /// 版本号是三段数字（有来源的插件要比版本大小）。旧清单都不写这些，照常合规。
    ///
    /// # Examples
    ///
    /// ```
    /// let mut manifest: hhh_im_protocol::Manifest = serde_json::from_str(
    ///     r#"{"manifestVersion":1,"id":"wechat-pad","name":"微信","version":"0.4.0","command":"p","os":["windows"],
    ///        "target":"windows-x64","source":{"type":"github","repo":"someone/hhh-wechat"},"minHost":"0.1.0"}"#,
    /// ).unwrap();
    /// assert!(manifest.release_fields_ok().is_ok());
    /// manifest.target = Some("windows-arm64".into());
    /// assert!(manifest.release_fields_ok().is_err());
    /// ```
    pub fn release_fields_ok(&self) -> Result<(), String> {
        if let Some(target) = &self.target {
            let Some(parsed) = crate::release::Target::parse(target) else {
                return Err(format!("插件清单的目标 {target} 认不出。"));
            };
            if !self.os.iter().any(|os| os == parsed.os()) {
                return Err(format!("插件清单的目标 {target} 和支持的系统对不上。"));
            }
        }
        if let Some(source) = &self.source {
            source.check()?;
            if crate::release::Version::parse(&self.version).is_none() {
                return Err(format!(
                    "有更新来源的插件，版本号要写成 主.次.修订（现在是 {}）。",
                    self.version
                ));
            }
        }
        if self
            .min_host
            .as_deref()
            .is_some_and(|min| crate::release::Version::parse(min).is_none())
        {
            return Err("插件清单的最低 HHH 版本（minHost）写法不对。".into());
        }
        Ok(())
    }

    /// 编号是否合法：小写字母开头，只含 `a-z0-9-`，2–32 字符。
    ///
    /// # Examples
    ///
    /// ```
    /// assert!(hhh_im_protocol::platform::Manifest::id_ok("wechat-pad"));
    /// assert!(!hhh_im_protocol::platform::Manifest::id_ok("1bad"));
    /// assert!(!hhh_im_protocol::platform::Manifest::id_ok("a"));
    /// ```
    pub fn id_ok(id: &str) -> bool {
        let bytes = id.as_bytes();
        (2..=32).contains(&id.len())
            && bytes[0].is_ascii_lowercase()
            && bytes
                .iter()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'-')
    }

    /// 清单里的 IM 类型是否合规：没写，或写法同编号（[`Self::id_ok`]）。
    ///
    /// # Examples
    ///
    /// ```
    /// let mut manifest: hhh_im_protocol::Manifest = serde_json::from_str(
    ///     r#"{"manifestVersion":1,"id":"feishu-pro","name":"飞书 Pro","version":"0.1.0","command":"p","os":["windows"],"imType":"feishu"}"#,
    /// ).unwrap();
    /// assert!(manifest.im_type_ok());
    /// manifest.im_type = Some("Feishu".into());
    /// assert!(!manifest.im_type_ok());
    /// ```
    pub fn im_type_ok(&self) -> bool {
        self.im_type.as_deref().is_none_or(Self::id_ok)
    }

    /// 标志的外框是否合规：没写，或四个数都是有限数、宽高大于 0（契约 provider-protocol v12 缺口 10）。
    ///
    /// # Examples
    ///
    /// ```
    /// let mut manifest: hhh_im_protocol::Manifest = serde_json::from_str(
    ///     r#"{"manifestVersion":1,"id":"sample","name":"示例","version":"0.1.0","command":"p","os":["windows"],
    ///        "logoStyle":"mark","logoInk":{"x":2,"y":2,"width":20,"height":20}}"#,
    /// ).unwrap();
    /// assert!(manifest.logo_ok());
    /// manifest.logo_ink = Some(hhh_im_protocol::platform::LogoInk { x: 0.0, y: 0.0, width: 0.0, height: 4.0 });
    /// assert!(!manifest.logo_ok());
    /// ```
    pub fn logo_ok(&self) -> bool {
        self.logo_ink.is_none_or(|ink| {
            [ink.x, ink.y, ink.width, ink.height]
                .iter()
                .all(|v| v.is_finite())
                && ink.width > 0.0
                && ink.height > 0.0
        })
    }
}

/// 头像形状。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AvatarShape {
    /// 圆形。
    Circle,
    /// 圆角方形。
    Rounded,
}

/// 平台历史能力：决定宿主要不要定时拉、能不能往上翻历史（protocol-v1 §5.1、§11.4）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HistoryMode {
    /// 消息随时能向平台重读（飞书/Slack 这类）。
    Server,
    /// 平台只推一次，不能翻历史（微信这类）：所有会话都存进 HHH，游标由宿主存。
    None,
}

/// 每秒 / 每分钟上限。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RateBudget {
    /// 每秒最多几次。
    pub per_second: u32,
    /// 每分钟最多几次。
    pub per_minute: u32,
}

/// 发送附件的上限。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Limits {
    /// 发送图片最大字节。
    pub image_upload_max_bytes: u64,
    /// 发送文件最大字节。
    pub file_upload_max_bytes: u64,
    /// 附件文件名最长字节（UTF-8）。
    pub attachment_name_max_bytes: u32,
}

/// 会话的叫法。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationNouns {
    /// 群。
    pub group: String,
    /// 私聊。
    pub direct: String,
    /// 频道。
    pub channel: String,
    /// 认不出类别。
    pub unknown: String,
}

/// 读取间隔默认值（秒）；字段同 hhh-im `ReadIntervals`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReadIntervalsSecs {
    /// 正在看。
    pub watching: u64,
    /// 1 小时内有动静。
    pub active: u64,
    /// 1 天内有动静。
    pub quiet: u64,
    /// 1 周内有动静。
    pub dormant: u64,
    /// 更早。
    pub stale: u64,
    /// 会话资料与成员刷新。
    pub profile: u64,
    /// 人名与头像过期。
    pub people: u64,
    /// 话题跟踪天数。
    pub thread_days: u32,
    /// 会话发现间隔（契约 provider-protocol v12 缺口 1）；声明了 `discovery` 才有用，不写按宿主默认。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discover: Option<u64>,
}

/// 会话发现参数（契约 provider-protocol v12 缺口 1）：插件能提供账号级活动查询，或能列出会话并按时间读消息供宿主逐会话限频探测时给。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryDesc {
    /// 会话发现间隔最快可设多少秒（由平台这个查询的限额决定）。
    pub fastest_secs: u64,
    /// 平时一次检查最多翻几页。
    pub pages: u16,
    /// 冷启动（补最近 1 天）一次最多翻几页。
    pub cold_pages: u16,
}

/// 开放给用户在「参数配置 → 读取与下载」里改的一项（契约 provider-protocol v12 缺口 2、11）。
/// 叫法、单位、控件由宿主统一；宿主不认识的取值读成 [`Adjustable::Unknown`]、不画。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Adjustable {
    /// 读取频率各档、资料与人名、话题天数、会话发现间隔一组。
    ReadIntervals,
    /// 按会话读一页最多几条。
    PageMax,
    /// 每秒最多读几次。
    ReadsPerSecond,
    /// 每分钟最多读几次。
    ReadsPerMinute,
    /// 每秒最多下载几次。
    DownloadsPerSecond,
    /// 每分钟最多下载几次。
    DownloadsPerMinute,
    /// 文件按多大一段下载。
    FileSegmentBytes,
    /// 宿主不认识的取值（新宿主才有的项）：忽略。
    #[serde(other)]
    Unknown,
}

/// 组成消息的一种内容块（`editsByContent` 用，契约 provider-protocol v12 缺口 3）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContentKind {
    /// 文字块。
    Text,
    /// 提及块。
    Mention,
    /// 格式正文块。
    Rich,
    /// 宿主不认识的取值：忽略。
    #[serde(other)]
    Unknown,
}

/// 一条重译规则（契约 provider-protocol v12 缺口 4）：插件换了翻译规则时列一个新名字，宿主按名字把存着原始数据的旧消息
/// 交给 `messages/retranslate` 一次。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetranslateRule {
    /// 规则名（插件自己定，同一个插件里不重复）。
    pub name: String,
    /// 只把 `raw.format` 在其中的旧消息交给它；空 = 全部存着原始数据的。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub formats: Vec<String>,
}

/// 同时发送到会话的能力。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Broadcast {
    /// 平台原生支持（一条两处都有）；为假时内核发两次。
    pub native: bool,
    /// 带附件的回复也能同时发送。
    pub attachments: bool,
}

/// 话题相关平台事实。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Threads {
    /// 读会话时根消息带话题现状。
    pub activity_on_root: bool,
    /// 按话题读能只要某时间之后的回复。
    pub reads_after: bool,
    /// 同时发送到会话的能力；不支持为空。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub broadcast: Option<Broadcast>,
}

/// 每条消息允许哪些操作的声明式规则（protocol-v1 §5.1）。宿主用通用代码执行。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ActionRules {
    /// 能在会话里引用回复。
    #[serde(default)]
    pub reply: bool,
    /// 能回复/新建话题。
    #[serde(default)]
    pub thread: bool,
    /// 能点表情。
    #[serde(default)]
    pub react: bool,
    /// 能撤回；null 表示不能。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recall: Option<OwnWithin>,
    /// 能编辑；null 表示不能。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edit: Option<EditRule>,
}

/// 编辑的规则：同 [`OwnWithin`]，另可声明每条消息自己说能不能编辑（契约 provider-protocol v12 缺口 12）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EditRule {
    /// 只能对本人发的消息做。
    #[serde(default)]
    pub own: bool,
    /// 平台时限（秒）；没有时限为空。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub within_secs: Option<u64>,
    /// 为真时只有带 `editDraft` 的消息能编辑，编辑框用它；为假时宿主按通用规则（文字与提及）。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub per_message: bool,
}

/// “只对本人发的、在时限内”这类操作约束。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnWithin {
    /// 只能对本人发的消息做。
    #[serde(default)]
    pub own: bool,
    /// 平台时限（秒）；过了宿主不再显示这个操作，最终以平台答复为准。没有时限为空。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub within_secs: Option<u64>,
}

/// 目录里没有的表情怎么显示。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnlistedEmoji {
    /// 画成占位图标的表情类型（正则，JavaScript 语法）；为空时都写类型名。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placeholder_pattern: Option<String>,
    /// 占位图标的叫法。
    pub placeholder_label: String,
    /// 目录外的表情类型也能问 `emoji/image` 要图：插件按自己的规律猜，猜不到回 `notFound`（契约 provider-protocol v12 缺口 8）。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub guess: bool,
    /// 声明了 `guess` 时只拿这些类型去问（正则，JavaScript 语法；v14）：猜不出的（如飞书企业表情的纯数字编号）
    /// 不必每次都问一遍。不写时目录外的都问。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guess_pattern: Option<String>,
}

/// 表情规则（有表情能力时给）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmojiRules {
    /// 表情编号合法的正则（JavaScript 语法）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_pattern: Option<String>,
    /// 目录外的表情怎么显示。
    pub unlisted: UnlistedEmoji,
    /// 正文里表情的写法（JavaScript 正则，如 `\[([^\[\]\r\n]{1,12})\]`）：宿主把正文里这样写的换成表情图；
    /// 不写就不换（契约 provider-protocol v12 缺口 9）。第一个括号认出的是 [`Self::text_template`] 里占位的那一项：
    /// 写法用 `{type}` 时是表情编号，用 `{name}` 或没声明写法时是名字（v31）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_pattern: Option<String>,
    /// 表情图是白底的：宿主显示前抠成透明（深色主题下不露白底）。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub white_background: bool,
    /// 表情写进文字的写法（v31）：恰好一个占位 `{name}`（目录里的名字）或 `{type}`（表情编号），如 `[{name}]`、
    /// `:{type}:`；宿主据此把面板里点的表情插进输入框。不写就没有输入框的表情按钮。格式见 [`TextTemplate::parse`]。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_template: Option<String>,
}

/// 写法里的占位：填名字还是填编号（v31）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateSlot {
    /// `{name}`：目录里的名字。
    Name,
    /// `{type}`：表情编号。
    Type,
}

/// 读懂了的“表情写进文字的写法”（[`EmojiRules::text_template`]，v31）：占位前后的文字与占位填什么。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextTemplate {
    prefix: String,
    slot: TemplateSlot,
    suffix: String,
}

impl TextTemplate {
    /// 按协议的规则读写法：恰好一个占位（`{name}` 或 `{type}`），不含换行与控制字符，去掉占位后至多
    /// [`crate::budget::TEXT_TEMPLATE_AFFIX_MAX_CHARS`] 个字符。不合要求时给出原因。
    ///
    /// # Examples
    ///
    /// ```
    /// use hhh_im_protocol::platform::{TemplateSlot, TextTemplate};
    /// let feishu = TextTemplate::parse("[{name}]").unwrap();
    /// assert_eq!(feishu.slot(), TemplateSlot::Name);
    /// assert_eq!(feishu.write("赞"), "[赞]");
    /// assert_eq!(TextTemplate::parse(":{type}:").unwrap().write("wave::skin-tone-3"), ":wave::skin-tone-3:");
    /// assert!(TextTemplate::parse("[{name}{type}]").is_err());
    /// assert!(TextTemplate::parse("[赞]").is_err());
    /// ```
    pub fn parse(template: &str) -> Result<Self, String> {
        if template.chars().any(char::is_control) {
            return Err("写法里不能有换行或控制字符".into());
        }
        let found: Vec<(usize, TemplateSlot, &str)> = [
            ("{name}", TemplateSlot::Name),
            ("{type}", TemplateSlot::Type),
        ]
        .into_iter()
        .flat_map(|(mark, slot)| {
            template
                .match_indices(mark)
                .map(move |(at, mark)| (at, slot, mark))
        })
        .collect();
        let [(at, slot, mark)] = found[..] else {
            return Err("写法要恰好有一个占位：{name} 或 {type}".into());
        };
        let (prefix, suffix) = (&template[..at], &template[at + mark.len()..]);
        if prefix.chars().count() + suffix.chars().count()
            > crate::budget::TEXT_TEMPLATE_AFFIX_MAX_CHARS
        {
            return Err(format!(
                "去掉占位后至多 {} 个字符",
                crate::budget::TEXT_TEMPLATE_AFFIX_MAX_CHARS
            ));
        }
        Ok(Self {
            prefix: prefix.to_owned(),
            slot,
            suffix: suffix.to_owned(),
        })
    }

    /// 占位填名字还是编号。
    pub fn slot(&self) -> TemplateSlot {
        self.slot
    }

    /// 把名字或编号（按 [`Self::slot`]）写成文字。
    pub fn write(&self, value: &str) -> String {
        format!("{}{value}{}", self.prefix, self.suffix)
    }
}

/// 头像键里的身份（v14，可选）：同一个头像每次给的键不一样（如换了 CDN 主机）时，宿主靠它认出没变——资料不算变了、
/// 头像不重下。两个写法都是 Rust `regex-lite` 的正则（不支持环视与反向引用）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvatarIdentity {
    /// 第一个括号是头像编号；对不上时整个键就是身份。
    pub key_pattern: String,
    /// 第一个括号是尺寸这类变体（同一个头像的不同尺寸各下一份）；不写或对不上时没有变体。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant_pattern: Option<String>,
}

/// 平台说明（protocol-v1 §5.1）：宿主据此限速、分页、显示界面、算每条消息能做什么。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformDesc {
    /// 头像形状（必填）。
    pub avatar_shape: AvatarShape,
    /// 头像键里的身份（v14）；不写时整个键就是身份。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_identity: Option<AvatarIdentity>,
    /// 历史能力（必填）。
    pub history: HistoryMode,
    /// 每条消息允许的操作（必填）。
    pub actions: ActionRules,
    /// 在平台客户端里打开会话的链接模板，`{conversation}` 换成 URL 编码的会话编号。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chat_link: Option<String>,
    /// 下载图片时文件名的开头；默认用平台名。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_name_prefix: Option<String>,
    /// 各类会话的叫法。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversation_nouns: Option<ConversationNouns>,
    /// 按会话读一页最多几条（正整数，不写按宿主默认 50）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_max: Option<u32>,
    /// 读取请求合计上限（不写按宿主默认每秒 5、每分钟 120）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reads: Option<RateBudget>,
    /// 下载资源的上限；平台不限为空。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub downloads: Option<RateBudget>,
    /// 文件按多大一段读（支持续传）；不给就整份读。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_segment_bytes: Option<u64>,
    /// 发送附件上限（能发图片/文件时必填）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limits: Option<Limits>,
    /// 默认读取间隔。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_defaults: Option<ReadIntervalsSecs>,
    /// 话题平台事实；平台有话题时给。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub threads: Option<Threads>,
    /// 撤回的消息直接从历史消失。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub deleted_messages_vanish: bool,
    /// 读消息时会随消息带回完整表情。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub reactions_in_reads: bool,
    /// 不同账号表情目录不同。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub emoji_per_account: bool,
    /// 表情规则（有表情能力时给）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emoji: Option<EmojiRules>,
    /// 会话发现参数；宿主按能力选择账号级 `conversations/recentActivity`，或按会话列表与消息读取做限频探测。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discovery: Option<DiscoveryDesc>,
    /// 开放给用户改的读取与下载项；不写就一项都不开放。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub user_adjustable: Vec<Adjustable>,
    /// 哪几种内容块组成的消息，平台改过时只有内容变了、不标编辑时间：宿主见过旧内容、更新时间往后走、内容真变了才记编辑过。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub edits_by_content: Vec<ContentKind>,
    /// 当前的重译规则，按顺序（声明了 `retranslate` 能力时有用）。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub retranslate_rules: Vec<RetranslateRule>,
    /// 按顺序执行的已存消息修正规则；不依赖原始响应。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content_repairs: Vec<String>,
    /// 当前视野的旧消息是否需要重新读取的判定规则。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content_rereads: Vec<String>,
}

impl PlatformDesc {
    /// 校验旧消息维护声明，供宿主与插件自检使用同一组约束。
    ///
    /// # Errors
    /// 规则名、预算或能力组合不合法时返回原因。
    pub fn check_maintenance(&self, caps: &Capabilities) -> Result<(), &'static str> {
        if caps.message_maintenance
            && self
                .retranslate_rules
                .iter()
                .any(|r| r.name.starts_with("repair:"))
        {
            return Err("重译规则占用了宿主的维护标记");
        }
        if !caps.message_maintenance
            && (!self.content_repairs.is_empty() || !self.content_rereads.is_empty())
        {
            return Err("消息维护规则没有声明对应能力");
        }
        if !self.content_rereads.is_empty() && !caps.messages.get {
            return Err("旧消息重读需要声明 messages.get");
        }
        for names in [&self.content_repairs, &self.content_rereads] {
            let mut seen = std::collections::HashSet::new();
            if names.len() > crate::budget::MAINTENANCE_RULES_MAX
                || names.iter().any(|name| {
                    name.is_empty()
                        || name.len() > 128
                        || !name
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
                        || !seen.insert(name)
                })
            {
                return Err("消息维护规则名不合法或重复");
            }
        }
        if self
            .content_rereads
            .iter()
            .any(|name| self.retranslate_rules.iter().any(|r| &r.name == name))
        {
            return Err("重读与重译规则使用了同一个完成标记");
        }
        Ok(())
    }
}

/// 一组能力开关（protocol-v1 §5.2）。没写出来的都算不支持。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    /// 别人的阅读回执，与本人的读写位置独立。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipts: Option<crate::reading::ReceiptCaps>,
    /// 本人明确查看后向平台写回。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mark_read: Option<crate::reading::MarkReadCaps>,
    /// 从平台读回本人在其它客户端的阅读事实。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_sync: Option<crate::reading::ReadSyncCaps>,
    /// 表情包与远端收藏的独立能力。
    #[serde(default, skip_serializing_if = "crate::StickerCaps::is_empty")]
    pub stickers: crate::StickerCaps,
    /// 是否能跨账号保存／读取伙伴代发署名；旧插件不声明则全部关闭。
    #[serde(default, skip_serializing_if = "AgentAttributionCaps::is_empty")]
    pub agent_attribution: AgentAttributionCaps,
    /// 会话能力。
    #[serde(default)]
    pub conversations: ConversationCaps,
    /// 消息读取能力。
    #[serde(default)]
    pub messages: MessageCaps,
    /// 人员能力。
    #[serde(default)]
    pub people: PeopleCaps,
    /// 发送能力；null 或全 false 即只读。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub send: Option<SendCaps>,
    /// 能撤回。
    #[serde(default)]
    pub recall: bool,
    /// 能编辑。
    #[serde(default)]
    pub edit: bool,
    /// 表情能力。
    #[serde(default)]
    pub reactions: ReactionCaps,
    /// 资源能力。
    #[serde(default)]
    pub resources: ResourceCaps,
    /// 推送事件能力。
    #[serde(default)]
    pub events: EventCaps,
    /// 支持原始数据与重新翻译。
    #[serde(default)]
    pub retranslate: bool,
    /// 支持对统一消息进行纯本地修正与重读判定。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub message_maintenance: bool,
    /// 会话（账号）能力。
    #[serde(default, skip_serializing_if = "SessionCaps::is_empty")]
    pub session: SessionCaps,
    /// 剖析开着时能自报内部耗时（`perf/start` / `perf/report` / `perf/stop`，契约 provider-protocol v32）。
    /// 用运行时（`runtime::serve_*`）的插件由运行时自动声明；不声明的插件宿主不发任何 `perf/*`。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub perf: bool,
    /// 本人账号资料能力（契约 provider-protocol v35）。
    #[serde(default, skip_serializing_if = "AccountCaps::is_empty")]
    pub account: AccountCaps,
    /// 发送与编辑时能收的格式正文（契约 provider-protocol v40）；不写就只收文字（与以前一样）。
    #[serde(default, skip_serializing_if = "RichSendCaps::is_empty")]
    pub rich_send: RichSendCaps,
}

/// 能交给插件的格式（契约 provider-protocol v40），名字同格式正文的节点类型（protocol-v1 §7.6）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RichFormat {
    /// 粗体。
    Strong,
    /// 斜体。
    Emphasis,
    /// 下划线。
    Underline,
    /// 删除线。
    Delete,
    /// 行内代码。
    InlineCode,
    /// 链接。
    Link,
    /// 引用。
    Blockquote,
    /// 列表（有序、无序，可以嵌套）。
    List,
    /// 代码块。
    Code,
    /// 不认识的名字（更新的插件写的）：宿主忽略这一项。
    #[serde(other)]
    Unknown,
}

/// 发送与编辑时的格式正文（契约 provider-protocol v40）。
///
/// 声明了格式的插件，宿主在用户用了格式时交 [`crate::Outgoing::Rich`]（单独成一条的全部内容），树里只有这里声明过的格式；
/// 段落、文字、换行、提及、表情不用声明，格式正文里总可以有。没用格式时照旧交文字与提及。
///
/// # Examples
///
/// ```
/// use hhh_im_protocol::platform::{RichFormat, RichSendCaps};
/// let caps: RichSendCaps = serde_json::from_str(r#"{"formats":["strong","list","sparkle"],"edit":true}"#).unwrap();
/// assert!(caps.allows(RichFormat::Strong) && caps.allows(RichFormat::List));
/// assert!(!caps.allows(RichFormat::Underline));
/// assert!(!caps.allows(RichFormat::Unknown), "不认识的名字不算声明了什么");
/// assert!(RichSendCaps::default().is_empty());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RichSendCaps {
    /// 能发的格式；空就不收格式正文。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub formats: Vec<RichFormat>,
    /// 编辑时也收格式正文（要先能编辑）；这时消息的编辑原文 `editDraft` 也可以是格式正文。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub edit: bool,
}

impl RichSendCaps {
    /// 什么都没声明。
    pub fn is_empty(&self) -> bool {
        self.formats.is_empty() && !self.edit
    }

    /// 声明了这种格式（`Unknown` 永远不算）。
    pub fn allows(&self, format: RichFormat) -> bool {
        format != RichFormat::Unknown && self.formats.contains(&format)
    }
}

/// 本人账号资料能力。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AccountCaps {
    /// 收宿主交来的本机用户头像（`account/avatar`），拿去设置本人在这家 IM 里的头像。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub host_avatar: bool,
}

impl AccountCaps {
    fn is_empty(&self) -> bool {
        !self.host_avatar
    }
}

/// 跨账号的伙伴署名能力；声明发送即承诺接收方能经平台读回相同信息。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AgentAttributionCaps {
    /// 发送时能将独立署名保存到接收账号可读取的位置。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub send: bool,
    /// 能从历史、补读或推送还原伙伴署名。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub receive: bool,
}

impl AgentAttributionCaps {
    fn is_empty(&self) -> bool {
        !self.send && !self.receive
    }
}

/// 会话能力。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConversationCaps {
    /// 能列出会话。
    #[serde(default)]
    pub list: bool,
    /// 能搜索会话。
    #[serde(default)]
    pub search: bool,
    /// 能列群成员。
    #[serde(default)]
    pub members: bool,
    /// 能定位已有私聊。
    #[serde(default)]
    pub find_direct: bool,
    /// 能列出一段时间里有人发过消息的会话（会话发现，契约 provider-protocol v12 缺口 1）。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub recent_activity: bool,
}

/// 会话（账号）能力。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SessionCaps {
    /// 能用存着的凭据续期（`session/renew`，契约 provider-protocol v12 缺口 7）。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub renew: bool,
}

impl SessionCaps {
    fn is_empty(&self) -> bool {
        !self.renew
    }
}

/// 消息读取能力。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MessageCaps {
    /// 能分页读消息。
    #[serde(default)]
    pub list: bool,
    /// 能补读单条消息。
    #[serde(default)]
    pub get: bool,
    /// 能把合并转发或原生转发快照翻成 forwarded 内容块（只读）。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub forwarded: bool,
}

/// 人员能力。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PeopleCaps {
    /// Explicit group-member contact lookup and addition; omitted for legacy plugins.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub contact_actions: bool,
    /// Explicit friend approval; never assumed from contactActions.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub friend_requests: bool,
    /// 能分页列出已建立关系的联系人及其私聊资料，不依赖近期消息。
    /// 宿主按 `contacts/list` 读取；未声明时不发请求。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub contacts: bool,
    /// 能按编号批量读人。
    #[serde(default)]
    pub get: bool,
    /// 能搜索人。
    #[serde(default)]
    pub search: bool,
    /// 能按编号读机器人与应用（v15，可选）：宿主把消息里认出的机器人编号放进 `people/get` 的 `bots` 一起读。
    /// 不写时机器人一律算“没有”，宿主不为它们问程序。要先能 `get`。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub bots: bool,
    /// 一次 `people/get` 最多问几个（`ids` 与 `bots` 合计；v15，可选）：平台一次读一大批更省时写（如读一遍整份成员名单
    /// 就认出一大批人），1 到 [`crate::budget::PEOPLE_GET_MAX`]；不写是 [`crate::budget::PEOPLE_GET_DEFAULT`]。要先能 `get`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_max: Option<u32>,
}

/// 发送能力。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SendCaps {
    /// 能发文字。
    #[serde(default)]
    pub text: bool,
    /// 能带结构化提及。
    #[serde(default)]
    pub mentions: bool,
    /// 能提及所有人。
    #[serde(default)]
    pub mention_all: bool,
    /// 能发图片。
    #[serde(default)]
    pub image: bool,
    /// 能发文件。
    #[serde(default)]
    pub file: bool,
    /// 能按操作编号查询发送结果。
    #[serde(default)]
    pub lookup: bool,
    /// 收得下发出视频文件附带的视频信息（provider-protocol v52）：宿主先按图片上传封面，再在 `Outgoing::File`
    /// 的 `video` 里给封面资源键、时长与宽高。要先能发文件；不写时宿主不上传封面、不带 `video`。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub video_info: bool,
}

impl SendCaps {
    /// 一项都不能发即只读。
    pub fn is_read_only(&self) -> bool {
        !(self.text || self.image || self.file)
    }
}

/// 表情能力。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReactionCaps {
    /// 能读表情。
    #[serde(default)]
    pub list: bool,
    /// 能点表情。
    #[serde(default)]
    pub add: bool,
    /// 能取消表情。
    #[serde(default)]
    pub remove: bool,
    /// 有表情目录。
    #[serde(default)]
    pub catalog: bool,
}

/// 资源能力。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCaps {
    /// 能按段读消息资源。
    #[serde(default)]
    pub read: bool,
    /// 能读头像。
    #[serde(default)]
    pub avatars: bool,
    /// 能描述并按稳定编号读取同一资源的预览与原件；旧插件不写。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub variants: bool,
}

/// 推送事件能力。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EventCaps {
    /// 推新消息。
    #[serde(default)]
    pub messages: bool,
    /// 推撤回（消失型平台）。
    #[serde(default)]
    pub deletions: bool,
    /// 推表情变化。
    #[serde(default)]
    pub reactions: bool,
    /// 推会话变化。
    #[serde(default)]
    pub conversations: bool,
    /// 推人员变化。
    #[serde(default)]
    pub people: bool,
}

/// 提供方内编号稳定的参数；缺少类型时沿用文字字段。
///
/// # Examples
/// ```
/// use hhh_im_protocol::platform::SettingDecl;
/// let field = SettingDecl { id: "endpoint".into(), label: "地址".into(), ..Default::default() };
/// assert!(field.field_type.is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SettingDecl {
    /// 字段编号。
    pub id: String,
    /// 显示名称。
    pub label: String,
    /// 值保存在系统凭据库。
    #[serde(default)]
    pub secret: bool,
    /// 是否必填。
    #[serde(default)]
    pub required: bool,
    /// 校验正则。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    /// 帮助文字。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    /// 控件类型：text、textarea、number、switch、select、secret。
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub field_type: Option<String>,
    /// 字符串默认值；密钥不能声明默认值。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    /// 数值下界。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<i64>,
    /// 数值上界。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<i64>,
    /// 数字旁的单位。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    /// 单选项。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<SettingOption>,
    /// 分组与折叠说明。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<SettingGroup>,
    /// 折叠摘要中的简短名称。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary_label: Option<String>,
    /// 另一个字段等于指定值时显示。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_when: Option<SettingCondition>,
    /// 不能编辑时给用户的原因。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled_reason: Option<String>,
    /// 数值不能小于这个字段的值。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub not_less_than: Option<String>,
    /// 数值不能大于这个字段的值。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub not_greater_than: Option<String>,
}

/// 一个单选项。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingOption {
    /// 传给提供方的值。
    pub value: String,
    /// 显示名称。
    pub label: String,
}

/// 字段分组；折叠组默认收起。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingGroup {
    /// 分组编号。
    pub id: String,
    /// 分组名称。
    pub label: String,
    /// 默认折叠。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub fold: bool,
    /// 展开时的说明。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    /// 不随折叠隐藏的补充说明。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// 简单相等条件，不能引用密钥。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingCondition {
    /// 另一个字段编号。
    pub field: String,
    /// 满足条件时的值。
    pub equals: String,
}

impl Capabilities {
    /// 校验能力组合是否自洽（protocol-v1 §5.2），不自洽时返回说明哪项不对。
    ///
    /// # Errors
    /// 组合不自洽时返回一句中文说明。
    pub fn check_consistent(&self, history: HistoryMode) -> Result<(), String> {
        if self.messages.forwarded
            && !(self.messages.list || self.messages.get || self.events.messages)
        {
            return Err("声明 messages.forwarded 时必须能读取或接收消息".into());
        }
        if let Some(problem) = crate::reading::capabilities_problem(self) {
            return Err(problem.into());
        }
        if self.resources.variants && !self.resources.read {
            return Err("声明 resources.variants 时必须能读取资源（resources.read）".into());
        }

        if self.agent_attribution.send && !self.send.as_ref().is_some_and(|send| send.text) {
            return Err("声明 agentAttribution.send 时必须能发送文字".into());
        }
        if self.agent_attribution.receive
            && !(self.messages.list || self.messages.get || self.events.messages)
        {
            return Err("声明 agentAttribution.receive 时必须能读取或接收消息".into());
        }
        match history {
            HistoryMode::Server if !self.messages.list => {
                return Err("history=server 必须声明 messages.list".into());
            }
            HistoryMode::None if !self.events.messages => {
                return Err("history=none 必须声明 events.messages".into());
            }
            HistoryMode::None if !self.conversations.list && !self.events.conversations => {
                return Err("history=none 且不能列会话时，必须声明 events.conversations".into());
            }
            _ => {}
        }
        if !self.rich_send.is_empty() && !self.send.as_ref().is_some_and(|send| send.text) {
            return Err("声明 richSend 时必须能发送文字".into());
        }
        if self
            .send
            .as_ref()
            .is_some_and(|send| send.video_info && !send.file)
        {
            return Err("声明 send.videoInfo 时必须能发文件（send.file）".into());
        }
        if self.rich_send.edit && !self.edit {
            return Err("声明 richSend.edit 时必须能编辑（capabilities.edit）".into());
        }
        if self.rich_send.edit && self.rich_send.formats.is_empty() {
            return Err("声明 richSend.edit 时要写能发的格式（richSend.formats）".into());
        }
        if self.people.bots && !self.people.get {
            return Err("声明 people.bots 时必须声明 people.get（机器人与人一起按编号读）".into());
        }
        if let Some(max) = self.people.batch_max {
            if !self.people.get {
                return Err("声明 people.batchMax 时必须声明 people.get".into());
            }
            if max == 0 || max as usize > crate::budget::PEOPLE_GET_MAX {
                return Err(format!(
                    "people.batchMax 要在 1 到 {} 之间",
                    crate::budget::PEOPLE_GET_MAX
                ));
            }
        }
        Ok(())
    }

    /// 是不是只读连接（没有发送能力）。
    pub fn is_read_only(&self) -> bool {
        !self.stickers.send && self.send.as_ref().is_none_or(SendCaps::is_read_only)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn forwarded_reading_is_optional_and_requires_a_message_source() {
        use super::*;
        let mut old: Capabilities =
            serde_json::from_value(serde_json::json!({"messages":{"list":true,"get":true}}))
                .unwrap();
        assert!(!old.messages.forwarded);
        assert!(
            serde_json::to_value(&old).unwrap()["messages"]
                .get("forwarded")
                .is_none()
        );
        old.messages.forwarded = true;
        assert!(old.check_consistent(HistoryMode::Server).is_ok());
        assert_eq!(
            serde_json::to_value(&old).unwrap()["messages"]["forwarded"],
            true
        );
        old.messages.list = false;
        old.messages.get = false;
        assert!(
            old.check_consistent(HistoryMode::None)
                .unwrap_err()
                .contains("forwarded")
        );
        old.events.messages = true;
        old.events.conversations = true;
        assert!(old.check_consistent(HistoryMode::None).is_ok());
    }

    // 契约 provider-protocol v52：收视频信息是可选声明，要先能发文件；不写时序列化里没有这一键（旧宿主照旧）。
    #[test]
    fn v52_video_info_is_optional_and_requires_file() {
        use super::*;
        use serde_json::json;
        let old: Capabilities = serde_json::from_value(
            json!({"send": {"text": true, "file": true}, "messages": {"list": true}}),
        )
        .unwrap();
        assert!(!old.send.unwrap().video_info, "旧插件不写就是不收视频信息");
        assert!(
            serde_json::to_value(&old).unwrap()["send"]
                .get("videoInfo")
                .is_none(),
            "写回不多出键"
        );

        let mut caps: Capabilities = serde_json::from_value(
            json!({"send": {"file": true, "videoInfo": true}, "messages": {"list": true}}),
        )
        .unwrap();
        assert!(caps.send.unwrap().video_info);
        assert!(caps.check_consistent(HistoryMode::Server).is_ok());
        assert_eq!(
            serde_json::to_value(&caps).unwrap()["send"]["videoInfo"],
            true
        );

        caps.send = Some(SendCaps {
            text: true,
            video_info: true,
            ..SendCaps::default()
        });
        assert!(
            caps.check_consistent(HistoryMode::Server)
                .unwrap_err()
                .contains("send.videoInfo")
        );
    }

    #[test]
    fn v40_rich_send_is_optional_needs_text_and_edit_and_skips_unknown_names() {
        use super::*;
        use serde_json::json;
        let old: Capabilities = serde_json::from_value(json!({"send": {"text": true}})).unwrap();
        assert!(old.rich_send.is_empty(), "旧插件不写就是不收格式正文");
        assert!(
            serde_json::to_value(&old)
                .unwrap()
                .get("richSend")
                .is_none(),
            "写回不多出键"
        );

        let mut caps: Capabilities = serde_json::from_value(json!({
            "send": {"text": true}, "edit": true,
            "richSend": {"formats": ["strong", "emphasis", "blockquote", "glitter"], "edit": true}
        }))
        .unwrap();
        assert_eq!(caps.rich_send.formats.len(), 4);
        assert!(caps.rich_send.allows(RichFormat::Blockquote));
        assert!(!caps.rich_send.allows(RichFormat::Unknown));
        assert!(
            caps.check_consistent(HistoryMode::Server).is_err(),
            "server 读法还缺 messages.list"
        );
        caps.messages.list = true;
        assert!(caps.check_consistent(HistoryMode::Server).is_ok());
        let back = serde_json::to_value(&caps).unwrap();
        assert_eq!(back["richSend"]["edit"], true);

        caps.edit = false;
        assert!(
            caps.check_consistent(HistoryMode::Server)
                .unwrap_err()
                .contains("richSend.edit")
        );
        caps.edit = true;
        caps.send = None;
        assert!(
            caps.check_consistent(HistoryMode::Server)
                .unwrap_err()
                .contains("richSend")
        );
        caps.send = Some(SendCaps {
            text: true,
            ..SendCaps::default()
        });
        caps.rich_send.formats.clear();
        assert!(
            caps.check_consistent(HistoryMode::Server)
                .unwrap_err()
                .contains("formats")
        );
    }

    #[test]
    fn maintenance_declarations_are_opt_in_bounded_and_keep_old_rules_valid() {
        use serde_json::json;
        let mut platform: PlatformDesc = serde_json::from_value(json!({
            "avatarShape": "circle", "history": "server",
            "actions": {"reply": false, "thread": false, "react": false, "edit": {"own": false}},
            "retranslateRules": [{"name": "repair:legacy"}]
        }))
        .unwrap();
        let mut caps = Capabilities::default();
        assert!(
            platform.check_maintenance(&caps).is_ok(),
            "旧插件不受新可选能力约束"
        );
        caps.message_maintenance = true;
        assert!(platform.check_maintenance(&caps).is_err());
        platform.retranslate_rules.clear();
        for names in [
            vec!["".into()],
            vec!["bad:name".into()],
            vec!["规则".into()],
            vec!["x".repeat(129)],
            vec!["same".into(), "same".into()],
            (0..65).map(|n| format!("rule-{n}")).collect(),
        ] {
            platform.content_repairs = names;
            assert!(platform.check_maintenance(&caps).is_err());
        }
        platform.content_repairs = vec!["case_v1.2-fix".into()];
        assert!(platform.check_maintenance(&caps).is_ok());
        caps.message_maintenance = false;
        assert!(platform.check_maintenance(&caps).is_err());
        caps.message_maintenance = true;
        platform.content_rereads = vec!["read".into()];
        assert!(
            platform.check_maintenance(&caps).is_err(),
            "重读需要 messages.get"
        );
        caps.messages.get = true;
        assert!(platform.check_maintenance(&caps).is_ok());
        platform.retranslate_rules = serde_json::from_value(json!([{"name": "read"}])).unwrap();
        assert!(
            platform.check_maintenance(&caps).is_err(),
            "重读和重译不能共用完成标记"
        );
    }

    // 契约 provider-protocol v31：表情写进文字的写法可选、原样往返；格式按规则读，不合要求的给出原因。
    #[test]
    fn v31_text_template_is_optional_and_parsed_by_the_rules() {
        use serde_json::json;
        let rules: EmojiRules =
            serde_json::from_value(json!({ "unlisted": { "placeholderLabel": "表情" } })).unwrap();
        assert!(rules.text_template.is_none());
        let declared = json!({ "unlisted": { "placeholderLabel": "表情" }, "textPattern": ":([^:\\s]+):", "textTemplate": ":{type}:" });
        let rules: EmojiRules = serde_json::from_value(declared.clone()).unwrap();
        assert_eq!(serde_json::to_value(&rules).unwrap(), declared, "原样往返");

        let slack = TextTemplate::parse(":{type}:").unwrap();
        assert_eq!(
            (slack.slot(), slack.write("+1")),
            (TemplateSlot::Type, ":+1:".to_owned())
        );
        let bare = TextTemplate::parse("{name}").unwrap();
        assert_eq!(bare.write("赞"), "赞", "占位前后可以没有字");
        assert!(
            TextTemplate::parse("<<<<<<<<{name}>>>>>>>>").is_ok(),
            "前后合计 16 个字符可以"
        );
        for (bad, why) in [
            ("[赞]", "没有占位"),
            ("{name}{name}", "两个占位"),
            ("[{name}|{type}]", "两种占位"),
            ("[{name}]\n", "换行"),
            ("<<<<<<<<<{name}>>>>>>>>", "前后合计超过 16 个字符"),
            ("[{Name}]", "占位大小写不对"),
        ] {
            assert!(TextTemplate::parse(bad).is_err(), "{why}：{bad}");
        }
    }

    // 契约 provider-protocol v14：头像身份、猜图范围都可选；不写时读成空、写回不多出键，写了原样往返。
    #[test]
    fn v14_platform_declarations_are_optional_and_round_trip() {
        use serde_json::json;
        let actions =
            json!({ "reply": true, "thread": false, "react": false, "edit": { "own": true } });
        let old: PlatformDesc = serde_json::from_value(
            json!({ "avatarShape": "circle", "history": "server", "actions": actions }),
        )
        .unwrap();
        assert!(old.avatar_identity.is_none());
        assert!(
            serde_json::to_value(&old)
                .unwrap()
                .get("avatarIdentity")
                .is_none()
        );
        let new = json!({
            "avatarShape": "circle", "history": "server", "actions": actions,
            "avatarIdentity": { "keyPattern": "/(v3_[^/?#]+)", "variantPattern": "[?&]image_size=([^&#]+)" },
            "emoji": { "unlisted": { "placeholderLabel": "企业表情", "guess": true, "guessPattern": "^[A-Za-z_]+$" } }
        });
        let desc: PlatformDesc = serde_json::from_value(new.clone()).unwrap();
        assert_eq!(serde_json::to_value(&desc).unwrap(), new, "新声明原样往返");
        let identity = desc.avatar_identity.unwrap();
        assert_eq!(
            (
                identity.key_pattern.as_str(),
                identity.variant_pattern.as_deref()
            ),
            ("/(v3_[^/?#]+)", Some("[?&]image_size=([^&#]+)"))
        );
    }
    use super::*;

    #[test]
    fn manifest_ids_follow_the_rule() {
        assert!(Manifest::id_ok("wechat-pad"));
        assert!(Manifest::id_ok("ab"));
        assert!(!Manifest::id_ok("a"));
        assert!(!Manifest::id_ok("1x"));
        assert!(!Manifest::id_ok("Has-Upper"));
        assert!(!Manifest::id_ok("under_score"));
    }

    // IM 类型可选：写了就以 imType 往返、按编号的写法校验；没写时不输出这个字段，旧清单照常读。
    #[test]
    fn manifest_im_type_is_optional_and_follows_the_id_rule() {
        let base = r#"{"manifestVersion":1,"id":"feishu-pro","name":"飞书 Pro","version":"0.1.0","command":"p","os":["windows"]}"#;
        let old: Manifest = serde_json::from_str(base).unwrap();
        assert_eq!(old.im_type, None);
        assert!(old.im_type_ok());
        assert!(serde_json::to_value(&old).unwrap().get("imType").is_none());

        let typed: Manifest =
            serde_json::from_str(&base.replace(r#""os""#, r#""imType":"feishu","os""#)).unwrap();
        assert_eq!(typed.im_type.as_deref(), Some("feishu"));
        assert!(typed.im_type_ok());
        assert_eq!(serde_json::to_value(&typed).unwrap()["imType"], "feishu");

        for bad in ["Feishu", "f", "1feishu", "fei_shu", ""] {
            let manifest = Manifest {
                im_type: Some(bad.into()),
                ..typed.clone()
            };
            assert!(!manifest.im_type_ok(), "{bad:?} 不该算合规的 IM 类型");
        }
    }

    #[test]
    fn read_only_is_no_send_or_all_false() {
        let mut caps = Capabilities::default();
        assert!(caps.is_read_only(), "没有 send 即只读");
        caps.send = Some(SendCaps::default());
        assert!(caps.is_read_only(), "全 false 也是只读");
        caps.send = Some(SendCaps {
            text: true,
            ..SendCaps::default()
        });
        assert!(!caps.is_read_only());
    }

    #[test]
    fn capability_consistency_follows_history_mode() {
        let mut caps = Capabilities::default();
        assert!(caps.check_consistent(HistoryMode::Server).is_err());
        caps.messages.list = true;
        assert!(caps.check_consistent(HistoryMode::Server).is_ok());

        let mut push = Capabilities::default();
        assert!(push.check_consistent(HistoryMode::None).is_err());
        push.events.messages = true;
        // 不能列会话且不推会话 → 仍不自洽。
        assert!(push.check_consistent(HistoryMode::None).is_err());
        push.events.conversations = true;
        assert!(push.check_consistent(HistoryMode::None).is_ok());
    }

    // 契约 provider-protocol v15：读机器人是可选声明，要先能按编号读人；不写时序列化里没有这一键（旧宿主照旧）。
    #[test]
    fn people_bots_needs_get_and_is_omitted_when_false() {
        let mut caps = Capabilities::default();
        caps.messages.list = true;
        caps.people.bots = true;
        assert!(caps.check_consistent(HistoryMode::Server).is_err());
        caps.people.get = true;
        assert!(caps.check_consistent(HistoryMode::Server).is_ok());
        let on = serde_json::to_value(caps.people).unwrap();
        assert_eq!(on["bots"], true);
        let off = serde_json::to_value(PeopleCaps {
            contact_actions: false,
            friend_requests: false,
            get: true,
            search: false,
            bots: false,
            contacts: false,
            batch_max: None,
        })
        .unwrap();
        assert!(off.get("bots").is_none());
        assert!(off.get("batchMax").is_none());
        let old: PeopleCaps =
            serde_json::from_value(serde_json::json!({ "get": true, "search": true })).unwrap();
        assert!(!old.bots, "旧插件不写就是不读机器人");
        assert_eq!(old.batch_max, None, "旧插件不写就是一次 100 个");
    }

    // 契约 provider-protocol v15：一次读人的上限可选，要先能按编号读人、在 1 到 2000 之间。
    #[test]
    fn people_batch_max_needs_get_and_stays_in_range() {
        let mut caps = Capabilities::default();
        caps.messages.list = true;
        caps.people.batch_max = Some(2000);
        assert!(caps.check_consistent(HistoryMode::Server).is_err());
        caps.people.get = true;
        assert!(caps.check_consistent(HistoryMode::Server).is_ok());
        assert_eq!(serde_json::to_value(caps.people).unwrap()["batchMax"], 2000);
        for bad in [0, 2001] {
            caps.people.batch_max = Some(bad);
            assert!(caps.check_consistent(HistoryMode::Server).is_err(), "{bad}");
        }
    }

    #[test]
    fn platform_desc_omits_empty_optionals_but_keeps_required() {
        let d = PlatformDesc {
            avatar_shape: AvatarShape::Rounded,
            avatar_identity: None,
            history: HistoryMode::None,
            actions: ActionRules {
                recall: Some(OwnWithin {
                    own: true,
                    within_secs: Some(120),
                }),
                ..ActionRules::default()
            },
            chat_link: None,
            image_name_prefix: None,
            conversation_nouns: None,
            page_max: None,
            reads: None,
            downloads: None,
            file_segment_bytes: None,
            limits: None,
            read_defaults: None,
            threads: None,
            deleted_messages_vanish: false,
            reactions_in_reads: false,
            emoji_per_account: false,
            emoji: None,
            discovery: None,
            user_adjustable: Vec::new(),
            edits_by_content: Vec::new(),
            retranslate_rules: Vec::new(),
            content_repairs: Vec::new(),
            content_rereads: Vec::new(),
        };
        let v = serde_json::to_value(&d).unwrap();
        assert_eq!(v["avatarShape"], "rounded");
        assert_eq!(v["history"], "none");
        assert_eq!(v["actions"]["recall"]["withinSecs"], 120);
        assert!(v.get("chatLink").is_none());
        assert!(v.get("deletedMessagesVanish").is_none(), "false 不序列化");
        for key in [
            "discovery",
            "userAdjustable",
            "editsByContent",
            "retranslateRules",
        ] {
            assert!(v.get(key).is_none(), "没声明的 {key} 不写");
        }
    }

    // 契约 provider-protocol v12：平台说明的新声明都是可选的——旧插件的形状照读、写回不多出键；新声明原样往返；
    // 宿主不认识的可调项、内容块种类读成 Unknown、不让整个握手失败。
    #[test]
    fn v12_platform_declarations_are_optional_and_round_trip() {
        use serde_json::json;
        let old = json!({
            "avatarShape": "circle", "history": "server", "actions": { "reply": true, "thread": false, "react": false,
              "edit": { "own": true } },
            "pageMax": 50, "reads": { "perSecond": 5, "perMinute": 120 },
            "emoji": { "unlisted": { "placeholderLabel": "表情" } }
        });
        let desc: PlatformDesc = serde_json::from_value(old.clone()).unwrap();
        assert_eq!(
            serde_json::to_value(&desc).unwrap(),
            old,
            "旧形状写回不多出键"
        );
        assert!(desc.user_adjustable.is_empty() && desc.discovery.is_none());

        let new = json!({
            "avatarShape": "rounded", "history": "server",
            "actions": { "reply": false, "thread": false, "react": false,
              "edit": { "own": true, "withinSecs": 86400, "perMessage": true } },
            "pageMax": 200000,
            "readDefaults": { "watching": 2, "active": 5, "quiet": 60, "dormant": 300, "stale": 600, "profile": 600,
              "people": 600, "threadDays": 7, "discover": 45 },
            "emoji": { "unlisted": { "placeholderLabel": "表情", "guess": true },
              "textPattern": "\\[([^\\[\\]\\r\\n]{1,12})\\]", "whiteBackground": true },
            "discovery": { "fastestSecs": 15, "pages": 3, "coldPages": 10 },
            "userAdjustable": ["readIntervals", "readsPerMinute", "fileSegmentBytes"],
            "editsByContent": ["text", "mention", "rich"],
            "retranslateRules": [{ "name": "sample-card-v1" }, { "name": "sample-card-v2", "formats": ["sample-msg-v1"] }]
        });
        let desc: PlatformDesc = serde_json::from_value(new.clone()).unwrap();
        assert_eq!(serde_json::to_value(&desc).unwrap(), new, "新声明原样往返");
        assert_eq!(desc.page_max, Some(200_000), "每页条数不设协议上限");
        assert!(desc.actions.edit.unwrap().per_message);
        assert_eq!(desc.read_defaults.unwrap().discover, Some(45));

        let future: PlatformDesc = serde_json::from_value(json!({
            "avatarShape": "circle", "history": "none", "actions": {},
            "userAdjustable": ["readIntervals", "somethingNew"], "editsByContent": ["card"]
        }))
        .unwrap();
        assert_eq!(
            future.user_adjustable,
            [Adjustable::ReadIntervals, Adjustable::Unknown]
        );
        assert_eq!(future.edits_by_content, [ContentKind::Unknown]);
    }

    // 契约 provider-protocol v35：`account.hostAvatar` 可选；没声明时线上不出现 `account`，旧形状写回不多出键。
    #[test]
    fn v35_account_caps_are_optional_and_round_trip() {
        use serde_json::json;
        let old: Capabilities =
            serde_json::from_value(json!({ "conversations": { "list": true } })).unwrap();
        assert!(!old.account.host_avatar);
        assert!(serde_json::to_value(&old).unwrap().get("account").is_none());
        let new: Capabilities = serde_json::from_value(
            json!({ "conversations": { "list": true }, "account": { "hostAvatar": true } }),
        )
        .unwrap();
        assert!(new.account.host_avatar);
        assert_eq!(
            serde_json::to_value(&new).unwrap()["account"],
            json!({ "hostAvatar": true })
        );
    }

    #[test]
    fn v12_capabilities_and_manifest_fields_are_optional() {
        use serde_json::json;
        let caps: Capabilities =
            serde_json::from_value(json!({ "conversations": { "list": true } })).unwrap();
        assert!(!caps.message_maintenance);
        assert!(
            serde_json::to_value(&caps)
                .unwrap()
                .get("messageMaintenance")
                .is_none()
        );
        assert!(!caps.conversations.recent_activity && !caps.session.renew);
        let v = serde_json::to_value(&caps).unwrap();
        assert!(v.get("session").is_none(), "没声明续期时不写 session");
        assert!(v["conversations"].get("recentActivity").is_none());

        let caps: Capabilities = serde_json::from_value(
            json!({ "conversations": { "recentActivity": true }, "session": { "renew": true } }),
        )
        .unwrap();
        assert!(caps.conversations.recent_activity && caps.session.renew);

        let manifest: Manifest = serde_json::from_value(json!({
            "manifestVersion": 1, "id": "sample", "name": "示例", "version": "0.1.0", "command": "p", "os": ["windows"]
        }))
        .unwrap();
        assert!(manifest.logo_style.is_none() && manifest.logo_ok());
        assert!(
            serde_json::to_value(&manifest)
                .unwrap()
                .get("logoStyle")
                .is_none()
        );
        let marked: Manifest = serde_json::from_value(json!({
            "manifestVersion": 1, "id": "sample", "name": "示例", "version": "0.1.0", "command": "p", "os": ["windows"],
            "logoStyle": "mark", "logoInk": { "x": 1.5, "y": 2, "width": 21, "height": 20 }
        }))
        .unwrap();
        assert_eq!(marked.logo_style, Some(LogoStyle::Mark));
        assert!(marked.logo_ok());
        let bad = Manifest {
            logo_ink: Some(LogoInk {
                x: f64::NAN,
                y: 0.0,
                width: 1.0,
                height: 1.0,
            }),
            ..marked
        };
        assert!(!bad.logo_ok());
    }
}
