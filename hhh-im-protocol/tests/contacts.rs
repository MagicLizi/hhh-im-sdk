use hhh_im_protocol::{Capabilities, method::Method};
use serde_json::json;

#[test]
fn contact_listing_is_opt_in_and_round_trips_without_changing_old_plugins() {
    let legacy: Capabilities = serde_json::from_value(json!({})).unwrap();
    assert!(
        serde_json::to_value(legacy).unwrap()["people"]
            .get("contacts")
            .is_none()
    );
    let capable: Capabilities =
        serde_json::from_value(json!({"people":{"contacts":true}})).unwrap();
    assert_eq!(
        serde_json::to_value(capable).unwrap()["people"]["contacts"],
        true
    );
    let method = Method::from_name("contacts/list").expect("contact directory has its own method");
    assert_eq!(method.as_str(), "contacts/list");
}
