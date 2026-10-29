//! 链接分享卡片的网页地址边界；插件和宿主不执行卡片里的其它协议。

/// 可选的封面损坏不应吞掉仍能阅读的文章；不把原始资源字段写入日志。
pub(crate) fn thumbnail<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<crate::Resource>, D::Error> {
    use serde::Deserialize;
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(value.and_then(|value| match serde_json::from_value(value) {
        Ok(resource) => Some(resource),
        Err(_) => {
            tracing::warn!("链接卡片封面格式无效，保留文章文字与地址");
            None
        }
    }))
}

/// 只接受带主机名的 HTTP/HTTPS 地址，不修补任意协议或控制字符。
/// 实际打开时仍使用宿主既有的网页链接入口。
pub fn is_web_url(url: &str) -> bool {
    let Some((scheme, rest)) = url.split_once("://") else {
        return false;
    };
    (scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https"))
        && !url
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || c == '\\')
        && rest
            .split(['/', '?', '#'])
            .next()
            .is_some_and(|host| !host.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Block;
    use serde_json::json;

    #[test]
    fn link_cards_round_trip_without_optional_fields_or_with_a_thumbnail() {
        for value in [
            json!({"type":"linkCard","url":"https://example.com/article"}),
            json!({"type":"linkCard","url":"https://example.com/article?a=1&b=2",
                "title":"标题 <正文>","description":"摘要","source":"来源",
                "thumbnail":{"key":"article-cover:1","kind":"image"}}),
        ] {
            let block: Block = serde_json::from_value(value.clone()).unwrap();
            assert_eq!(serde_json::to_value(block).unwrap(), value);
        }
        assert!(
            serde_json::from_value::<Block>(json!({"type":"linkCard","title":"无地址"})).is_err()
        );
        for thumbnail in [json!({"key":"x","kind":"unknown"}), json!({}), json!("bad")] {
            let block: Block = serde_json::from_value(
                json!({"type":"linkCard","url":"https://example.com", "thumbnail":thumbnail}),
            )
            .unwrap();
            assert!(matches!(
                block,
                Block::LinkCard {
                    thumbnail: None,
                    ..
                }
            ));
        }
    }

    #[test]
    fn link_cards_only_open_web_addresses() {
        for url in ["https://example.com/a?q=1&b=2#part", "HTTP://example.com"] {
            assert!(is_web_url(url), "{url}");
        }
        for url in [
            "javascript:alert(1)",
            "file:///tmp/article",
            "data:text/html,a",
            "https://",
            "https:///path",
            "https://x\n.y",
            " https://x",
            "https://x\\y",
        ] {
            assert!(!is_web_url(url), "{url}");
        }
    }
}
