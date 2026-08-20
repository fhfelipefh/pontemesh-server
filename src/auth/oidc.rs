use axum::{
    extract::{Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Redirect, Response},
};
use openidconnect::{
    AuthenticationFlow, AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope, TokenResponse,
    core::{CoreClient, CoreProviderMetadata, CoreResponseType},
};
use serde::{Deserialize, Serialize};

use crate::{
    auth::{client_ip, hash_session_token, session_cookie, user_agent},
    config,
    http::AppState,
    security::random::secure_url_token,
};
use base64::Engine;

const OIDC_STATE_COOKIE: &str = "pm_oidc_state";

#[derive(Debug, Serialize, Deserialize)]
struct OidcState {
    csrf_token: String,
    pkce_verifier: String,
    nonce: String,
}

pub async fn login_oidc(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let config = match config::load_instance_config(&state.paths) {
        Ok(c) => c,
        Err(e) => return crate::auth::internal_error(e),
    };

    if !config.oidc.enabled {
        return (StatusCode::BAD_REQUEST, "OIDC is not enabled").into_response();
    }

    let issuer_url_str = config.oidc.issuer_url.clone().unwrap_or_default();
    let client_id_str = config.oidc.client_id.clone().unwrap_or_default();
    let client_secret_str = config.oidc.client_secret.clone().unwrap_or_default();

    if issuer_url_str.is_empty() || client_id_str.is_empty() || client_secret_str.is_empty() {
        return (StatusCode::BAD_REQUEST, "OIDC is not fully configured").into_response();
    }

    let issuer_url = match IssuerUrl::new(issuer_url_str) {
        Ok(url) => url,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Invalid issuer URL").into_response(),
    };

    let provider_metadata =
        match CoreProviderMetadata::discover_async(issuer_url, &reqwest::Client::new()).await {
            Ok(metadata) => metadata,
            Err(e) => {
                tracing::error!("Failed to discover OIDC provider: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to discover OIDC provider",
                )
                    .into_response();
            }
        };

    let client_id = ClientId::new(client_id_str);
    let client_secret = ClientSecret::new(client_secret_str);

    // Determine our public URL for the redirect
    let base_url = config.public_endpoints.web_url.clone().unwrap_or_else(|| {
        let host = headers
            .get(header::HOST)
            .and_then(|h| h.to_str().ok())
            .unwrap_or("localhost");
        format!("http://{}", host)
    });

    let redirect_url_str = format!("{}/api/auth/oidc/callback", base_url.trim_end_matches('/'));

    let redirect_url = match RedirectUrl::new(redirect_url_str) {
        Ok(url) => url,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "Invalid redirect URL").into_response();
        }
    };

    let client =
        CoreClient::from_provider_metadata(provider_metadata, client_id, Some(client_secret))
            .set_redirect_uri(redirect_url);

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (auth_url, csrf_token, nonce) = client
        .authorize_url(
            AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .add_scope(Scope::new("openid".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .add_scope(Scope::new("email".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    let state_data = OidcState {
        csrf_token: String::from(csrf_token.secret()),
        pkce_verifier: String::from(pkce_verifier.secret()),
        nonce: String::from(nonce.secret()),
    };

    let state_json = match serde_json::to_string(&state_data) {
        Ok(json) => json,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to serialize state",
            )
                .into_response();
        }
    };

    let state_base64 = base64::engine::general_purpose::STANDARD.encode(state_json);

    let secure =
        crate::auth::request_is_https(&headers) || !crate::auth::request_is_localhost(&headers);
    let cookie = format!(
        "{OIDC_STATE_COOKIE}={state_base64}; Path=/; HttpOnly; SameSite=Lax; Max-Age=300{}",
        if secure { "; Secure" } else { "" }
    );

    let mut response: Response = Redirect::to(auth_url.as_str()).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&cookie).expect("valid cookie"),
    );

    response
}

#[derive(Debug, Deserialize)]
pub struct OidcCallbackQuery {
    code: String,
    state: String,
}

