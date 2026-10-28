//! 格式正文与卡片的类型（protocol-v1 §7.6、附录 B，契约 provider-protocol v15）：[`crate::Block::Rich`] 的 `root`、
//! [`crate::Block::Card`] 的 `card` 在线上是 JSON，这里给插件一套照规范写好的类型，搭好后 [`rich_block`]、
//! [`card_block`] 换成内容块，不用手拼 JSON。宿主照收（`hhh-im/src/provider/translate.rs` 的 `rich_root_in`、`card_in`）。
//!
//! 写法同规范：节点按 `type` 分辨；人写成 `{userId, idType?, kind?, name?}`；资源写成 `{resource, name?, width?, height?}`，
//! `resource` 是这条消息里的线上资源（[`crate::Resource`]）。其余节点是 mdast 的子集加几种 HHH 扩展。
//!
//! # Examples
//!
//! ```
//! use hhh_im_protocol::content::{PersonRef, RichNode, rich_block};
//! let block = rich_block(vec![RichNode::Paragraph {
//!     children: vec![
//!         RichNode::Text { value: "你好 ".into() },
//!         RichNode::Mention { person: Some(PersonRef::user("U1", Some("小明"))), everyone: false },
//!     ],
//! }]);
//! let json = serde_json::to_value(&block).unwrap();
//! assert_eq!(json["type"], "rich");
//! assert_eq!(json["root"]["type"], "root");
//! let mention = &json["root"]["children"][0]["children"][1];
//! assert_eq!(mention["userId"], "U1");
//! assert_eq!(mention["name"], "小明");
//! assert!(mention.get("everyone").is_none());
//! ```

use serde::{Deserialize, Serialize};

use crate::types::{Block, PersonKind, Resource};

/// 格式正文与卡片里的一个人（被提及的、卡片的人员）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonRef {
    /// 平台用户编号。
    pub user_id: String,
    /// 编号的种类（v14，可选），同发送者的 `idType`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_type: Option<String>,
    /// 人还是机器人（v14，可选）；不写按人。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<PersonKind>,
    /// 名字；读不到为空（宿主按人名册补）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl PersonRef {
    /// 一个人（不写编号种类与种类）。
    pub fn user(id: impl Into<String>, name: Option<&str>) -> Self {
        Self {
            user_id: id.into(),
            id_type: None,
            kind: None,
            name: name.map(str::to_owned),
        }
    }
}

/// 表格列的对齐。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RichAlign {
    /// 靠左。
    Left,
    /// 居中。
    Center,
    /// 靠右。
    Right,
}

/// 卡片的色调：平台的标题、文字、标签颜色归到这几种。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CardTone {
    /// 不上色。
    #[default]
    Neutral,
    /// 灰（次要文字色；标题栏不用）。
    Grey,
    /// 蓝。
    Blue,
    /// 绿。
    Green,
    /// 黄。
    Yellow,
    /// 橙。
    Orange,
    /// 红。
    Red,
    /// 紫。
    Purple,
}

