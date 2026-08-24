use axum::http::{HeaderMap, header};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreconditionResult {
    Proceed,
    NotModified,
    PreconditionFailed,
}

pub fn evaluate_preconditions(
    headers: &HeaderMap,
    method: &str,
    etag: &str,
    last_modified_rfc3339: &str,
) -> PreconditionResult {
    let normalized_etag = normalize_etag(etag);
    let resource_date = parse_datetime(last_modified_rfc3339);

    if let Some(if_match) = headers.get(header::IF_MATCH).and_then(|h| h.to_str().ok()) {
        if !match_etag_list(if_match, &normalized_etag, false) {
            return PreconditionResult::PreconditionFailed;
        }
    } else if let Some(if_unmod) = headers
        .get(header::IF_UNMODIFIED_SINCE)
        .and_then(|h| h.to_str().ok())
    {
        if let (Some(cond_date), Some(res_date)) = (parse_datetime(if_unmod), resource_date) {
            if res_date > cond_date {
                return PreconditionResult::PreconditionFailed;
            }
        }
    }

    if let Some(if_none_match) = headers
        .get(header::IF_NONE_MATCH)
        .and_then(|h| h.to_str().ok())
    {
        if match_etag_list(if_none_match, &normalized_etag, true) {
            return PreconditionResult::NotModified;
        }
    } else if (method == "GET" || method == "HEAD")
        && headers.contains_key(header::IF_MODIFIED_SINCE)
    {
        if let Some(if_mod) = headers
            .get(header::IF_MODIFIED_SINCE)
            .and_then(|h| h.to_str().ok())
        {
            if let (Some(cond_date), Some(res_date)) = (parse_datetime(if_mod), resource_date) {
                if res_date <= cond_date {
                    return PreconditionResult::NotModified;
                }
            }
        }
    }

    PreconditionResult::Proceed
}

pub fn evaluate_if_range(headers: &HeaderMap, etag: &str, last_modified_rfc3339: &str) -> bool {
    let Some(if_range) = headers.get(header::IF_RANGE).and_then(|h| h.to_str().ok()) else {
        return true;
    };
    let if_range = if_range.trim();
    if if_range.starts_with('"') || if_range.starts_with("W/\"") {
        let normalized = normalize_etag(etag);
        let target = normalize_etag(if_range);
        normalized == target
    } else if let (Some(cond_date), Some(res_date)) = (
        parse_datetime(if_range),
        parse_datetime(last_modified_rfc3339),
    ) {
        res_date <= cond_date
    } else {
        false
    }
}

pub fn normalize_etag(etag: &str) -> String {
    let trimmed = etag.trim();
    let unquoted = trimmed
        .strip_prefix("W/\"")
        .or_else(|| trimmed.strip_prefix("w/\""))
        .or_else(|| trimmed.strip_prefix('"'))
        .unwrap_or(trimmed);
    let clean = unquoted.strip_suffix('"').unwrap_or(unquoted);
    clean.to_string()
}

pub fn format_http_date(rfc3339_or_raw: &str) -> String {
    if let Some(dt) = parse_datetime(rfc3339_or_raw) {
        dt.format("%a, %d %b %Y %H:%M:%S GMT").to_string()
    } else {
        rfc3339_or_raw.to_string()
    }
}

pub fn parse_datetime(raw: &str) -> Option<DateTime<Utc>> {
    let trimmed = raw.trim();
    if let Ok(dt) = DateTime::parse_from_rfc3339(trimmed) {
        return Some(dt.with_timezone(&Utc));
    }
    if let Ok(dt) = DateTime::parse_from_rfc2822(trimmed) {
        return Some(dt.with_timezone(&Utc));
    }
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(trimmed, "%a, %d %b %Y %H:%M:%S GMT") {
        return Some(DateTime::from_naive_utc_and_offset(naive, Utc));
    }
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(trimmed, "%A, %d-%b-%y %H:%M:%S GMT") {
        return Some(DateTime::from_naive_utc_and_offset(naive, Utc));
    }
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(trimmed, "%a %b %e %H:%M:%S %Y") {
        return Some(DateTime::from_naive_utc_and_offset(naive, Utc));
    }
    None
}