pub async fn callback_oidc(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OidcCallbackQuery>,
) -> Response {
    let config = match config::load_instance_config(&state.paths) {
        Ok(c) => c,
        Err(e) => return crate::auth::internal_error(e),
    };

    if !config.oidc.enabled {
        return (StatusCode::BAD_REQUEST, "OIDC is not enabled").into_response();
    }

    let cookie_val = headers
        .get(header::COOKIE)
        .and_then(|c| c.to_str().ok())
        .and_then(|c| {
            c.split(';')
                .find(|s| s.trim().starts_with(&format!("{}=", OIDC_STATE_COOKIE)))
        })
        .map(|s| {
            s.trim()
                .trim_start_matches(&format!("{}=", OIDC_STATE_COOKIE))
        });

    let state_cookie_base64 = match cookie_val {
        Some(v) => v,
        None => return (StatusCode::BAD_REQUEST, "Missing OIDC state cookie").into_response(),
    };

    let state_json = match base64::engine::general_purpose::STANDARD.decode(state_cookie_base64) {
        Ok(json) => String::from_utf8_lossy(&json).into_owned(),
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid OIDC state cookie").into_response(),
    };

    let state_data: OidcState = match serde_json::from_str(&state_json) {
        Ok(data) => data,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid OIDC state data").into_response(),
    };

    if query.state != state_data.csrf_token {
        return (StatusCode::BAD_REQUEST, "CSRF token mismatch").into_response();
    }

    let issuer_url_str = config.oidc.issuer_url.unwrap_or_default();
    let client_id_str = config.oidc.client_id.unwrap_or_default();
    let client_secret_str = config.oidc.client_secret.unwrap_or_default();

    if issuer_url_str.is_empty() || client_id_str.is_empty() || client_secret_str.is_empty() {
        return (StatusCode::BAD_REQUEST, "OIDC is not fully configured").into_response();
    }

    let issuer_url = match IssuerUrl::new(issuer_url_str) {
        Ok(url) => url,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Invalid issuer URL").into_response(),
    };

    let provider_metadata: CoreProviderMetadata =
        match CoreProviderMetadata::discover_async(issuer_url, &reqwest::Client::new()).await {
            Ok(metadata) => metadata,
            Err(e) => {
                tracing::error!("Failed to discover OIDC provider: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to discover OIDC provider",
                )
                    .into_response();
            }
        };

    let client_id = ClientId::new(client_id_str);
    let client_secret = ClientSecret::new(client_secret_str);

    let base_url = config.public_endpoints.web_url.unwrap_or_else(|| {
        let host = headers
            .get(header::HOST)
            .and_then(|h| h.to_str().ok())
            .unwrap_or("localhost");
        format!("http://{}", host)
    });

    let redirect_url_str = format!("{}/api/auth/oidc/callback", base_url.trim_end_matches('/'));
    let redirect_url = match RedirectUrl::new(redirect_url_str) {
        Ok(url) => url,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "Invalid redirect URL").into_response();
        }
    };

    let client =
        CoreClient::from_provider_metadata(provider_metadata, client_id, Some(client_secret))
            .set_redirect_uri(redirect_url);

    let pkce_verifier = PkceCodeVerifier::new(state_data.pkce_verifier);

    let exchange_req = match client.exchange_code(AuthorizationCode::new(query.code)) {
        Ok(req) => req,
        Err(e) => {
            tracing::error!("Failed to create token exchange request: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Invalid OIDC configuration for code exchange",
            )
                .into_response();
        }
    };

    let token_response: openidconnect::core::CoreTokenResponse = match exchange_req
        .set_pkce_verifier(pkce_verifier)
        .request_async(&reqwest::Client::new())
        .await
    {
        Ok(token) => token,
        Err(e) => {
            tracing::error!("Failed to exchange OIDC code: {}", e);
            return (
                StatusCode::UNAUTHORIZED,
                "Failed to exchange authorization code",
            )
                .into_response();
        }
    };

    let id_token: &openidconnect::core::CoreIdToken = match token_response.id_token() {
        Some(token) => token,
        None => return (StatusCode::UNAUTHORIZED, "Missing ID token").into_response(),
    };

    let claims: &openidconnect::core::CoreIdTokenClaims =
        match id_token.claims(&client.id_token_verifier(), &Nonce::new(state_data.nonce)) {
            Ok(claims) => claims,
            Err(e) => {
                tracing::error!("Failed to verify ID token claims: {}", e);
                return (StatusCode::UNAUTHORIZED, "Invalid ID token").into_response();
            }
        };

    let username = claims
        .preferred_username()
        .map(|u| u.as_str())
        .unwrap_or_else(|| claims.subject().as_str())
        .to_string();

    let user_id = match state.catalog.find_active_user_by_username(&username).await {
        Ok(Some(user)) => user.id,
        Ok(None) => {
            let password_hash = format!("*OIDC*{}", secure_url_token("", 16));
            match state
                .catalog
                .create_user(&username, &password_hash, "viewer")
                .await
            {
                Ok(id) => id,
                Err(e) => return crate::auth::internal_error(e),
            }
        }
        Err(e) => return crate::auth::internal_error(e),
    };

    let token = secure_url_token("", 32);
    let token_hash = hash_session_token(&token);

    if let Err(error) = state
        .catalog
        .create_user_session(
            &user_id,
            &token_hash,
            user_agent(&headers),
            client_ip(&headers),
        )
        .await
    {
        return crate::auth::internal_error(error);
    }

    crate::auth::record_auth_audit(
        &state,
        "login_success_oidc",
        Some(&username),
        "success",
        "session created via OIDC",
    )
    .await;

    // Clear state cookie
    let secure =
        crate::auth::request_is_https(&headers) || !crate::auth::request_is_localhost(&headers);
    let clear_state_cookie = format!(
        "{OIDC_STATE_COOKIE}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0{}",
        if secure { "; Secure" } else { "" }
    );

    let mut response = Redirect::to("/dashboard").into_response();
    response
        .headers_mut()
        .append(header::SET_COOKIE, session_cookie(&headers, &token));
    response.headers_mut().append(
        header::SET_COOKIE,
        HeaderValue::from_str(&clear_state_cookie).unwrap(),
    );

    response
}
