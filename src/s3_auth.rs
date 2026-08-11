use crate::{audit, http::AppState, security::s3_secret::s3_secret_encryption_key};
use axum::{
    body::Body,
    extract::{Request, State},
    http::{HeaderMap, StatusCode, header},
    middleware::Next,
    response::Response,
};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

type HmacSha256 = Hmac<Sha256>;
const SIGV4_MAX_CLOCK_SKEW_SECONDS: i64 = 900;

#[derive(Debug, Clone)]
pub struct S3Identity {
    pub access_key_id: String,
}

pub async fn require_s3_signature(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Response {
    let auth =
        match validate_sigv4(&state, request.method().as_str(), request.uri(), &headers).await {
            Ok(identity) => identity,
            Err(error) => {
                return s3_error(StatusCode::FORBIDDEN, error.code(), error.message());
            }
        };

    request.extensions_mut().insert(auth);
    next.run(request).await
}

async fn validate_sigv4(
    state: &AppState,
    method: &str,
    uri: &axum::http::Uri,
    headers: &HeaderMap,
) -> Result<S3Identity, S3AuthError> {
    if let Some(authorization) = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
    {
        return validate_header_sigv4(state, method, uri, headers, authorization).await;
    }

    let query = parse_query_params(uri.query().unwrap_or(""));
    if query
        .get("X-Amz-Algorithm")
        .is_some_and(|algorithm| algorithm == "AWS4-HMAC-SHA256")
    {
        return validate_presigned_sigv4(state, method, uri, headers, &query).await;
    }

    Err(S3AuthError::signature(
        "AWS Signature Version 4 authorization required",
    ))
}

async fn validate_header_sigv4(
    state: &AppState,
    method: &str,
    uri: &axum::http::Uri,
    headers: &HeaderMap,
    authorization: &str,
) -> Result<S3Identity, S3AuthError> {
    let authorization = authorization
        .strip_prefix("AWS4-HMAC-SHA256 ")
        .ok_or_else(|| S3AuthError::signature("unsupported authorization algorithm"))?;

    let auth_parts = parse_authorization(authorization).map_err(S3AuthError::signature)?;
    let credential = auth_parts
        .get("Credential")
        .ok_or_else(|| S3AuthError::signature("missing Credential"))?;
    let signed_headers = auth_parts
        .get("SignedHeaders")
        .ok_or_else(|| S3AuthError::signature("missing SignedHeaders"))?;
    let signature = auth_parts
        .get("Signature")
        .ok_or_else(|| S3AuthError::signature("missing Signature"))?;

    let credential_parts: Vec<&str> = credential.split('/').collect();
    if credential_parts.len() != 5 {
        return Err(S3AuthError::signature("invalid Credential scope"));
    }
    let access_key_id = credential_parts[0];
    let date = credential_parts[1];
    let region = credential_parts[2];
    let service = credential_parts[3];
    let terminal = credential_parts[4];
    if service != "s3" || terminal != "aws4_request" {
        return Err(S3AuthError::signature(
            "Credential scope must target s3/aws4_request",
        ));
    }

    let (identity, signing_secret) = load_signing_secret(state, access_key_id).await?;

    let amz_date = headers
        .get("x-amz-date")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| S3AuthError::signature("missing x-amz-date"))?;
    validate_amz_date(amz_date, date)?;
    let payload_hash = headers
        .get("x-amz-content-sha256")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("UNSIGNED-PAYLOAD");

    let canonical_request = canonical_request(method, uri, headers, signed_headers, payload_hash)
        .map_err(S3AuthError::signature)?;
    let canonical_hash = format!("{:x}", Sha256::digest(canonical_request.as_bytes()));
    let credential_scope = format!("{date}/{region}/s3/aws4_request");
    let string_to_sign =
        format!("AWS4-HMAC-SHA256\n{amz_date}\n{credential_scope}\n{canonical_hash}");
    let signing_key = signing_key(&signing_secret, date, region);
    let expected_signature = hex_hmac(&signing_key, string_to_sign.as_bytes());

    if !constant_time_eq(signature.as_bytes(), expected_signature.as_bytes()) {
        audit::failure(
            "s3_auth_failed",
            Some(access_key_id),
            "SigV4 signature mismatch",
        );
        return Err(S3AuthError::signature(
            "The request signature could not be verified.",
        ));
    }

    state
        .catalog
        .record_s3_access_key_used(access_key_id)
        .await
        .map_err(|error| S3AuthError::signature(error.to_string()))?;

    Ok(identity)
}

