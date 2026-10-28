//! 转发快照的接收预算。未知的内部内容保留占位，不丢掉同一记录中可读的消息。

use serde::Deserialize;
use serde_json::{Value, json};

use crate::{budget::*, types::Block};

// 给数组分隔符、完整性字段与错误占位预留空间，使输出也保持在 JSON 字节预算里。
const NODE_OVERHEAD_BYTES: usize = 256;

#[derive(Default)]
struct Budget {
    messages: usize,
    blocks: usize,
    bytes: usize,
}

fn unavailable(kind: &str) -> Value {
    json!({"type":"unsupported", "kind":kind, "label":"部分内容无法读取"})
}

fn optional_string(
    object: &mut serde_json::Map<String, Value>,
    key: &str,
    max_bytes: usize,
) -> bool {
    if object.get(key).is_none_or(|value| {
        value.is_null() || value.as_str().is_some_and(|text| text.len() <= max_bytes)
    }) {
        return false;
    }
    object.remove(key);
    true
}

// 可选元数据坏了只去掉该字段，正文仍保留；不能因未知的身份种类丢掉整条可读快照。
fn message_metadata(message: &mut serde_json::Map<String, Value>) -> bool {
    let mut partial = optional_string(message, "id", ID_MAX_BYTES);
    if message
        .get("createdAtMs")
        .is_some_and(|value| !value.is_null() && value.as_i64().is_none())
    {
        message.remove("createdAtMs");
        partial = true;
    }
    if message
        .get("incomplete")
        .is_some_and(|value| !value.is_boolean())
    {
        message.remove("incomplete");
        partial = true;
    }
    if message
        .get("sender")
        .is_some_and(|value| !value.is_object())
    {
        message.remove("sender");
        partial = true;
    }
    if let Some(sender) = message.get_mut("sender").and_then(Value::as_object_mut) {
        for field in ["id", "idType", "avatar"] {
            partial |= optional_string(sender, field, ID_MAX_BYTES);
        }
        partial |= optional_string(sender, "name", TEXT_BLOCK_MAX_BYTES);
        if sender.get("kind").is_some_and(|value| {
            !value.is_null() && !matches!(value.as_str(), Some("user" | "bot" | "system"))
        }) {
            sender.remove("kind");
            partial = true;
        }
    }
    partial
}

/// 核对消息内容中的转发树并应用预算；返回是否存在不可读或被截断的内容。
///
/// 宿主在反序列化线上类型之前调用；插件也可在交付快照之前调用。只遍历协议内容，
/// 不解释 raw、平台编号或任意卡片字段。未知普通块仍交给宿主既有的兼容入口处理。
pub fn sanitize_content(content: &mut Value) -> bool {
    let Some(blocks) = content.as_array_mut() else {
        return false;
    };
    let mut budget = Budget::default();
    blocks.iter_mut().fold(false, |changed, block| {
        let incomplete = forwarded(block, 1, &mut budget);
        changed || incomplete
    })
}

