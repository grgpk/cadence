use axum::http::HeaderMap;

pub fn extract_client_ip(headers: &HeaderMap) -> Option<String> {
    header_value(headers, "cf-connecting-ip")
        .or_else(|| {
            header_value(headers, "x-forwarded-for")
                .and_then(|value| value.split(',').next())
                .map(str::trim)
        })
        .or_else(|| header_value(headers, "x-real-ip"))
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub fn header_value<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderName, HeaderValue};

    use super::*;

    #[test]
    fn prefers_cloudflare_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("cf-connecting-ip", HeaderValue::from_static("1.1.1.1"));
        headers.insert("x-forwarded-for", HeaderValue::from_static("2.2.2.2"));
        assert_eq!(extract_client_ip(&headers).as_deref(), Some("1.1.1.1"));
    }

    #[test]
    fn uses_first_forwarded_ip() {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("x-forwarded-for"),
            HeaderValue::from_static("3.3.3.3, 4.4.4.4"),
        );
        assert_eq!(extract_client_ip(&headers).as_deref(), Some("3.3.3.3"));
    }
}