async fn validate_presigned_sigv4(
    state: &AppState,
    method: &str,
    uri: &axum::http::Uri,
    headers: &HeaderMap,
    query: &BTreeMap<String, String>,
) -> Result<S3Identity, S3AuthError> {
    let credential = query
        .get("X-Amz-Credential")
        .ok_or_else(|| S3AuthError::signature("missing X-Amz-Credential"))?;
    let amz_date = query
        .get("X-Amz-Date")
        .ok_or_else(|| S3AuthError::signature("missing X-Amz-Date"))?;
    let expires = query
        .get("X-Amz-Expires")
        .ok_or_else(|| S3AuthError::signature("missing X-Amz-Expires"))?;
    let signed_headers = query
        .get("X-Amz-SignedHeaders")
        .ok_or_else(|| S3AuthError::signature("missing X-Amz-SignedHeaders"))?;
    let signature = query
        .get("X-Amz-Signature")
        .ok_or_else(|| S3AuthError::signature("missing X-Amz-Signature"))?;

    let credential_parts: Vec<&str> = credential.split('/').collect();
    if credential_parts.len() != 5 {
        return Err(S3AuthError::signature("invalid X-Amz-Credential scope"));
    }
    let access_key_id = credential_parts[0];
    let date = credential_parts[1];
    let region = credential_parts[2];
    let service = credential_parts[3];
    let terminal = credential_parts[4];
    if service != "s3" || terminal != "aws4_request" {
        return Err(S3AuthError::signature(
            "X-Amz-Credential scope must target s3/aws4_request",
        ));
    }
    let expires_seconds = expires
        .parse::<i64>()
        .map_err(|_| S3AuthError::signature("invalid X-Amz-Expires"))?;
    validate_presigned_time(amz_date, date, expires_seconds)?;

    let (identity, signing_secret) = load_signing_secret(state, access_key_id).await?;
    let canonical_query = canonical_presigned_query(uri.query().unwrap_or(""));
    let canonical_request = canonical_request_with_query(
        method,
        uri,
        headers,
        signed_headers,
        "UNSIGNED-PAYLOAD",
        &canonical_query,
    )
    .map_err(S3AuthError::signature)?;
    let canonical_hash = format!("{:x}", Sha256::digest(canonical_request.as_bytes()));
    let credential_scope = format!("{date}/{region}/s3/aws4_request");
    let string_to_sign =
        format!("AWS4-HMAC-SHA256\n{amz_date}\n{credential_scope}\n{canonical_hash}");
    let signing_key = signing_key(&signing_secret, date, region);
    let expected_signature = hex_hmac(&signing_key, string_to_sign.as_bytes());

    if !constant_time_eq(signature.as_bytes(), expected_signature.as_bytes()) {
        audit::failure(
            "s3_auth_failed",
            Some(access_key_id),
            "SigV4 presigned URL signature mismatch",
        );
        return Err(S3AuthError::signature(
            "The request signature could not be verified.",
        ));
    }

    state
        .catalog
        .record_s3_access_key_used(access_key_id)
        .await
        .map_err(|error| S3AuthError::signature(error.to_string()))?;

    Ok(identity)
}

