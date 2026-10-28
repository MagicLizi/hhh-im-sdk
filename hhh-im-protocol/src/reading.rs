//! 人的阅读事实。三项能力独立；读取这些事实本身不能标记已读。
use serde::{Deserialize, Serialize};

/// 一项能力支持的范围；话题的水位不推进会话，反之亦然。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadScopes {
    /// 普通会话。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub conversations: bool,
    /// 独立话题。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub threads: bool,
}

impl ReadScopes {
    /// 没声明任何范围。
    pub fn is_empty(&self) -> bool {
        !self.conversations && !self.threads
    }
    /// 范围是否获声明。
    pub fn allows(&self, scope: &ReadScope) -> bool {
        if scope.thread_root.is_some() {
            self.threads
        } else {
            self.conversations
        }
    }
}

/// 他人的阅读回执；人数及分母仍以每次响应为准。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptCaps {
    /// 可查询的范围。
    #[serde(flatten)]
    pub scopes: ReadScopes,
    /// 能列出读过的人；没有声明时不查询名单。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub readers: bool,
    /// 能列出未读的人，不能用群成员减已读名单推算。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub unreaders: bool,
    /// 只可查询距今这么多秒以内的本人消息；省略表示没有声明时限。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_age_secs: Option<u64>,
}

/// 本人阅读位置写回；平台必须精确尊重消息边界。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkReadCaps {
    /// 可写回的范围。
    #[serde(flatten)]
    pub scopes: ReadScopes,
    /// 账号每分钟的写额度，与历史读取额度分开。
    pub per_minute: u32,
}

/// 本账号在其它客户端的阅读事实。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadSyncCaps {
    /// 可查询的范围。
    #[serde(flatten)]
    pub scopes: ReadScopes,
    /// 可返回连续水位，使用消息 `readPosition` 的同一序数空间。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub watermark: bool,
    /// 可返回逐条已读事实；不蕴含任何前后消息已读。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub individual: bool,
}

/// 账号由已打开的插件会话限定；同一会话的话题彼此独立。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadScope {
    /// 不透明会话编号。
    pub conversation_id: String,
    /// 不透明话题根消息编号；省略为会话消息流。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread_root: Option<String>,
}

/// 单条消息的阅读摘要；省略计数表示未知，不表示零。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageReceipt {
    /// 当前请求中的本人消息。
    pub message_id: String,
    /// 确认已读的人数。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_count: Option<u32>,
    /// 平台给出的完整参与人数，不包含自己；没有就不能计算比例。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_count: Option<u32>,
    /// 这条消息当前可读已读名单。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub readers: bool,
    /// 这条消息当前可读未读名单。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub unreaders: bool,
}

/// 批量查询回执；只问可见的本人消息。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptsParams {
    /// 会话或话题。
    pub scope: ReadScope,
    /// 有界、不重复的消息编号。
    pub message_ids: Vec<String>,
}

/// 当前可核实的回执。未返回的编号仍为未知。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptsResult {
    /// 不超过请求范围。
    pub items: Vec<MessageReceipt>,
}

/// 只在用户打开名单后分页读取。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptPeopleParams {
    /// 会话或话题。
    pub scope: ReadScope,
    /// 本人发出的消息。
    pub message_id: String,
    /// true 读已读名单，false 读未读名单。
    pub read: bool,
    /// 不透明翻页游标。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    /// 1 到 `READ_PEOPLE_MAX`。
    pub limit: u16,
}

/// 名单的一页；没有下一页不一定意味着名单完整。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptPeopleResult {
    /// 已核实的人员。
    pub people: Vec<crate::Person>,
    /// 有下一页时的游标。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    /// 从首页到此页包含完整名单；部分权限时为 false。
    pub complete: bool,
}

/// 查询本账号的阅读状态，绝不能产生写副作用。
pub type ReadSyncParams = ReceiptsParams;

/// 单调合并的已读事实。平台主动标未读与旧结果不会让宿主倒退。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadState {
    /// 事实所属范围。
    pub scope: ReadScope,
    /// 连续水位；规范十进制非负整数字符串（无前导零），与消息 readPosition 对应。
    /// 插件将平台位置无损转换；不能用毫秒截断精确游标，不能从一条 is_read 推导水位。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub through: Option<String>,
    /// 平台逐条确认已读的消息；未列出不表示未读。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub read_message_ids: Vec<String>,
}

/// 用户本人看到消息后，显式写回确切边界。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkReadParams {
    /// 独立会话或话题。
    pub scope: ReadScope,
    /// 用户当时看到的消息，不能换成执行时最新一条。
    pub message_id: String,
    /// 这条消息的精确序数；没有可靠序数时省略，由插件核验编号。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<String>,
}

/// 写入结果。Unknown 需要只读核对，不能自动重放。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum MarkReadResult {
    /// 平台明确接受。
    Applied,
    /// 结果不确定。
    Unknown,
}

