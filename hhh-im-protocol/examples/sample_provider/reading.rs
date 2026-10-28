//! 示例阅读平台：三项可单独关闭。可选共享事实文件只用于两个隔离示例实例的验收。
use super::{Out, State, error, notify};
use hhh_im_protocol::budget::READ_PEOPLE_MAX;
use hhh_im_protocol::reading::*;
use hhh_im_protocol::rpc::RpcError;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Default)]
pub(super) struct Reading {
    pub events: bool,
    pub shared: Option<PathBuf>,
    pub(super) facts: HashMap<ReadScope, ReadState>,
}

pub(super) fn declare(result: &mut Value, plain: bool) {
    if plain {
        return;
    }
    for (field, disabled, declaration) in [
        (
            "receipts",
            "no-receipts",
            json!({"conversations":true,"threads":true,"readers":true,"unreaders":true}),
        ),
        (
            "markRead",
            "no-mark-read",
            json!({"conversations":true,"threads":true,"perMinute":60}),
        ),
        (
            "readSync",
            "no-read-sync",
            json!({"conversations":true,"threads":true,"watermark":true,"individual":true}),
        ),
    ] {
        if !Path::new(disabled).exists() {
            result["capabilities"][field] = declaration;
        }
    }
}

fn params<T: serde::de::DeserializeOwned>(value: &Value) -> Result<T, RpcError> {
    serde_json::from_value(value.clone()).map_err(|_| error("invalidRequest", "阅读请求格式不对。"))
}
fn value(value: impl serde::Serialize) -> Result<Value, RpcError> {
    serde_json::to_value(value).map_err(|_| error("internal", "阅读结果无法编码。"))
}
fn belongs(message: &Value, scope: &ReadScope) -> bool {
    message["conversationId"].as_str() == Some(&scope.conversation_id)
        && match scope.thread_root.as_deref() {
            Some(root) => {
                message["thread"]["inThread"] == true
                    && message["thread"]["rootId"].as_str() == Some(root)
            }
            None => message["thread"]["inThread"] != true || message["thread"]["broadcast"] == true,
        }
}
fn merge(old: &mut ReadState, next: ReadState) {
    if let Some(next) = next.through
        && old
            .through
            .as_ref()
            .is_none_or(|previous| compare_read_positions(&next, previous).is_gt())
    {
        old.through = Some(next);
    }
    for id in next.read_message_ids {
        if !old.read_message_ids.contains(&id) {
            old.read_message_ids.push(id);
        }
    }
}
impl Reading {
    /// 两个隔离示例客户端读同一份模拟平台消息；不触碰真实平台或宿主聊天库。
    pub(super) fn refresh_messages(&self, messages: &mut Vec<Value>) -> std::io::Result<()> {
        let Some(shared) = &self.shared else {
            return Ok(());
        };
        let path = shared.with_extension("messages.jsonl");
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => {
                for message in messages.iter() {
                    serde_json::to_writer(&mut file, message)?;
                    file.write_all(b"\n")?;
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
        for line in std::fs::read_to_string(path)?
            .lines()
            .filter(|line| !line.is_empty())
        {
            let message: Value = serde_json::from_str(line)?;
            if let Some(old) = messages.iter_mut().find(|m| {
                m["id"] == message["id"] && m["conversationId"] == message["conversationId"]
            }) {
                *old = message;
            } else {
                messages.push(message);
            }
        }
        Ok(())
    }

    pub(super) fn publish_message(&self, message: &Value) -> std::io::Result<()> {
        let Some(shared) = &self.shared else {
            return Ok(());
        };
        let mut bytes = serde_json::to_vec(message)?;
        bytes.push(b'\n');
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(shared.with_extension("messages.jsonl"))?
            .write_all(&bytes)
    }

    fn read(&self, scope: &ReadScope) -> Result<ReadState, RpcError> {
        let mut fact = self.facts.get(scope).cloned().unwrap_or(ReadState {
            scope: scope.clone(),
            through: None,
            read_message_ids: Vec::new(),
        });
        if let Some(path) = &self.shared {
            let source = match std::fs::read_to_string(path) {
                Ok(source) => source,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
                Err(e) => {
                    eprintln!("sample reading state: {e}");
                    return Err(error("temporary", "示例阅读事实读不到。"));
                }
            };
            for line in source.lines().filter(|line| !line.is_empty()) {
                let next: ReadState = serde_json::from_str(line)
                    .map_err(|_| error("invalidRequest", "示例阅读事实格式不对。"))?;
                if next.scope == *scope {
                    merge(&mut fact, next);
                }
            }
        }
        Ok(fact)
    }
    fn save(&mut self, fact: ReadState) -> Result<(), RpcError> {
        if let Some(path) = &self.shared {
            let mut bytes = serde_json::to_vec(&fact)
                .map_err(|_| error("internal", "示例阅读事实无法编码。"))?;
            bytes.push(b'\n');
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .and_then(|mut file| file.write_all(&bytes))
                .map_err(|e| {
                    eprintln!("sample reading save: {e}");
                    error("temporary", "示例阅读事实存不下来。")
                })?;
        }
        let old = self.facts.entry(fact.scope.clone()).or_insert(ReadState {
            scope: fact.scope.clone(),
            through: None,
            read_message_ids: Vec::new(),
        });
        merge(old, fact);
        Ok(())
    }
}

pub(super) fn handle(
    method: &str,
    input: &Value,
    state: &mut State,
    out: &Out,
) -> Result<Value, RpcError> {
    let disabled = match method {
        "reading/receipts" | "reading/people" => "no-receipts",
        "reading/mark" => "no-mark-read",
        _ => "no-read-sync",
    };
    if state.plain || Path::new(disabled).exists() {
        return Err(error("unsupported", "示例没有声明这项阅读能力。"));
    }
    match method {
        "reading/receipts" => {
            let query: ReceiptsParams = params(input)?;
            if !valid_read_scope(&query.scope) || !valid_read_ids(&query.message_ids) {
                return Err(error("invalidRequest", "回执查询范围不对。"));
            }
            let unknown = Path::new("reading-unknown").exists();
            let counts = Path::new("reading-counts").exists();
            let all = Path::new("reading-all").exists();
            let long = Path::new("reading-long").exists();
            let items = state
                .messages
                .iter()
                .filter(|m| {
                    belongs(m, &query.scope)
                        && m["sender"]["id"] == "me"
                        && query
                            .message_ids
                            .iter()
                            .any(|id| m["id"].as_str() == Some(id))
                })
                .map(|m| MessageReceipt {
                    message_id: m["id"].as_str().unwrap().into(),
                    read_count: (!unknown).then_some(if all {
                        4
                    } else if long {
                        80
                    } else {
                        2
                    }),
                    total_count: (!unknown && !counts).then_some(if long { 84 } else { 4 }),
                    readers: !unknown && !counts,
                    unreaders: !unknown && !counts,
                })
                .collect();
            value(ReceiptsResult { items })
        }
        "reading/people" => {
            let query: ReceiptPeopleParams = params(input)?;
            if Path::new("deny-reading-people").exists() {
                return Err(error("permissionDenied", "示例名单暂时不可读。"));
            }
            if !valid_read_scope(&query.scope)
                || query.limit == 0
                || usize::from(query.limit) > READ_PEOPLE_MAX
                || !state.messages.iter().any(|m| {
                    belongs(m, &query.scope)
                        && m["id"] == query.message_id
                        && m["sender"]["id"] == "me"
                })
            {
                return Err(error("invalidRequest", "名单查询范围不对。"));
            }
            let all = Path::new("reading-all").exists();
            let long = Path::new("reading-long").exists();
            let count = if query.read {
                if all {
                    4
                } else if long {
                    80
                } else {
                    2
                }
            } else if all {
                0
            } else if long {
                4
            } else {
                2
            };
            let offset = query
                .cursor
                .as_deref()
                .unwrap_or("0")
                .parse::<usize>()
                .map_err(|_| error("invalidRequest", "名单游标不对。"))?;
            let end = count.min(offset.saturating_add(usize::from(query.limit)));
            let people = (offset..end)
                .map(|index| hhh_im_protocol::types::Person {
                    id: format!("reading-{}-{index}", query.read),
                    kind: hhh_im_protocol::types::PersonKind::User,
                    name: Some(if index == 0 {
                        "名字很长的同事也保持一行".into()
                    } else {
                        format!("同事 {}", index + 1)
                    }),
                    id_type: None,
                    avatar: None,
                })
                .collect();
            value(ReceiptPeopleResult {
                people,
                next_cursor: (end < count).then(|| end.to_string()),
                complete: end == count && !Path::new("reading-partial").exists(),
            })
        }
        "reading/sync" => {
            let query: ReadSyncParams = params(input)?;
            if !valid_read_scope(&query.scope) || !valid_read_ids(&query.message_ids) {
                return Err(error("invalidRequest", "回读范围不对。"));
            }
            let mut fact = state.reading.read(&query.scope)?;
            fact.read_message_ids
                .retain(|id| query.message_ids.contains(id));
            value(fact)
        }
        "reading/mark" => {
            let query: MarkReadParams = params(input)?;
            let message = state
                .messages
                .iter()
                .find(|m| belongs(m, &query.scope) && m["id"] == query.message_id)
                .ok_or_else(|| error("notFound", "这条消息不在这个阅读范围。"))?;
            let position = message["readPosition"].as_str();
            if query
                .position
                .as_deref()
                .is_some_and(|given| Some(given) != position)
            {
                return Err(error("invalidRequest", "阅读边界与消息不一致。"));
            }
            let fact = ReadState {
                scope: query.scope,
                through: position.map(str::to_owned),
                read_message_ids: if position.is_none() {
                    vec![query.message_id]
                } else {
                    Vec::new()
                },
            };
            state.reading.save(fact.clone())?;
            if state.reading.events {
                notify(out, "reading/changed", value(fact)?);
            }
            value(if Path::new("reading-write-unknown").exists() {
                MarkReadResult::Unknown
            } else {
                MarkReadResult::Applied
            })
        }
        _ => Err(error("unsupported", "没有这个阅读方法。")),
    }
}