async fn load_signing_secret(
    state: &AppState,
    access_key_id: &str,
) -> Result<(S3Identity, String), S3AuthError> {
    let secret_encryption_key = s3_secret_encryption_key(&state.paths)
        .map_err(|error| S3AuthError::signature(error.to_string()))?;
    let Some(key) = state
        .catalog
        .find_s3_access_key_for_signing(access_key_id, &secret_encryption_key)
        .await
        .map_err(|error| S3AuthError::signature(error.to_string()))?
    else {
        audit::failure("s3_auth_failed", None, "unknown or revoked S3 access key");
        return Err(S3AuthError::invalid_access_key());
    };

    let signing_secret = match key.secret_access_key {
        Some(secret) => secret,
        None => {
            return Err(S3AuthError::signature(
                "S3 access key has no stored signing secret",
            ));
        }
    };
    let expected_secret_hash = format!("{:x}", Sha256::digest(signing_secret.as_bytes()));
    if expected_secret_hash != key.secret_key_hash {
        return Err(S3AuthError::signature(
            "configured S3 secret does not match catalog hash",
        ));
    }

    Ok((
        S3Identity {
            access_key_id: key.access_key_id,
        },
        signing_secret,
    ))
}

fn validate_amz_date(amz_date: &str, credential_date: &str) -> Result<(), S3AuthError> {
    if amz_date.len() != 16 || !amz_date.ends_with('Z') {
        return Err(S3AuthError::signature("invalid x-amz-date"));
    }
    let request_date = amz_date
        .get(0..8)
        .ok_or_else(|| S3AuthError::signature("invalid x-amz-date"))?;
    if request_date != credential_date {
        return Err(S3AuthError::signature(
            "x-amz-date must match Credential date",
        ));
    }
    let naive = chrono::NaiveDateTime::parse_from_str(amz_date, "%Y%m%dT%H%M%SZ")
        .map_err(|_| S3AuthError::signature("invalid x-amz-date"))?;
    let request_time =
        chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(naive, chrono::Utc);
    let skew = (chrono::Utc::now() - request_time).num_seconds().abs();
    if skew > SIGV4_MAX_CLOCK_SKEW_SECONDS {
        return Err(S3AuthError::signature(
            "x-amz-date is outside the allowed signature window",
        ));
    }
    Ok(())
}

fn validate_presigned_time(
    amz_date: &str,
    credential_date: &str,
    expires_seconds: i64,
) -> Result<(), S3AuthError> {
    if !(1..=604_800).contains(&expires_seconds) {
        return Err(S3AuthError::signature(
            "X-Amz-Expires must be between 1 and 604800 seconds",
        ));
    }
    if amz_date.len() != 16 || !amz_date.ends_with('Z') {
        return Err(S3AuthError::signature("invalid X-Amz-Date"));
    }
    let request_date = amz_date
        .get(0..8)
        .ok_or_else(|| S3AuthError::signature("invalid X-Amz-Date"))?;
    if request_date != credential_date {
        return Err(S3AuthError::signature(
            "X-Amz-Date must match X-Amz-Credential date",
        ));
    }
    let naive = chrono::NaiveDateTime::parse_from_str(amz_date, "%Y%m%dT%H%M%SZ")
        .map_err(|_| S3AuthError::signature("invalid X-Amz-Date"))?;
    let request_time =
        chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(naive, chrono::Utc);
    let now = chrono::Utc::now();
    if request_time > now + chrono::Duration::seconds(SIGV4_MAX_CLOCK_SKEW_SECONDS) {
        return Err(S3AuthError::signature(
            "X-Amz-Date is outside the allowed signature window",
        ));
    }
    if now > request_time + chrono::Duration::seconds(expires_seconds) {
        return Err(S3AuthError::signature("presigned URL has expired"));
    }
    Ok(())
}

struct S3AuthError {
    code: &'static str,
    message: String,
}

impl S3AuthError {
    fn invalid_access_key() -> Self {
        Self::signature("The request signature could not be verified.")
    }

    fn signature(message: impl Into<String>) -> Self {
        Self {
            code: "SignatureDoesNotMatch",
            message: message.into(),
        }
    }

    fn code(&self) -> &'static str {
        self.code
    }

    fn message(&self) -> &str {
        &self.message
    }
}

fn parse_authorization(raw: &str) -> Result<BTreeMap<String, String>, String> {
    let mut parts = BTreeMap::new();
    for part in raw.split(',') {
        let (key, value) = part
            .trim()
            .split_once('=')
            .ok_or_else(|| "invalid Authorization attribute".to_owned())?;
        parts.insert(key.to_owned(), value.to_owned());
    }
    Ok(parts)
}

