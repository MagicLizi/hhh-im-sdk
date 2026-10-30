//! Optional explicit friend approval, scoped to the connected account.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(
    tag = "operation",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ContactRequestsCommand {
    List {
        incoming: bool,
        cursor: Option<String>,
        user_id: Option<String>,
    },
    Send {
        request_id: String,
        conversation_id: String,
        user_id: String,
        message: String,
    },
    Resend {
        request_id: String,
        new_request_id: String,
        message: String,
    },
    Respond {
        request_id: String,
        action: ContactRequestAction,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContactRequestAction {
    Accept,
    Reject,
    Withdraw,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContactRequestState {
    Pending,
    Accepted,
    Rejected,
    Withdrawn,
    Expired,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContactRequest {
    pub request_id: String,
    pub incoming: bool,
    pub user_id: String,
    pub name: String,
    pub message: Option<String>,
    pub source_conversation_id: Option<String>,
    pub source_label: String,
    pub state: ContactRequestState,
    pub created_ms: i64,
    pub updated_ms: i64,
    pub expires_ms: i64,
    pub retry_after_ms: Option<i64>,
    pub conversation_id: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContactRequestsResult {
    pub items: Vec<ContactRequest>,
    pub next: Option<String>,
    pub pending: u32,
}

impl ContactRequestsCommand {
    pub fn valid(&self) -> bool {
        let id =
            |s: &str| !s.trim().is_empty() && s.len() <= 4096 && !s.chars().any(char::is_control);
        match self {
            Self::List {
                cursor, user_id, ..
            } => cursor.as_ref().is_none_or(|s| id(s)) && user_id.as_ref().is_none_or(|s| id(s)),
            Self::Send {
                request_id,
                conversation_id,
                user_id,
                message,
            } => {
                id(request_id)
                    && id(conversation_id)
                    && id(user_id)
                    && message.chars().count() <= 80
                    && !message.chars().any(|c| c.is_control() && c != '\n')
            }
            Self::Respond { request_id, .. } => id(request_id),
            Self::Resend {
                request_id,
                new_request_id,
                message,
            } => {
                id(request_id)
                    && id(new_request_id)
                    && message.chars().count() <= 80
                    && !message.chars().any(|c| c.is_control() && c != '\n')
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn optional_capability_and_untrusted_commands() {
        let caps: crate::platform::Capabilities = serde_json::from_str("{}").unwrap();
        assert!(!caps.people.friend_requests);
        let mut cmd = ContactRequestsCommand::Send {
            request_id: "request".into(),
            conversation_id: "group".into(),
            user_id: "peer".into(),
            message: "🙂".repeat(80),
        };
        assert!(cmd.valid());
        if let ContactRequestsCommand::Send { message, .. } = &mut cmd {
            message.push('🙂');
        }
        assert!(!cmd.valid());
        assert!(
            serde_json::from_str::<ContactRequestsCommand>(
                r#"{"operation":"respond","requestId":"r","action":"accept","userId":"other"}"#
            )
            .is_err()
        );
        assert!(
            !ContactRequestsCommand::List {
                incoming: true,
                cursor: Some("\n".into()),
                user_id: None
            }
            .valid()
        );
    }
}