/// 格式正文的一个节点（protocol-v1 §7.6）：mdast 的子集，加提及、表情、资源与卡片用的扩展。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum RichNode {
    /// 段落。
    Paragraph {
        /// 行内节点。
        children: Vec<RichNode>,
    },
    /// 标题（1–6 级）。
    Heading {
        /// 级别。
        depth: u8,
        /// 行内节点。
        children: Vec<RichNode>,
    },
    /// 分割线。
    ThematicBreak,
    /// 引用。
    Blockquote {
        /// 块。
        children: Vec<RichNode>,
    },
    /// 列表。
    List {
        /// 有序。
        ordered: bool,
        /// 有序列表的起始序号。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        start: Option<u32>,
        /// 列表项。
        children: Vec<RichNode>,
    },
    /// 列表项。
    ListItem {
        /// 块。
        children: Vec<RichNode>,
    },
    /// 代码块。
    Code {
        /// 语言。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        lang: Option<String>,
        /// 原文。
        value: String,
    },
    /// 表格。
    Table {
        /// 各列的对齐；不写都靠左。
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        align: Vec<Option<RichAlign>>,
        /// 一页几行（平台分页显示时）。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        page_size: Option<u16>,
        /// 行。
        children: Vec<RichNode>,
    },
    /// 表格行。
    TableRow {
        /// 单元格。
        children: Vec<RichNode>,
    },
    /// 单元格。
    TableCell {
        /// 行内节点。
        children: Vec<RichNode>,
    },
    /// 文字。
    Text {
        /// 原文。
        value: String,
    },
    /// 粗体。
    Strong {
        /// 行内节点。
        children: Vec<RichNode>,
    },
    /// 斜体。
    Emphasis {
        /// 行内节点。
        children: Vec<RichNode>,
    },
    /// 删除线。
    Delete {
        /// 行内节点。
        children: Vec<RichNode>,
    },
    /// 下划线。
    Underline {
        /// 行内节点。
        children: Vec<RichNode>,
    },
    /// 行内代码。
    InlineCode {
        /// 原文。
        value: String,
    },
    /// 换行。
    Break,
    /// 链接。
    Link {
        /// 地址。
        url: String,
        /// 行内节点。
        children: Vec<RichNode>,
    },
    /// 提及：一个人（`person` 的几项摊在节点上），或所有人。
    Mention {
        /// 被提及的人；提及所有人时为空。
        #[serde(flatten, skip_serializing_if = "Option::is_none")]
        person: Option<PersonRef>,
        /// 提及所有人。
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        everyone: bool,
    },
    /// 表情（平台的表情类型，宿主按表情目录显示）。
    Emoji {
        /// 表情类型。
        emoji_type: String,
    },
    /// 这条消息里的图片或文件。
    Resource {
        /// 资源。
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
    /// 暂不支持的内容。
    Unsupported {
        /// 平台内容种类。
        kind: String,
        /// 平台里这类内容的叫法。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
    },
    /// 彩色文字（卡片正文）。
    Color {
        /// 色调。
        tone: CardTone,
        /// 行内节点。
        children: Vec<RichNode>,
    },
    /// 小号彩色标签（卡片正文与表格选项）。
    Tag {
        /// 标签文字。
        text: String,
        /// 色调。
        tone: CardTone,
    },
    /// 一个人（卡片正文与表格）：头像 + 名字（人的几项摊在节点上）。
    Person {
        /// 这个人。
        #[serde(flatten)]
        person: PersonRef,
    },
    /// 一个时间点（卡片正文与表格），宿主按本机时区显示。
    Time {
        /// Unix 毫秒。
        ms: i64,
        /// 平台的格式：格式种类（如 `date_num`）或格式串（如 `YYYY-MM-DD`）。
        format: String,
    },
}

/// 一张卡片（protocol-v1 附录 B）：可选标题、色调与按顺序的若干块。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    /// 标题。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// 副标题。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    /// 标题后面的标签。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<CardTag>,
    /// 标题色调。
    pub tone: CardTone,
    /// 撑满消息宽度。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub wide: bool,
    /// 块。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<CardBlock>,
}

/// 卡片标题后面的一个标签。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardTag {
    /// 标签文字。
    pub text: String,
    /// 色调。
    pub tone: CardTone,
}

/// 卡片里的一张图：这条消息里的图片资源。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardImage {
    /// 资源。
    pub resource: Resource,
    /// 说明文字。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alt: Option<String>,
    /// 宽（像素）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    /// 高（像素）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
}

/// 卡片里人员的显示大小。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PersonSize {
    /// 小。
    Small,
    /// 中。
    Medium,
    /// 大。
    Large,
}

/// 卡片里的一块。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum CardBlock {
    /// 一段文字，由若干片按顺序拼成。
    Paragraph {
        /// 各片。
        parts: Vec<CardInline>,
    },
    /// 并排的几栏，每栏是若干块。
    Columns {
        /// 各栏，从左到右。
        columns: Vec<Vec<CardBlock>>,
    },
    /// 分割线。
    Divider,
    /// 一段格式正文。
    Rich {
        /// 块。
        children: Vec<RichNode>,
    },
    /// 一组图片，每行 `columns` 张。
    Images {
        /// 一行几张（1–3）。
        columns: u8,
        /// 图片。
        images: Vec<CardImage>,
    },
    /// 一组人。
    People {
        /// 人。
        people: Vec<PersonRef>,
        /// 大小。
        size: PersonSize,
        /// 显示头像。
        avatar: bool,
        /// 显示名字。
        name: bool,
    },
    /// 折叠面板。
    Panel {
        /// 面板标题（行内节点）。
        title: Vec<RichNode>,
        /// 初始展开。
        expanded: bool,
        /// 画边框。
        border: bool,
        /// 面板里的块。
        blocks: Vec<CardBlock>,
    },
}

/// 卡片段落里的一片。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum CardInline {
    /// 标准 Markdown 文字（平台的纯文字要先转义）。
    Markdown {
        /// Markdown 原文。
        text: String,
    },
}

#[derive(Serialize)]
struct Root<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    children: &'a [RichNode],
}

/// 一串块级节点 → 格式正文内容块（`{"type":"rich","root":{"type":"root","children":[...]}}`）。
pub fn rich_block(children: Vec<RichNode>) -> Block {
    let root = serde_json::to_value(Root {
        kind: "root",
        children: &children,
    })
    .unwrap_or_default();
    Block::Rich { root }
}