fn forwarded(block: &mut Value, depth: usize, budget: &mut Budget) -> bool {
    if block.get("type").and_then(Value::as_str) != Some("forwarded") {
        return false;
    }
    if depth > FORWARDED_DEPTH_MAX {
        *block = unavailable("forwardedDepth");
        return true;
    }
    let Some(record) = block.get_mut("record").and_then(Value::as_object_mut) else {
        *block = unavailable("forwarded");
        return true;
    };
    let mut incomplete = record
        .get("incomplete")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if record
        .get("incomplete")
        .is_some_and(|value| !value.is_boolean())
    {
        record.remove("incomplete");
        incomplete = true;
    }
    let title_ok = record
        .get("title")
        .is_none_or(|v| v.as_str().is_some_and(|s| s.len() <= TEXT_BLOCK_MAX_BYTES));
    if !title_ok {
        record.insert("title".into(), json!("聊天记录"));
        incomplete = true;
    }
    let mut messages = match record.remove("messages") {
        Some(Value::Array(messages)) => messages,
        None => {
            incomplete = true;
            Vec::new()
        }
        _ => {
            incomplete = true;
            Vec::new()
        }
    };
    let record_bytes = serde_json::to_vec(&record)
        .map_or(FORWARDED_BYTES_MAX, |v| v.len())
        .saturating_add(NODE_OVERHEAD_BYTES);
    if budget.bytes.saturating_add(record_bytes) > FORWARDED_BYTES_MAX {
        *block = unavailable("forwardedBudget");
        return true;
    }
    budget.bytes += record_bytes;
    if messages.len() > FORWARDED_MESSAGES_PER_RECORD_MAX {
        messages.truncate(FORWARDED_MESSAGES_PER_RECORD_MAX);
        incomplete = true;
    }
    messages.retain_mut(|message| {
        if budget.messages >= FORWARDED_MESSAGES_MAX || budget.bytes >= FORWARDED_BYTES_MAX {
            incomplete = true;
            return false;
        }
        budget.messages += 1;
        if !message.is_object() {
            *message = json!({"content":[unavailable("forwardedMessage")],"incomplete":true});
        }
        let metadata = message.as_object_mut().expect("object checked");
        let mut partial = message_metadata(metadata);
        let mut content = metadata.remove("content").unwrap_or(Value::Null);
        // 元数据同样算入字节预算；避免巨大的名字、编号或未知字段绕过正文上限。
        let metadata_bytes = serde_json::to_vec(message)
            .map_or(FORWARDED_BYTES_MAX, |v| v.len())
            .saturating_add(NODE_OVERHEAD_BYTES);
        if budget.bytes.saturating_add(metadata_bytes) > FORWARDED_BYTES_MAX {
            incomplete = true;
            return false;
        }
        budget.bytes += metadata_bytes;
        if let Some(parts) = content.as_array_mut() {
            if parts.len() > BLOCKS_PER_MESSAGE_MAX {
                parts.truncate(BLOCKS_PER_MESSAGE_MAX);
                partial = true;
            }
            parts.retain_mut(|part| {
                if budget.blocks >= FORWARDED_BLOCKS_MAX {
                    partial = true;
                    return false;
                }
                budget.blocks += 1;
                partial |= forwarded(part, depth + 1, budget);
                if Block::deserialize(&*part).is_err() {
                    *part = unavailable("unknown");
                    partial = true;
                }
                if part.get("type").and_then(Value::as_str) == Some("unsupported") {
                    partial = true;
                }
                // 嵌套正文已逐块计数，不重复计算整个子树。
                if part.get("type").and_then(Value::as_str) != Some("forwarded") {
                    let bytes = serde_json::to_vec(&*part).map_or(FORWARDED_BYTES_MAX, |v| v.len());
                    if budget.bytes.saturating_add(NODE_OVERHEAD_BYTES) > FORWARDED_BYTES_MAX {
                        partial = true;
                        return false;
                    }
                    if bytes > TEXT_BLOCK_MAX_BYTES
                        || budget
                            .bytes
                            .saturating_add(bytes)
                            .saturating_add(NODE_OVERHEAD_BYTES)
                            > FORWARDED_BYTES_MAX
                    {
                        *part = unavailable("forwardedBudget");
                        budget.bytes += NODE_OVERHEAD_BYTES;
                        partial = true;
                    } else {
                        budget.bytes += bytes + NODE_OVERHEAD_BYTES;
                    }
                }
                true
            });
        } else {
            content = json!([unavailable("empty")]);
            partial = true;
        }
        message["content"] = content;
        if crate::types::ForwardedMessage::deserialize(&*message).is_err() {
            *message = json!({"content":[unavailable("forwardedMessage")],"incomplete":true});
            partial = true;
        }
        if partial {
            message["incomplete"] = json!(true);
        }
        incomplete |= partial || message.get("incomplete").and_then(Value::as_bool) == Some(true);
        true
    });
    if record.get("totalCount").is_some_and(|total| {
        !total.is_null()
            && total
                .as_u64()
                .is_none_or(|n| n > u32::MAX as u64 || n < messages.len() as u64)
    }) {
        record.remove("totalCount");
        incomplete = true;
    }
    if record
        .get("totalCount")
        .and_then(Value::as_u64)
        .is_some_and(|total| total > messages.len() as u64)
    {
        incomplete = true;
    }
    record.insert("messages".into(), Value::Array(messages));
    if incomplete {
        record.insert("incomplete".into(), json!(true));
    }
    incomplete
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(content: Value) -> Value {
        json!({"type":"forwarded","record":{"title":"讨论","messages":[{"sender":{"name":"原发送人"},"createdAtMs":3,"content":content}]}})
    }

    #[test]
    fn unknown_snapshot_parts_keep_neighbours_and_order() {
        let mut content = json!([record(
            json!([{"type":"text","text":"前"},{"type":"future"},{"type":"text","text":"后"}])
        )]);
        assert!(sanitize_content(&mut content));
        let blocks: Vec<Block> = serde_json::from_value(content).unwrap();
        let Block::Forwarded { record } = &blocks[0] else {
            panic!()
        };
        assert!(record.incomplete);
        assert_eq!(record.messages[0].sender.name.as_deref(), Some("原发送人"));
        assert_eq!(record.messages[0].created_at_ms, Some(3));
        assert_eq!(record.messages[0].content.len(), 3);
        assert!(matches!(
            record.messages[0].content[1],
            Block::Unsupported { .. }
        ));
        assert_eq!(record.total_count, None);
    }

    #[test]
    fn invalid_metadata_does_not_discard_readable_neighbours() {
        let mut content = json!([record(json!([{"type":"text","text":"仍可读取"}]))]);
        content[0]["record"]["totalCount"] = json!("unknown");
        content[0]["record"]["messages"].as_array_mut().unwrap().push(json!({"id":42,"sender":{"kind":"future","name":"保留的名字"},"createdAtMs":"bad","content":[{"type":"text","text":"无效身份的可读正文"}]}));
        assert!(sanitize_content(&mut content));
        let blocks: Vec<Block> = serde_json::from_value(content).unwrap();
        let Block::Forwarded { record } = &blocks[0] else {
            panic!()
        };
        assert_eq!(record.total_count, None);
        assert_eq!(record.messages.len(), 2);
        assert!(
            matches!(&record.messages[0].content[0], Block::Text { text } if text == "仍可读取")
        );
        assert!(record.messages[1].incomplete);
        assert_eq!(record.messages[1].sender.kind, None);
        assert_eq!(
            record.messages[1].sender.name.as_deref(),
            Some("保留的名字")
        );
        assert_eq!(record.messages[1].id, None);
        assert_eq!(record.messages[1].created_at_ms, None);
        assert!(
            matches!(&record.messages[1].content[0], Block::Text { text } if text == "无效身份的可读正文")
        );
    }

    #[test]
    fn excessive_depth_and_length_are_explicitly_partial() {
        let mut nested = json!({"type":"text","text":"终点"});
        for _ in 0..FORWARDED_DEPTH_MAX + 2 {
            nested = record(json!([nested]));
        }
        let mut content = json!([nested]);
        assert!(sanitize_content(&mut content));
        let _: Vec<Block> = serde_json::from_value(content).unwrap();
        let mut long = record(json!([{"type":"text","text":"x".repeat(TEXT_BLOCK_MAX_BYTES+1)}]));
        long["record"]["messages"] = json!(vec![
            long["record"]["messages"][0].clone();
            FORWARDED_MESSAGES_PER_RECORD_MAX + 1
        ]);
        let mut content = json!([long]);
        assert!(sanitize_content(&mut content));
        assert_eq!(
            content[0]["record"]["messages"].as_array().unwrap().len(),
            FORWARDED_MESSAGES_PER_RECORD_MAX
        );
        assert_eq!(content[0]["record"]["incomplete"], true);
    }
}
