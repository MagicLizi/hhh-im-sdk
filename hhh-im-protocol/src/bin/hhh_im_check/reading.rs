//! 阅读自检：读查询和写授权分开，跳过与结果不确定均不能算真实验收通过。
use super::*;
use hhh_im_protocol::budget::READ_CURSOR_MAX_BYTES;
use hhh_im_protocol::reading::*;
use hhh_im_protocol::types::Message;

async fn request<T: serde::de::DeserializeOwned>(
    conn: &Arc<Connection>,
    method: &str,
    params: impl serde::Serialize,
) -> Result<T, String> {
    let value = call_t(
        conn,
        method,
        serde_json::to_value(params).map_err(|e| e.to_string())?,
        Duration::from_secs(30),
    )
    .await?;
    serde_json::from_value(value).map_err(|e| format!("{method} 响应格式不对：{e}"))
}

pub(super) async fn check(
    conn: &Arc<Connection>,
    report: &mut Report,
    caps: &Capabilities,
    me: Option<&str>,
    conversation: Option<&str>,
    mark: Option<&str>,
) {
    if caps.receipts.is_none() && caps.mark_read.is_none() && caps.read_sync.is_none() {
        report.skip("阅读能力", "未声明三项可选能力");
        return;
    }
    let Some(me) = me else {
        report.skip("阅读能力", "当前未登录，已声明能力仍需真实验收");
        return;
    };
    let Some(conversation_id) = conversation else {
        report.skip("阅读能力", "未指定 --reading-chat <会话编号>");
        return;
    };
    let messages: Value = match request(
        conn,
        "messages/list",
        json!({"conversationId":conversation_id,"newestFirst":true,"limit":100}),
    )
    .await
    {
        Ok(value) => value,
        Err(error) => {
            report.fail("阅读检查样本", error);
            return;
        }
    };
    let messages: Vec<Message> = match serde_json::from_value(messages["items"].clone()) {
        Ok(messages) => messages,
        Err(error) => {
            report.fail("阅读检查样本", error.to_string());
            return;
        }
    };
    let scope = ReadScope {
        conversation_id: conversation_id.into(),
        thread_root: None,
    };
    let messages: Vec<_> = messages
        .iter()
        .filter(|m| {
            m.conversation_id == conversation_id
                && m.thread.as_ref().is_none_or(|t| t.in_thread != Some(true))
        })
        .collect();
    let ids: Vec<_> = messages.iter().map(|m| m.id.clone()).collect();
    if let Some(read_sync) = &caps.read_sync {
        if read_sync.scopes.allows(&scope) {
            let requested = if read_sync.individual {
                ids.clone()
            } else {
                Vec::new()
            };
            match request::<ReadState>(
                conn,
                "reading/sync",
                ReadSyncParams {
                    scope: scope.clone(),
                    message_ids: requested.clone(),
                },
            )
            .await
            {
                Ok(facts)
                    if valid_read_state(&facts, read_sync)
                        && facts.scope == scope
                        && facts
                            .read_message_ids
                            .iter()
                            .all(|id| requested.contains(id)) =>
                {
                    report.pass("阅读回读只返回声明范围的精确事实")
                }
                Ok(_) => report.fail(
                    "阅读回读只返回声明范围的精确事实",
                    "回读越过请求范围或未声明粒度",
                ),
                Err(error) => report.fail("阅读回读只返回声明范围的精确事实", error),
            }
        } else {
            report.skip("阅读回读", "未声明会话粒度；话题须另用产品入口验证");
        }
    } else {
        report.skip("阅读回读", "未声明可选能力");
    }
    if let Some(receipts) = &caps.receipts {
        if receipts.scopes.allows(&scope) {
            let own: Vec<_> = messages
                .iter()
                .filter(|m| m.sender.id == me)
                .map(|m| m.id.clone())
                .collect();
            if own.is_empty() {
                report.skip("阅读回执", "指定会话样本中没有本人消息");
            } else {
                match request::<ReceiptsResult>(
                    conn,
                    "reading/receipts",
                    ReceiptsParams {
                        scope: scope.clone(),
                        message_ids: own.clone(),
                    },
                )
                .await
                {
                    Ok(result) => {
                        let mut seen = std::collections::HashSet::new();
                        let valid = result.items.len() <= own.len()
                            && result.items.iter().all(|r| {
                                own.contains(&r.message_id)
                                    && seen.insert(&r.message_id)
                                    && r.read_count
                                        .zip(r.total_count)
                                        .is_none_or(|(read, total)| read <= total)
                                    && (!r.readers || receipts.readers)
                                    && (!r.unreaders || receipts.unreaders)
                            });
                        if valid {
                            report.pass("阅读回执不补造人数和分母");
                            if result.items.iter().all(|r| r.read_count.is_none()) {
                                report.skip("阅读回执真实数据", "本次均未知，不能当作已验证可用");
                            }
                            for read in [true, false] {
                                if let Some(item) = result
                                    .items
                                    .iter()
                                    .find(|r| if read { r.readers } else { r.unreaders })
                                {
                                    let query = ReceiptPeopleParams {
                                        scope: scope.clone(),
                                        message_id: item.message_id.clone(),
                                        read,
                                        cursor: None,
                                        limit: 50,
                                    };
                                    let name = if read {
                                        "阅读已读名单"
                                    } else {
                                        "阅读未读名单"
                                    };
                                    match request::<ReceiptPeopleResult>(
                                        conn,
                                        "reading/people",
                                        query,
                                    )
                                    .await
                                    {
                                        Ok(page)
                                            if page.people.len() <= 50
                                                && page.people.iter().all(|p| !p.id.is_empty())
                                                && page.next_cursor.as_ref().is_none_or(
                                                    |cursor| {
                                                        !cursor.is_empty()
                                                            && cursor.len() <= READ_CURSOR_MAX_BYTES
                                                    },
                                                )
                                                && !(page.complete
                                                    && page.next_cursor.is_some()) =>
                                        {
                                            report.pass(name)
                                        }
                                        Ok(_) => report.fail(name, "分页或名单越过协议边界"),
                                        Err(error) => report.fail(name, error),
                                    }
                                }
                            }
                        } else {
                            report.fail(
                                "阅读回执不补造人数和分母",
                                "返回了请求外消息、重复项或未声明内容",
                            );
                        }
                    }
                    Err(error) => report.fail("阅读回执不补造人数和分母", error),
                }
            }
        } else {
            report.skip("阅读回执", "未声明会话粒度；话题须另用产品入口验证");
        }
    } else {
        report.skip("阅读回执", "未声明可选能力");
    }
    let Some(mark_caps) = &caps.mark_read else {
        report.skip("阅读写回", "未声明可选能力");
        return;
    };
    let Some(id) = mark else {
        report.skip(
            "阅读写回",
            "未给 --allow-mark-read <确切消息编号>，只读检查不会写入",
        );
        return;
    };
    if !mark_caps.scopes.allows(&scope) {
        report.skip("阅读写回", "未声明会话粒度");
        return;
    }
    let Some(message) = messages.iter().find(|m| m.id == id) else {
        report.fail("阅读写回", "授权消息不在取到的会话样本中，未写入");
        return;
    };
    match request::<MarkReadResult>(
        conn,
        "reading/mark",
        MarkReadParams {
            scope,
            message_id: id.into(),
            position: message.read_position.clone(),
        },
    )
    .await
    {
        Ok(MarkReadResult::Applied) => report.pass("阅读写回指定边界"),
        Ok(MarkReadResult::Unknown) => {
            report.fail("阅读写回指定边界", "平台结果不确定，未重试；须只读对账")
        }
        Err(error) => report.fail("阅读写回指定边界", error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hhh_im_protocol::transport::LinePipe;

    #[tokio::test]
    async fn reading_check_does_not_call_undeclared_or_unconfigured_platforms() {
        let (writer, _reader) = tokio::io::duplex(4096);
        let (_writer, reader) = tokio::io::duplex(4096);
        let conn = Connection::new(Arc::new(LinePipe::new(writer, reader, || {})));
        let mut report = Report::new();
        let mut caps = Capabilities::default();
        check(
            &conn,
            &mut report,
            &caps,
            Some("me"),
            Some("chat"),
            Some("m"),
        )
        .await;
        assert!(
            report
                .rows
                .iter()
                .all(|(_, status, _)| *status == Status::Skip)
        );
        caps.mark_read = Some(MarkReadCaps {
            scopes: ReadScopes {
                conversations: true,
                threads: false,
            },
            per_minute: 10,
        });
        check(&conn, &mut report, &caps, None, Some("chat"), Some("m")).await;
        check(&conn, &mut report, &caps, Some("me"), None, Some("m")).await;
        assert!(
            report
                .rows
                .iter()
                .all(|(_, status, _)| *status == Status::Skip)
        );
    }
}