/// 若干块 → 要发出的格式正文（契约 provider-protocol v40），宿主交给插件的就是这个形状。
///
/// # Examples
///
/// ```
/// use hhh_im_protocol::content::{RichNode, rich_nodes, rich_outgoing};
/// use hhh_im_protocol::Outgoing;
/// let out = rich_outgoing(vec![RichNode::Paragraph {
///     children: vec![RichNode::Strong { children: vec![RichNode::Text { value: "明早 9 点".into() }] }],
/// }]);
/// let Outgoing::Rich { root } = &out else { panic!() };
/// assert_eq!(root["type"], "root");
/// assert_eq!(rich_nodes(root).unwrap().len(), 1);
/// ```
pub fn rich_outgoing(children: Vec<RichNode>) -> crate::Outgoing {
    let root = serde_json::to_value(Root {
        kind: "root",
        children: &children,
    })
    .unwrap_or_default();
    crate::Outgoing::Rich { root }
}

/// 读出格式正文的块（插件收到 [`crate::Outgoing::Rich`] 或编辑原文时用）。
///
/// # Errors
/// 根不是 `{"type":"root","children":[...]}`，或节点写法不对时返回说明。
pub fn rich_nodes(root: &serde_json::Value) -> Result<Vec<RichNode>, String> {
    if root.get("type").and_then(serde_json::Value::as_str) != Some("root") {
        return Err("格式正文的根要是 {\"type\":\"root\"}".into());
    }
    let children = root
        .get("children")
        .cloned()
        .ok_or_else(|| "格式正文的根缺 children".to_string())?;
    serde_json::from_value(children).map_err(|error| format!("格式正文的节点写法不对：{error}"))
}

/// 一张卡片 → 卡片内容块。
///
/// # Examples
///
/// ```
/// use hhh_im_protocol::content::{Card, CardBlock, CardInline, CardTone, card_block};
/// let block = card_block(&Card {
///     title: Some("日报".into()),
///     tone: CardTone::Blue,
///     blocks: vec![CardBlock::Paragraph { parts: vec![CardInline::Markdown { text: "**3** 条".into() }] }],
///     ..Card::default()
/// });
/// let json = serde_json::to_value(&block).unwrap();
/// assert_eq!(json["card"]["tone"], "blue");
/// assert_eq!(json["card"]["blocks"][0]["parts"][0]["text"], "**3** 条");
/// ```
pub fn card_block(card: &Card) -> Block {
    Block::Card {
        card: serde_json::to_value(card).unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::types::ResourceKind;

    // 人、资源照规范的写法；所有人的提及只有 everyone；其余节点同 mdast。
    #[test]
    fn people_and_resources_use_the_written_form() {
        let nodes = vec![
            RichNode::Mention {
                person: Some(PersonRef {
                    user_id: "B1".into(),
                    id_type: Some("user_id".into()),
                    kind: Some(PersonKind::Bot),
                    name: Some("日报机器人".into()),
                }),
                everyone: false,
            },
            RichNode::Mention {
                person: None,
                everyone: true,
            },
            RichNode::Person {
                person: PersonRef::user("U2", None),
            },
            RichNode::Resource {
                resource: Resource {
                    voice: false,
                    key: "F1".into(),
                    kind: ResourceKind::Image,
                    mime: None,
                    bytes: None,
                    duration_ms: None,
                    cover: None,
                },
                name: Some("a.png".into()),
                width: Some(640),
                height: Some(480),
            },
        ];
        let json = serde_json::to_value(&nodes).unwrap();
        assert_eq!(
            json,
            json!([
                { "type": "mention", "userId": "B1", "idType": "user_id", "kind": "bot", "name": "日报机器人" },
                { "type": "mention", "everyone": true },
                { "type": "person", "userId": "U2" },
                { "type": "resource", "resource": { "key": "F1", "kind": "image" }, "name": "a.png", "width": 640, "height": 480 }
            ])
        );
        let back: Vec<RichNode> = serde_json::from_value(json).unwrap();
        assert_eq!(back, nodes, "原样往返");
    }

    #[test]
    fn cards_omit_empty_optionals() {
        let card = Card {
            title: Some("T".into()),
            blocks: vec![
                CardBlock::Divider,
                CardBlock::People {
                    people: vec![PersonRef::user("U1", Some("小明"))],
                    size: PersonSize::Small,
                    avatar: true,
                    name: true,
                },
            ],
            ..Card::default()
        };
        assert_eq!(
            serde_json::to_value(&card).unwrap(),
            json!({
                "title": "T", "tone": "neutral",
                "blocks": [
                    { "type": "divider" },
                    { "type": "people", "people": [{ "userId": "U1", "name": "小明" }], "size": "small", "avatar": true, "name": true }
                ]
            })
        );
    }
}