/// 是否是可精确比较的规范十进制序数。
pub fn valid_read_position(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= crate::budget::READ_POSITION_MAX_BYTES
        && value.bytes().all(|c| c.is_ascii_digit())
        && (value.len() == 1 || !value.starts_with('0'))
}

/// 精确比较，不转换成浮点数或毫秒。
pub fn compare_read_positions(a: &str, b: &str) -> std::cmp::Ordering {
    a.len().cmp(&b.len()).then_with(|| a.cmp(b))
}

/// 核对三项独立能力的内部一致性。
pub fn capabilities_problem(caps: &crate::Capabilities) -> Option<&'static str> {
    if caps
        .receipts
        .is_some_and(|c| c.scopes.is_empty() || c.max_age_secs == Some(0))
    {
        return Some("receipts 需要支持的范围和有效时限");
    }
    if caps
        .mark_read
        .is_some_and(|c| c.scopes.is_empty() || c.per_minute == 0)
    {
        return Some("markRead 需要支持的范围和正数写额度");
    }
    if caps
        .read_sync
        .is_some_and(|c| c.scopes.is_empty() || !(c.watermark || c.individual))
    {
        return Some("readSync 需要支持的范围和至少一种事实类型");
    }
    None
}

/// 编号仍是不透明字符串，只约束大小与控制字符。
pub fn valid_read_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= crate::budget::ID_MAX_BYTES
        && !value.chars().any(char::is_control)
}

/// 核对查询范围。
pub fn valid_read_scope(scope: &ReadScope) -> bool {
    valid_read_id(&scope.conversation_id) && scope.thread_root.as_deref().is_none_or(valid_read_id)
}

/// 核对有界、不重复的编号集合。
pub fn valid_read_ids(ids: &[String]) -> bool {
    ids.len() <= crate::budget::READ_MESSAGES_MAX
        && ids.iter().all(|s| valid_read_id(s))
        && ids.iter().collect::<std::collections::HashSet<_>>().len() == ids.len()
}

/// 通知与查询结果共用校验，不接受未声明或不可比较的水位。
pub fn valid_read_state(state: &ReadState, caps: &ReadSyncCaps) -> bool {
    valid_read_scope(&state.scope)
        && caps.scopes.allows(&state.scope)
        && state
            .through
            .as_deref()
            .is_none_or(|p| caps.watermark && valid_read_position(p))
        && (state.read_message_ids.is_empty() || caps.individual)
        && valid_read_ids(&state.read_message_ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reading_capabilities_are_independent_and_old_shapes_stay_absent() {
        let old = serde_json::to_value(crate::Capabilities::default()).unwrap();
        for key in ["receipts", "markRead", "readSync"] {
            assert!(old.get(key).is_none());
        }
        for json in [
            serde_json::json!({"receipts":{"conversations":true,"readers":true}}),
            serde_json::json!({"markRead":{"threads":true,"perMinute":30}}),
            serde_json::json!({"readSync":{"conversations":true,"individual":true}}),
        ] {
            let caps = serde_json::from_value(json).unwrap();
            assert_eq!(capabilities_problem(&caps), None);
        }
        let invalid =
            serde_json::from_value(serde_json::json!({"readSync":{"conversations":true}})).unwrap();
        assert!(capabilities_problem(&invalid).is_some());
    }
    #[test]
    fn reading_positions_do_not_lose_microseconds_or_large_integer_precision() {
        assert!(compare_read_positions("1800000000123001", "1800000000123002").is_lt());
        assert!(compare_read_positions("999999999999999999999", "1000000000000000000000").is_lt());
        for invalid in ["", "01", "-1", "1.01", "1e8", " 10"] {
            assert!(!valid_read_position(invalid));
        }
        assert!(valid_read_position("0"));
    }
    #[test]
    fn sparse_read_facts_cannot_claim_a_watermark_or_an_unrelated_scope() {
        let caps = ReadSyncCaps {
            scopes: ReadScopes {
                conversations: true,
                threads: false,
            },
            individual: true,
            watermark: false,
        };
        let mut state = ReadState {
            scope: ReadScope {
                conversation_id: "c1".into(),
                thread_root: None,
            },
            through: None,
            read_message_ids: vec!["m2".into()],
        };
        assert!(valid_read_state(&state, &caps));
        state.through = Some("2".into());
        assert!(!valid_read_state(&state, &caps));
        state.through = None;
        state.scope.thread_root = Some("root".into());
        assert!(!valid_read_state(&state, &caps));
    }
    #[test]
    fn reading_methods_round_trip() {
        for method in [
            crate::Method::ReadingReceipts,
            crate::Method::ReadingPeople,
            crate::Method::ReadingSync,
            crate::Method::ReadingMark,
            crate::Method::ReadingChanged,
        ] {
            assert_eq!(crate::Method::from_name(method.as_str()), Some(method));
        }
    }
}
