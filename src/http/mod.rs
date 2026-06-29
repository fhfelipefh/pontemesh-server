use crate::{admin, auth, catalog::Catalog, config::PontemeshHome, setup, web_assets};
use axum::{
    Router, middleware,
    routing::{any, get, post},
};
use std::time::Instant;
use tower_http::trace::TraceLayer;

#[derive(Debug, Clone)]
pub struct AppState {
    pub paths: PontemeshHome,
    pub setup: setup::SetupState,
    pub auth: auth::AuthState,
    pub catalog: Catalog,
    pub started_at: Instant,
}

pub fn router(paths: PontemeshHome, setup: setup::SetupState, catalog: Catalog) -> Router {
    let state = AppState {
        paths,
        setup,
        auth: auth::AuthState::new(),
        catalog,
        started_at: Instant::now(),
    };

    let admin_routes = Router::new()
        .route(
            "/api/admin/dashboard/summary",
            get(admin::dashboard_summary),
        )
        .route("/api/admin/instance", get(admin::instance_summary))
        .route("/api/admin/system/resources", get(admin::system_resources))
        .route("/api/admin/storage/status", get(admin::storage_status))
        .route(
            "/api/admin/buckets",
            get(admin::list_buckets).post(admin::create_bucket),
        )
        .route(
            "/api/admin/buckets/{bucket_name}",
            get(admin::get_bucket).delete(admin::delete_bucket),
        )
        .route(
            "/api/admin/buckets/{bucket_name}/objects",
            get(admin::list_objects).post(admin::upload_object),
        )
        .route(
            "/api/admin/buckets/{bucket_name}/objects/{*object_key}",
            get(admin::get_object).delete(admin::delete_object),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_admin_session,
        ));

    Router::new()
        .route("/api/setup/status", get(setup::routes::status))
        .route("/api/setup/unlock", post(setup::routes::unlock))
        .route("/api/setup/complete", post(setup::routes::complete))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/logout", post(auth::logout))
        .route("/api/auth/me", get(auth::me))
        .merge(admin_routes)
        .route("/api/{*path}", any(setup::routes::api_not_found))
        .fallback(web_assets::serve)
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{
            AdminSection, HttpSection, InstanceConfig, InstanceRole, InstanceSection,
            LocalStorageSection, StorageSection,
        },
        security::password::hash_admin_password,
    };
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode, header},
    };
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn admin_routes_require_session_cookie() {
        let protected_routes = [
            "/api/admin/dashboard/summary",
            "/api/admin/system/resources",
            "/api/admin/storage/status",
            "/api/admin/buckets",
        ];

        for route in protected_routes {
            let app = test_router("admin-routes-require-session").await;
            let response = app
                .oneshot(
                    Request::builder()
                        .uri(route)
                        .body(Body::empty())
                        .expect("valid request"),
                )
                .await
                .expect("router response");

            assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{route}");
            let body = response_text(response).await;
            assert!(body.contains("authentication required"), "{route}");
        }
    }

    #[tokio::test]
    async fn login_rejects_invalid_password_without_setting_session_cookie() {
        let app = test_router("login-rejects-invalid-password").await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/login")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"username":"admin","password":"wrong-password"}"#,
                    ))
                    .expect("valid request"),
            )
            .await
            .expect("router response");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert!(response.headers().get(header::SET_COOKIE).is_none());
    }

    #[tokio::test]
    async fn valid_login_cookie_allows_admin_summary() {
        let app = test_router("valid-login-cookie-allows-admin-summary").await;

        let login_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/login")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"username":"admin","password":"correct-password"}"#,
                    ))
                    .expect("valid request"),
            )
            .await
            .expect("router response");

        assert_eq!(login_response.status(), StatusCode::OK);
        let set_cookie = login_response
            .headers()
            .get(header::SET_COOKIE)
            .expect("login must set session cookie")
            .to_str()
            .expect("cookie must be valid header text")
            .to_owned();
        assert!(set_cookie.contains("pm_admin_session="));
        assert!(set_cookie.contains("HttpOnly"));
        assert!(set_cookie.contains("SameSite=Lax"));

        let session_cookie = set_cookie
            .split(';')
            .next()
            .expect("cookie has name/value")
            .to_owned();

        let summary_response = app
            .oneshot(
                Request::builder()
                    .uri("/api/admin/dashboard/summary")
                    .header(header::COOKIE, session_cookie)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");

        assert_eq!(summary_response.status(), StatusCode::OK);
        let body = response_text(summary_response).await;
        assert!(body.contains(r#""authenticated":true"#));
        assert!(body.contains(r#""lastCheckedAt":"#));
    }

    async fn test_router(name: &str) -> Router {
        let paths = test_home(name);
        paths.ensure_layout().expect("test home layout");
        write_test_config(&paths);
        fs::write(paths.setup_lock_file(), "completed_at = \"test\"\n").expect("setup lock");
        let setup = setup::SetupState::new();
        let catalog = Catalog::initialize(&paths).await.expect("catalog");
        router(paths, setup, catalog)
    }

    fn test_home(name: &str) -> PontemeshHome {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("pontemesh-{name}-{nanos}"));
        PontemeshHome::from_path(root).expect("test home")
    }

    fn write_test_config(paths: &PontemeshHome) {
        let config = InstanceConfig {
            instance: InstanceSection {
                name: "Test Origin".to_owned(),
                role: InstanceRole::Origin,
            },
            http: HttpSection {
                bind: "127.0.0.1".to_owned(),
                port: 8080,
            },
            storage: StorageSection {
                local: LocalStorageSection {
                    path: paths.storage_dir(),
                },
            },
            admin: AdminSection {
                username: "admin".to_owned(),
                password_hash: hash_admin_password("correct-password").expect("password hash"),
                created_at: chrono::Utc::now(),
            },
        };
        let raw_config = toml::to_string(&config).expect("serialize config");
        fs::write(paths.config_file(), raw_config).expect("write config");
    }

    async fn response_text(response: axum::response::Response) -> String {
        let bytes = to_bytes(response.into_body(), 1024 * 1024)
            .await
            .expect("read response body");
        String::from_utf8(bytes.to_vec()).expect("utf8 response")
    }
}