fn match_etag_list(header_value: &str, target_etag: &str, allow_weak: bool) -> bool {
    let target = normalize_etag(target_etag);
    for item in header_value.split(',') {
        let item = item.trim();
        if item == "*" {
            return true;
        }
        let normalized = normalize_etag(item);
        if !allow_weak && (item.starts_with("W/") || item.starts_with("w/")) {
            continue;
        }
        if normalized == target {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn test_normalize_etag() {
        assert_eq!(normalize_etag("\"12345\""), "12345");
        assert_eq!(normalize_etag("W/\"12345\""), "12345");
        assert_eq!(normalize_etag("12345"), "12345");
    }

    #[test]
    fn test_if_none_match_matches() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::IF_NONE_MATCH,
            HeaderValue::from_static("\"abc\", \"def\""),
        );
        let result = evaluate_preconditions(&headers, "GET", "def", "2026-08-24T12:00:00Z");
        assert_eq!(result, PreconditionResult::NotModified);
    }

    #[test]
    fn test_if_none_match_wildcard() {
        let mut headers = HeaderMap::new();
        headers.insert(header::IF_NONE_MATCH, HeaderValue::from_static("*"));
        let result = evaluate_preconditions(&headers, "GET", "any-etag", "2026-08-24T12:00:00Z");
        assert_eq!(result, PreconditionResult::NotModified);
    }

    #[test]
    fn test_if_none_match_mismatch() {
        let mut headers = HeaderMap::new();
        headers.insert(header::IF_NONE_MATCH, HeaderValue::from_static("\"abc\""));
        let result = evaluate_preconditions(&headers, "GET", "xyz", "2026-08-24T12:00:00Z");
        assert_eq!(result, PreconditionResult::Proceed);
    }

    #[test]
    fn test_if_match_success_and_failure() {
        let mut headers = HeaderMap::new();
        headers.insert(header::IF_MATCH, HeaderValue::from_static("\"abc\""));
        let ok_res = evaluate_preconditions(&headers, "GET", "abc", "2026-08-24T12:00:00Z");
        assert_eq!(ok_res, PreconditionResult::Proceed);

        let fail_res = evaluate_preconditions(&headers, "GET", "xyz", "2026-08-24T12:00:00Z");
        assert_eq!(fail_res, PreconditionResult::PreconditionFailed);
    }

    #[test]
    fn test_if_modified_since() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::IF_MODIFIED_SINCE,
            HeaderValue::from_static("Mon, 24 Aug 2026 12:00:00 GMT"),
        );
        let not_mod = evaluate_preconditions(&headers, "GET", "etag1", "2026-08-24T11:00:00Z");
        assert_eq!(not_mod, PreconditionResult::NotModified);

        let mod_res = evaluate_preconditions(&headers, "GET", "etag1", "2026-08-24T13:00:00Z");
        assert_eq!(mod_res, PreconditionResult::Proceed);
    }

    #[test]
    fn test_if_unmodified_since_fails() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::IF_UNMODIFIED_SINCE,
            HeaderValue::from_static("Mon, 24 Aug 2026 10:00:00 GMT"),
        );
        let res = evaluate_preconditions(&headers, "GET", "etag1", "2026-08-24T12:00:00Z");
        assert_eq!(res, PreconditionResult::PreconditionFailed);
    }

    #[test]
    fn test_if_range_etag_and_date() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::IF_RANGE,
            HeaderValue::from_static("\"target-etag\""),
        );
        assert!(evaluate_if_range(
            &headers,
            "target-etag",
            "2026-08-24T12:00:00Z"
        ));
        assert!(!evaluate_if_range(
            &headers,
            "different-etag",
            "2026-08-24T12:00:00Z"
        ));

        headers.insert(
            header::IF_RANGE,
            HeaderValue::from_static("Mon, 24 Aug 2026 13:00:00 GMT"),
        );
        assert!(evaluate_if_range(
            &headers,
            "target-etag",
            "2026-08-24T12:00:00Z"
        ));
    }
}
