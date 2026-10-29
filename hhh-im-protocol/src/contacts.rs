//! 群成员名片：读取关系不产生副作用，添加必须由用户明确触发。
use serde::{Deserialize, Serialize};

/// 请求始终携带来源群和目标账号；插件必须核验双方仍在这个群中。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContactTarget {
    pub conversation_id: String,
    pub user_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContactRelationship {
    SelfAccount,
    Contact,
    NotContact,
}

/// 在线查询的关系事实。无法读取时返回错误，不能回 NotContact。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContactProfile {
    pub relationship: ContactRelationship,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
    #[serde(default)]
    pub can_add: bool,
}

pub fn valid_target(target: &ContactTarget) -> bool {
    [&target.conversation_id, &target.user_id]
        .into_iter()
        .all(|id| !id.trim().is_empty() && id.len() <= 4096 && !id.chars().any(char::is_control))
}

pub fn valid_profile(profile: &ContactProfile) -> bool {
    match profile.relationship {
        ContactRelationship::Contact => {
            !profile.can_add
                && profile.conversation_id.as_ref().is_some_and(|id| {
                    !id.trim().is_empty() && id.len() <= 4096 && !id.chars().any(char::is_control)
                })
        }
        ContactRelationship::SelfAccount => !profile.can_add && profile.conversation_id.is_none(),
        ContactRelationship::NotContact => profile.conversation_id.is_none(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn contact_actions_are_opt_in_and_method_names_round_trip() {
        use crate::{method::Method, platform::PeopleCaps};
        let old: PeopleCaps = serde_json::from_str("{}").unwrap();
        assert!(!old.contact_actions);
        assert!(
            serde_json::to_value(old)
                .unwrap()
                .get("contactActions")
                .is_none()
        );
        for method in [Method::ContactsProfile, Method::ContactsAdd] {
            assert_eq!(Method::from_name(method.as_str()), Some(method));
        }
    }
    #[test]
    fn contact_profiles_never_confuse_unknown_or_inconsistent_relations() {
        assert!(serde_json::from_str::<ContactProfile>(r#"{"relationship":"unknown"}"#).is_err());
        assert!(!valid_profile(&ContactProfile {
            relationship: ContactRelationship::Contact,
            conversation_id: None,
            can_add: false
        }));
        assert!(!valid_profile(&ContactProfile {
            relationship: ContactRelationship::SelfAccount,
            conversation_id: None,
            can_add: true
        }));
        assert!(valid_profile(&ContactProfile {
            relationship: ContactRelationship::NotContact,
            conversation_id: None,
            can_add: false
        }));
    }
    #[test]
    fn contact_targets_require_bounded_real_group_and_user_ids() {
        assert!(valid_target(&ContactTarget {
            conversation_id: "group".into(),
            user_id: "person".into()
        }));
        assert!(!valid_target(&ContactTarget {
            conversation_id: " ".into(),
            user_id: "person".into()
        }));
        assert!(!valid_target(&ContactTarget {
            conversation_id: "group".into(),
            user_id: "x\n".into()
        }));
    }
}
