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
use std::{collections::BTreeMap, env};

type HmacSha256 = Hmac<Sha256>;

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
            Err(error) => return s3_error(StatusCode::FORBIDDEN, "SignatureDoesNotMatch", &error),
        };

    request.extensions_mut().insert(auth);
    next.run(request).await
}

async fn validate_sigv4(
    state: &AppState,
    method: &str,
    uri: &axum::http::Uri,
    headers: &HeaderMap,
) -> Result<S3Identity, String> {
    let authorization = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| "AWS Signature Version 4 authorization required".to_owned())?;
    let authorization = authorization
        .strip_prefix("AWS4-HMAC-SHA256 ")
        .ok_or_else(|| "unsupported authorization algorithm".to_owned())?;

    let auth_parts = parse_authorization(authorization)?;
    let credential = auth_parts
        .get("Credential")
        .ok_or_else(|| "missing Credential".to_owned())?;
    let signed_headers = auth_parts
        .get("SignedHeaders")
        .ok_or_else(|| "missing SignedHeaders".to_owned())?;
    let signature = auth_parts
        .get("Signature")
        .ok_or_else(|| "missing Signature".to_owned())?;

    let credential_parts: Vec<&str> = credential.split('/').collect();
    if credential_parts.len() != 5 {
        return Err("invalid Credential scope".to_owned());
    }
    let access_key_id = credential_parts[0];
    let date = credential_parts[1];
    let region = credential_parts[2];
    let service = credential_parts[3];
    let terminal = credential_parts[4];
    if service != "s3" || terminal != "aws4_request" {
        return Err("Credential scope must target s3/aws4_request".to_owned());
    }

    let secret_encryption_key =
        s3_secret_encryption_key(&state.paths).map_err(|error| error.to_string())?;
    let Some(key) = state
        .catalog
        .find_s3_access_key_for_signing(access_key_id, &secret_encryption_key)
        .await
        .map_err(|error| error.to_string())?
    else {
        audit::failure("s3_auth_failed", None, "unknown or revoked S3 access key");
        return Err("invalid S3 access key".to_owned());
    };

    let signing_secret = match key.secret_access_key {
        Some(secret) => secret,
        None => bootstrap_signing_secret(access_key_id)?,
    };
    let expected_secret_hash = format!("{:x}", Sha256::digest(signing_secret.as_bytes()));
    if expected_secret_hash != key.secret_key_hash {
        return Err("configured S3 secret does not match catalog hash".to_owned());
    }

    let amz_date = headers
        .get("x-amz-date")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| "missing x-amz-date".to_owned())?;
    let payload_hash = headers
        .get("x-amz-content-sha256")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("UNSIGNED-PAYLOAD");

    let canonical_request = canonical_request(method, uri, headers, signed_headers, payload_hash)?;
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
        return Err("request signature does not match".to_owned());
    }

    state
        .catalog
        .record_s3_access_key_used(access_key_id)
        .await
        .map_err(|error| error.to_string())?;

    Ok(S3Identity {
        access_key_id: key.access_key_id,
    })
}

fn bootstrap_signing_secret(access_key_id: &str) -> Result<String, String> {
    let bootstrap_id = env::var("PONTEMESH_S3_BOOTSTRAP_ACCESS_KEY_ID")
        .map_err(|_| "S3 access key has no stored signing secret".to_owned())?;
    let bootstrap_secret = env::var("PONTEMESH_S3_BOOTSTRAP_SECRET_ACCESS_KEY")
        .map_err(|_| "S3 access key has no stored signing secret".to_owned())?;
    if bootstrap_id != access_key_id {
        return Err("S3 access key has no stored signing secret".to_owned());
    }
    Ok(bootstrap_secret)
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
    let canonical_uri = if uri.path().is_empty() {
        "/"
    } else {
        uri.path()
    };
    let canonical_query = canonical_query(uri.query().unwrap_or(""));
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