fn canonical_request(
    method: &str,
    uri: &axum::http::Uri,
    headers: &HeaderMap,
    signed_headers: &str,
    payload_hash: &str,
) -> Result<String, String> {
    canonical_request_with_query(
        method,
        uri,
        headers,
        signed_headers,
        payload_hash,
        &canonical_query(uri.query().unwrap_or("")),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_access_keys_do_not_have_a_distinct_public_error() {
        let unknown = S3AuthError::invalid_access_key();
        let invalid = S3AuthError::signature("The request signature could not be verified.");

        assert_eq!(unknown.code(), invalid.code());
        assert_eq!(unknown.message(), invalid.message());
    }
}

fn canonical_request_with_query(
    method: &str,
    uri: &axum::http::Uri,
    headers: &HeaderMap,
    signed_headers: &str,
    payload_hash: &str,
    canonical_query: &str,
) -> Result<String, String> {
    let canonical_uri = if uri.path().is_empty() {
        "/"
    } else {
        uri.path()
    };
    let mut canonical_headers = String::new();

    for header_name in signed_headers.split(';') {
        let header_name = header_name.trim().to_ascii_lowercase();
        let value = headers
            .get(&header_name)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| format!("missing signed header: {header_name}"))?;
        canonical_headers.push_str(&header_name);
        canonical_headers.push(':');
        canonical_headers.push_str(&normalize_header_value(value));
        canonical_headers.push('\n');
    }

    Ok(format!(
        "{method}\n{canonical_uri}\n{canonical_query}\n{canonical_headers}\n{signed_headers}\n{payload_hash}"
    ))
}

fn canonical_query(raw: &str) -> String {
    if raw.is_empty() {
        return String::new();
    }
    let mut pairs: Vec<&str> = raw.split('&').collect();
    pairs.sort_unstable();
    pairs.join("&")
}

fn canonical_presigned_query(raw: &str) -> String {
    if raw.is_empty() {
        return String::new();
    }
    let mut pairs: Vec<&str> = raw
        .split('&')
        .filter(|pair| {
            let key = pair.split_once('=').map_or(*pair, |(key, _)| key);
            key != "X-Amz-Signature"
        })
        .collect();
    pairs.sort_unstable();
    pairs.join("&")
}

fn parse_query_params(raw: &str) -> BTreeMap<String, String> {
    let mut params = BTreeMap::new();
    for pair in raw.split('&').filter(|pair| !pair.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        params.insert(percent_decode(key), percent_decode(value));
    }
    params
}

fn percent_decode(raw: &str) -> String {
    let mut output = Vec::with_capacity(raw.len());
    let bytes = raw.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let Ok(value) = u8::from_str_radix(&raw[index + 1..index + 3], 16) {
                output.push(value);
                index += 3;
                continue;
            }
        }
        output.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&output).into_owned()
}

fn normalize_header_value(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn signing_key(secret: &str, date: &str, region: &str) -> Vec<u8> {
    let date_key = hmac_bytes(format!("AWS4{secret}").as_bytes(), date.as_bytes());
    let date_region_key = hmac_bytes(&date_key, region.as_bytes());
    let date_region_service_key = hmac_bytes(&date_region_key, b"s3");
    hmac_bytes(&date_region_service_key, b"aws4_request")
}

fn hmac_bytes(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts keys of any length");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

fn hex_hmac(key: &[u8], data: &[u8]) -> String {
    to_hex(&hmac_bytes(key, data))
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    use subtle::ConstantTimeEq;
    left.ct_eq(right).into()
}

fn s3_error(status: StatusCode, code: &str, message: &str) -> Response {
    let body = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
<Error><Code>{}</Code><Message>{}</Message><RequestId>{}</RequestId></Error>",
        xml_escape(code),
        xml_escape(message),
        uuid::Uuid::new_v4()
    );
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/xml")
        .header("x-amz-request-id", "pontemesh-auth")
        .body(Body::from(body))
        .expect("valid S3 auth error response")
}

fn xml_escape(raw: &str) -> String {
    raw.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
