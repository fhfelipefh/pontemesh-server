use crate::{
    admin, auth, catalog::Catalog, config::PontemeshHome, mcp, mesh, origin, replica, s3_auth,
    setup, web_assets,
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Request, State},
    handler::Handler,
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{any, delete, get, post, put},
};
use std::time::Instant;
use tower_http::trace::TraceLayer;

#[derive(Debug, Clone)]
pub struct AppState {
    pub paths: PontemeshHome,
    pub setup: setup::SetupState,
    pub catalog: Catalog,
    pub started_at: Instant,
}

fn app_state(paths: PontemeshHome, setup: setup::SetupState, catalog: Catalog) -> AppState {
    AppState {
        paths,
        setup,
        catalog,
        started_at: Instant::now(),
    }
}

pub fn web_router(paths: PontemeshHome, setup: setup::SetupState, catalog: Catalog) -> Router {
    let state = app_state(paths, setup, catalog);

    Router::new()
        .route("/api/setup/status", get(setup::routes::status))
        .route("/api/setup/unlock", post(setup::routes::unlock))
        .route("/api/setup/complete", post(setup::routes::complete))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/logout", post(auth::logout))
        .route("/api/auth/me", get(auth::me))
        .route(
            "/mcp",
            post(mcp::transport_http::post_mcp)
                .get(mcp::transport_http::method_not_allowed)
                .delete(mcp::transport_http::method_not_allowed),
        )
        .nest("/pontemesh", pontemesh_routes(state.clone()))
        .merge(admin_routes(state.clone()))
        .route("/api/{*path}", any(setup::routes::api_not_found))
        .fallback(web_assets::serve)
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}

pub fn s3_router(paths: PontemeshHome, setup: setup::SetupState, catalog: Catalog) -> Router {
    let state = app_state(paths, setup, catalog);

    Router::new()
        .route("/", get(origin::list_buckets))
        .route(
            "/{bucket_name}",
            put(origin::put_bucket)
                .post(origin::post_bucket)
                .get(origin::list_objects)
                .head(origin::head_bucket)
                .delete(origin::delete_bucket),
        )
        .route(
            "/{bucket_name}/{*object_key}",
            put(origin::put_object)
                .post(origin::post_object)
                .head(origin::head_object)
                .get(origin::get_object)
                .delete(origin::delete_object),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            s3_auth::require_s3_signature,
        ))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_origin_instance,
        ))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}

fn admin_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/api/admin/dashboard/summary",
            get(admin::dashboard_summary),
        )
        .route("/api/admin/instance", get(admin::instance_summary))
        .route("/api/admin/system/resources", get(admin::system_resources))
        .route("/api/admin/storage/status", get(admin::storage_status))
        .route("/api/admin/audit-events", get(admin::list_audit_events))
        .route("/api/admin/logs/application", get(admin::application_logs))
        .route(
            "/api/admin/configuration",
            get(admin::export_configuration).post(admin::import_configuration),
        )
        .route(
            "/api/admin/mcp/settings",
            get(admin::get_mcp_settings).put(admin::update_mcp_settings),
        )
        .route("/api/admin/mcp/status", get(admin::mcp_status))
        .route(
            "/api/admin/mcp/tokens",
            get(admin::list_mcp_tokens).post(admin::create_mcp_token),
        )
        .route(
            "/api/admin/mcp/tokens/{id}",
            delete(admin::revoke_mcp_token),
        )
        .route("/api/admin/mcp/activity", get(admin::mcp_activity))
        .route(
            "/api/admin/metrics/origin-traffic",
            get(admin::origin_traffic_metrics),
        )
        .route(
            "/api/admin/metrics/replica-traffic",
            get(admin::replica_traffic_metrics),
        )
        .route(
            "/api/admin/metrics/buckets",
            get(admin::bucket_traffic_metrics),
        )
        .route(
            "/api/admin/metrics/objects",
            get(admin::object_traffic_metrics),
        )
        .route(
            "/api/admin/metrics/replicas/{replica_id}",
            get(admin::replica_detail_metrics),
        )
        .merge(origin_admin_routes(state.clone()))
        .route_layer(middleware::from_fn_with_state(
            state,
            auth::require_admin_session,
        ))
}

fn origin_admin_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/api/admin/buckets",
            get(admin::list_buckets).post(admin::create_bucket),
        )
        .route(
            "/api/admin/buckets/{bucket_name}",
            get(admin::get_bucket).delete(admin::delete_bucket),
        )
        .route(
            "/api/admin/buckets/{bucket_name}/policy",
            get(admin::get_bucket_policy).put(admin::update_bucket_policy),
        )
        .route(
            "/api/admin/buckets/{bucket_name}/objects",
            get(admin::list_objects).post(admin::upload_object.layer(DefaultBodyLimit::disable())),
        )
        .route(
            "/api/admin/buckets/{bucket_name}/objects/{*object_key}",
            get(admin::get_object).delete(admin::delete_object),
        )
        .route(
            "/api/admin/buckets/{bucket_name}/object-revocations/{*object_key}",
            post(admin::revoke_object),
        )
        .route(
            "/api/admin/application-credentials",
            get(admin::list_application_credentials).post(admin::create_application_credential),
        )
        .route(
            "/api/admin/application-credentials/{id}/revoke",
            post(admin::revoke_application_credential),
        )
        .route(
            "/api/admin/access-packages/{package_id}/revoke",
            post(admin::revoke_access_package),
        )
        .route(
            "/api/admin/s3-access-keys",
            get(admin::list_s3_access_keys).post(admin::create_s3_access_key),
        )
        .route(
            "/api/admin/s3/access-keys",
            get(admin::list_s3_access_keys).post(admin::create_s3_access_key),
        )
        .route(
            "/api/admin/s3/access-keys/{id}",
            delete(admin::revoke_s3_access_key_by_id),
        )
        .route(
            "/api/admin/s3-access-keys/{access_key_id}/revoke",
            post(admin::revoke_s3_access_key),
        )
        .route(
            "/api/admin/replicas",
            get(admin::list_replicas).post(admin::create_replica_credential),
        )
        .route(
            "/api/admin/replicas/{replica_id}/revoke",
            post(admin::revoke_replica),
        )
        .route_layer(middleware::from_fn_with_state(
            state,
            require_origin_instance,
        ))
}

fn pontemesh_routes(state: AppState) -> Router<AppState> {
    let package_routes = Router::new()
        .route(
            "/access-packages/{package_id}/objects/{bucket_name}/{*object_key}",
            get(mesh::get_object_with_access_package),
        )
        .route(
            "/access-packages/{package_id}/revalidate/{bucket_name}/{*object_key}",
            post(mesh::revalidate_access_package),
        )
        .route(
            "/access-packages/{package_id}/peers/{bucket_name}/{*object_key}",
            post(mesh::announce_peer_availability),
        )
        .route(
            "/access-packages/{package_id}/events/{bucket_name}/{*object_key}",
            post(mesh::record_sdk_fragment_event),
        );

    let application_routes = Router::new()
        .route("/access-packages", post(mesh::create_access_package))
        .route(
            "/objects/{bucket_name}/manifest/{*object_key}",
            get(mesh::get_manifest),
        )
        .route(
            "/objects/{bucket_name}/sources/{*object_key}",
            get(mesh::get_sources),
        )
        .route(
            "/objects/{bucket_name}/availability/{*object_key}",
            get(mesh::get_availability),
        )
        .route(
            "/objects/{bucket_name}/policies/{*object_key}",
            get(mesh::get_object_policy),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_application_credential,
        ));

    let replica_routes = Router::new()
        .route("/replicas/{replica_id}/sync-plan", get(replica::sync_plan))
        .route(
            "/replicas/{replica_id}/objects/{bucket_name}/{*object_key}",
            get(replica::sync_object),
        )
        .route(
            "/replicas/{replica_id}/manifests/{manifest_id}/fragments/{fragment_id}",
            get(replica::sync_fragment),
        )
        .route(
            "/replicas/{replica_id}/availability",
            post(replica::announce_availability),
        )
        .route(
            "/replicas/{replica_id}/health",
            post(replica::report_health),
        )
        .route(
            "/replicas/{replica_id}/metrics",
            post(replica::report_metrics),
        )
        .route(
            "/replicas/{replica_id}/policy-updates",
            get(replica::policy_updates),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_replica_credential,
        ));

    Router::new()
        .merge(package_routes)
        .merge(application_routes)
        .merge(replica_routes)
        .route_layer(middleware::from_fn_with_state(
            state,
            require_origin_instance,
        ))
}

async fn require_origin_instance(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    match crate::config::require_instance_role(&state.paths, crate::config::InstanceRole::Origin) {
        Ok(()) => next.run(request).await,
        Err(error) => (
            axum::http::StatusCode::CONFLICT,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{
            HttpSection, InstanceConfig, InstanceRole, InstanceSection, LocalStorageSection,
            StorageSection,
        },
        security::{password::hash_admin_password, s3_secret::s3_secret_encryption_key},
    };
    use axum::{
        body::{Body, to_bytes},
        http::{Method, Request, StatusCode, header},
    };
    use hmac::{Hmac, Mac};
    use sha2::{Digest, Sha256};
    use sqlx::PgPool;
    use sqlx_core::row::Row;
    use std::{
        fs,
        sync::OnceLock,
        time::{SystemTime, UNIX_EPOCH},
    };
    use tokio::sync::Mutex;
    use tower::ServiceExt;

    static TEST_DB_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    const TEST_S3_ACCESS_KEY: &str = "PMTESTACCESSKEY";
    const TEST_S3_SECRET_KEY: &str = "pm-test-secret-key-material";
    const TEST_REGION: &str = "us-east-1";
    type HmacSha256 = Hmac<Sha256>;

    #[tokio::test]
    async fn postgres_auth_session_lifecycle() {
        let Some(ctx) = TestContext::new("postgres-auth-session").await else {
            return;
        };
        let _guard = ctx.guard;
        let app = ctx.app;

        let bad_login = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/auth/login")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"username":"admin","password":"wrong-password"}"#,
                    ))
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(bad_login.status(), StatusCode::UNAUTHORIZED);
        assert!(bad_login.headers().get(header::SET_COOKIE).is_none());

        let cookie = login_cookie(app.clone()).await;
        let session_count: i64 =
            sqlx_core::query_scalar::query_scalar("SELECT COUNT(*)::bigint FROM sessions")
                .fetch_one(ctx.catalog.pool())
                .await
                .expect("count sessions");
        assert_eq!(session_count, 1);

        let me = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/auth/me")
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(me.status(), StatusCode::OK);

        let logout = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/auth/logout")
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(logout.status(), StatusCode::OK);

        let revoked_count: i64 = sqlx_core::query_scalar::query_scalar(
            "SELECT COUNT(*)::bigint FROM sessions WHERE revoked_at IS NOT NULL",
        )
        .fetch_one(ctx.catalog.pool())
        .await
        .expect("count revoked sessions");
        assert_eq!(revoked_count, 1);

        let me_after_logout = app
            .oneshot(
                Request::builder()
                    .uri("/api/auth/me")
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(me_after_logout.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn api_contract_admin_routes_require_auth_and_return_stable_json_shapes() {
        let Some(ctx) = TestContext::new("api-contract-admin-routes").await else {
            return;
        };
        let _guard = ctx.guard;
        let app = ctx.app.clone();

        let unauthenticated = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/admin/dashboard/summary")
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);
        assert_json_content_type(&unauthenticated);
        let unauthenticated_body = json_body(unauthenticated).await;
        assert_eq!(unauthenticated_body["error"], "authentication required");

        let cookie = login_cookie(app.clone()).await;

        let dashboard = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/admin/dashboard/summary")
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(dashboard.status(), StatusCode::OK);
        assert_json_content_type(&dashboard);
        let dashboard_body = json_body(dashboard).await;
        assert_json_object_keys(
            &dashboard_body,
            &[
                "health",
                "instance",
                "mcp",
                "objects",
                "resources",
                "storage",
            ],
        );
        assert_json_object_keys(
            &dashboard_body["instance"],
            &["environment", "name", "role", "uptimeSeconds", "version"],
        );
        assert_eq!(dashboard_body["instance"]["role"], "origin");
        assert_json_object_keys(
            &dashboard_body["objects"],
            &["totalBuckets", "totalObjectBytes", "totalObjects"],
        );
        assert_json_object_keys(
            &dashboard_body["health"],
            &[
                "authenticated",
                "databaseConnected",
                "lastCheckedAt",
                "setupCompleted",
                "storageWritable",
            ],
        );
        assert_eq!(dashboard_body["health"]["authenticated"], true);
        assert_json_object_keys(
            &dashboard_body["mcp"],
            &[
                "activeSessionsCount",
                "authRequired",
                "enabled",
                "endpoint",
                "lastActivityAt",
                "promptsEnabled",
                "readToolsEnabled",
                "recentCallsCount",
                "resourcesEnabled",
                "writeToolsEnabled",
            ],
        );
        assert_eq!(dashboard_body["mcp"]["enabled"], false);

        let buckets_before = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/admin/buckets?page=1&pageSize=20")
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(buckets_before.status(), StatusCode::OK);
        let buckets_before_body = json_body(buckets_before).await;
        assert_paginated_contract(&buckets_before_body);
        assert_eq!(buckets_before_body["items"], serde_json::json!([]));

        let create_bucket = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/admin/buckets")
                    .header(header::COOKIE, &cookie)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"name":"contract-bucket"}"#))
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(create_bucket.status(), StatusCode::CREATED);
        assert_json_content_type(&create_bucket);
        let created_bucket_body = json_body(create_bucket).await;
        assert_json_object_keys(
            &created_bucket_body,
            &["createdAt", "name", "objectCount", "totalBytes"],
        );
        assert_eq!(created_bucket_body["name"], "contract-bucket");
        assert_eq!(created_bucket_body["objectCount"], 0);
        assert_eq!(created_bucket_body["totalBytes"], 0);

        let objects = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/admin/buckets/contract-bucket/objects?page=1&pageSize=20")
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(objects.status(), StatusCode::OK);
        let objects_body = json_body(objects).await;
        assert_paginated_contract(&objects_body);
        assert_eq!(objects_body["items"], serde_json::json!([]));

        let origin_metrics = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/admin/metrics/origin-traffic")
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(origin_metrics.status(), StatusCode::OK);
        let origin_metrics_body = json_body(origin_metrics).await;
        assert_json_object_keys(
            &origin_metrics_body,
            &[
                "fullObjectRequests",
                "rangeRequests",
                "totalBytesServed",
                "totalRequests",
            ],
        );

        let replica_metrics = app
            .oneshot(
                Request::builder()
                    .uri("/api/admin/metrics/replica-traffic")
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(replica_metrics.status(), StatusCode::OK);
        let replica_metrics_body = json_body(replica_metrics).await;
        assert_json_object_keys(
            &replica_metrics_body,
            &[
                "activeReplicas",
                "authFailures",
                "syncFailures",
                "totalBytesServed",
                "totalBytesSynced",
                "totalFragmentsServed",
                "totalFragmentsSynced",
                "totalReplicas",
            ],
        );
    }

    #[tokio::test]
    async fn mcp_endpoint_is_secure_by_default_and_serves_json_rpc_with_token() {
        let Some(ctx) = TestContext::new("mcp-json-rpc-contract").await else {
            return;
        };
        let _guard = ctx.guard;
        let app = ctx.app.clone();

        let disabled = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/mcp")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#,
                    ))
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(disabled.status(), StatusCode::NOT_FOUND);

        let cookie = login_cookie(app.clone()).await;
        let enable = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::PUT)
                    .uri("/api/admin/mcp/settings")
                    .header(header::COOKIE, &cookie)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"enabled":true,"endpointPath":"/mcp","bindHost":null,"requireAuth":true,"readToolsEnabled":true,"writeToolsEnabled":false,"adminToolsEnabled":false,"exposeResources":true,"exposePrompts":true,"allowLocalhostOnly":true}"#,
                    ))
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(enable.status(), StatusCode::OK);

        let no_token = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/mcp")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#,
                    ))
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(no_token.status(), StatusCode::UNAUTHORIZED);

        let invalid_token = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/mcp")
                    .header(header::AUTHORIZATION, "Bearer invalid")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#,
                    ))
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(invalid_token.status(), StatusCode::UNAUTHORIZED);

        let create_token = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/admin/mcp/tokens")
                    .header(header::COOKIE, &cookie)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"name":"contract-client"}"#))
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(create_token.status(), StatusCode::CREATED);
        let created_token = json_body(create_token).await;
        let secret = created_token["secret"].as_str().expect("MCP secret");
        assert!(secret.starts_with("pmcp_"));
        assert_eq!(created_token["token"]["tokenPrefix"], &secret[..12]);

        let initialize = mcp_call(
            app.clone(),
            secret,
            "initialize",
            serde_json::json!({
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": { "name": "contract-test", "version": "0.1.0" }
            }),
        )
        .await;
        assert_eq!(initialize["result"]["protocolVersion"], "2025-11-25");
        assert_eq!(
            initialize["result"]["serverInfo"]["name"],
            "pontemesh-server"
        );

        let tools = mcp_call(app.clone(), secret, "tools/list", serde_json::json!({})).await;
        let tool_names: Vec<&str> = tools["result"]["tools"]
            .as_array()
            .expect("tools array")
            .iter()
            .filter_map(|tool| tool["name"].as_str())
            .collect();
        assert!(tool_names.contains(&"pontemesh_get_health"));
        assert!(!tool_names.contains(&"pontemesh_create_bucket"));

        let health = mcp_call(
            app.clone(),
            secret,
            "tools/call",
            serde_json::json!({
                "name": "pontemesh_get_health",
                "arguments": {}
            }),
        )
        .await;
        assert_eq!(
            health["result"]["structuredContent"]["databaseConnected"],
            true
        );

        let blocked_write = mcp_call(
            app.clone(),
            secret,
            "tools/call",
            serde_json::json!({
                "name": "pontemesh_create_bucket",
                "arguments": { "bucket": "blocked" }
            }),
        )
        .await;
        assert_eq!(blocked_write["error"]["code"], -32603);

        let resources =
            mcp_call(app.clone(), secret, "resources/list", serde_json::json!({})).await;
        assert!(
            resources["result"]["resources"]
                .as_array()
                .expect("resources")
                .iter()
                .any(|resource| resource["uri"] == "pontemesh://instance/health")
        );

        let storage_resource = mcp_call(
            app.clone(),
            secret,
            "resources/read",
            serde_json::json!({ "uri": "pontemesh://storage/summary" }),
        )
        .await;
        let storage_text = storage_resource["result"]["contents"][0]["text"]
            .as_str()
            .expect("resource text");
        assert!(!storage_text.to_ascii_lowercase().contains("secret"));

        let prompts = mcp_call(app.clone(), secret, "prompts/list", serde_json::json!({})).await;
        assert!(
            prompts["result"]["prompts"]
                .as_array()
                .expect("prompts")
                .iter()
                .any(|prompt| prompt["name"] == "diagnose_instance")
        );

        let activity = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/admin/mcp/activity")
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(activity.status(), StatusCode::OK);
        let activity_body = json_body(activity).await;
        assert!(activity_body.as_array().expect("activity").len() >= 4);

        let revoke = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::DELETE)
                    .uri(format!(
                        "/api/admin/mcp/tokens/{}",
                        created_token["token"]["id"].as_str().expect("token id")
                    ))
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(revoke.status(), StatusCode::NO_CONTENT);

        let revoked = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/mcp")
                    .header(header::AUTHORIZATION, format!("Bearer {secret}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"jsonrpc":"2.0","id":99,"method":"ping"}"#))
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(revoked.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn s3_compatible_contract_covers_core_bucket_and_object_operations() {
        let Some(ctx) = TestContext::new("s3-compatible-contract").await else {
            return;
        };
        let _guard = ctx.guard;
        let s3_app = ctx.s3_app.clone();

        let unsigned = s3_app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/")
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(unsigned.status(), StatusCode::FORBIDDEN);
        assert!(
            response_text(unsigned)
                .await
                .contains("<Code>SignatureDoesNotMatch</Code>")
        );

        let stale_signature = s3_app
            .clone()
            .oneshot(
                signed_s3_request_with_date(
                    Request::builder().uri("/").body(Body::empty()),
                    b"",
                    TEST_S3_ACCESS_KEY,
                    TEST_S3_SECRET_KEY,
                    "20200101T000000Z",
                    "20200101",
                )
                .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(stale_signature.status(), StatusCode::FORBIDDEN);
        assert!(
            response_text(stale_signature)
                .await
                .contains("outside the allowed signature window")
        );

        let create_bucket = s3_app
            .clone()
            .oneshot(
                signed_s3_request(
                    Request::builder()
                        .method(Method::PUT)
                        .uri("/compat-bucket")
                        .body(Body::empty()),
                    b"",
                )
                .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(create_bucket.status(), StatusCode::OK);
        assert_eq!(
            create_bucket
                .headers()
                .get(header::LOCATION)
                .expect("CreateBucket location")
                .to_str()
                .expect("location text"),
            "/compat-bucket"
        );
        assert!(create_bucket.headers().contains_key("x-amz-request-id"));

        let list_buckets = s3_app
            .clone()
            .oneshot(
                signed_s3_request(Request::builder().uri("/").body(Body::empty()), b"")
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(list_buckets.status(), StatusCode::OK);
        assert_s3_xml_content_type(&list_buckets);
        let list_buckets_body = response_text(list_buckets).await;
        assert!(list_buckets_body.contains("<ListAllMyBucketsResult"));
        assert!(list_buckets_body.contains("<Name>compat-bucket</Name>"));

        let head_bucket = s3_app
            .clone()
            .oneshot(
                signed_s3_request(
                    Request::builder()
                        .method(Method::HEAD)
                        .uri("/compat-bucket")
                        .body(Body::empty()),
                    b"",
                )
                .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(head_bucket.status(), StatusCode::OK);
        assert!(head_bucket.headers().contains_key("x-amz-request-id"));

        let object_body = b"hello cloudflare-style s3";
        let put_object = s3_app
            .clone()
            .oneshot(
                signed_s3_request(
                    Request::builder()
                        .method(Method::PUT)
                        .uri("/compat-bucket/prefix/hello.txt")
                        .header(header::CONTENT_TYPE, "text/plain")
                        .body(Body::from(object_body.as_slice())),
                    object_body,
                )
                .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(put_object.status(), StatusCode::OK);
        assert!(put_object.headers().contains_key("ETag"));
        assert!(put_object.headers().contains_key("x-amz-request-id"));
        assert_eq!(header_value(&put_object, header::CONTENT_LENGTH), "0");
        let etag = header_value(&put_object, "ETag").to_owned();
        assert_eq!(etag, format!("\"{}\"", sha256_hex(object_body)));

        let object_row = sqlx_core::query::query(
            r#"
            SELECT v.size_bytes, v.content_type, v.object_hash, v.storage_path
            FROM objects o
            JOIN buckets b ON b.id = o.bucket_id
            JOIN object_versions v ON v.id = o.current_version_id
            WHERE b.name = $1 AND o.object_key = $2
            "#,
        )
        .bind("compat-bucket")
        .bind("prefix/hello.txt")
        .fetch_one(ctx.catalog.pool())
        .await
        .expect("object row");
        assert_eq!(
            object_row.get::<i64, _>("size_bytes"),
            object_body.len() as i64
        );
        assert_eq!(object_row.get::<String, _>("content_type"), "text/plain");
        assert_eq!(
            object_row.get::<String, _>("object_hash"),
            sha256_hex(object_body)
        );
        let stored_path = object_row.get::<String, _>("storage_path");
        assert_eq!(fs::read(&stored_path).expect("stored object"), object_body);

        let audit_count: i64 = sqlx_core::query_scalar::query_scalar(
            "SELECT COUNT(*)::bigint FROM audit_events WHERE event_type = 's3_object_put'",
        )
        .fetch_one(ctx.catalog.pool())
        .await
        .expect("audit count");
        assert_eq!(audit_count, 1);

        let default_body = b"expect header upload";
        let default_put = s3_app
            .clone()
            .oneshot(
                signed_s3_request(
                    Request::builder()
                        .method(Method::PUT)
                        .uri("/compat-bucket/default.bin")
                        .header(header::EXPECT, "100-continue")
                        .body(Body::from(default_body.as_slice())),
                    default_body,
                )
                .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(default_put.status(), StatusCode::OK);
        assert_eq!(header_value(&default_put, header::CONTENT_LENGTH), "0");

        let bucket_location = s3_app
            .clone()
            .oneshot(
                signed_s3_request(
                    Request::builder()
                        .uri("/compat-bucket?location")
                        .body(Body::empty()),
                    b"",
                )
                .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(bucket_location.status(), StatusCode::OK);
        assert!(
            response_text(bucket_location)
                .await
                .contains("<LocationConstraint")
        );

        let default_head = s3_app
            .clone()
            .oneshot(
                signed_s3_request(
                    Request::builder()
                        .method(Method::HEAD)
                        .uri("/compat-bucket/default.bin")
                        .body(Body::empty()),
                    b"",
                )
                .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(default_head.status(), StatusCode::OK);
        assert_eq!(
            header_value(&default_head, header::CONTENT_TYPE),
            "application/octet-stream"
        );

        let list_objects = s3_app
            .clone()
            .oneshot(
                signed_s3_request(
                    Request::builder()
                        .uri("/compat-bucket?list-type=2&prefix=prefix/")
                        .body(Body::empty()),
                    b"",
                )
                .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(list_objects.status(), StatusCode::OK);
        assert_s3_xml_content_type(&list_objects);
        let list_objects_body = response_text(list_objects).await;
        assert!(list_objects_body.contains("<ListBucketResult"));
        assert!(list_objects_body.contains("<Name>compat-bucket</Name>"));
        assert!(list_objects_body.contains("<Prefix>prefix/</Prefix>"));
        assert!(list_objects_body.contains("<Key>prefix/hello.txt</Key>"));
        assert!(list_objects_body.contains("<KeyCount>1</KeyCount>"));

        let head_object = s3_app
            .clone()
            .oneshot(
                signed_s3_request(
                    Request::builder()
                        .method(Method::HEAD)
                        .uri("/compat-bucket/prefix/hello.txt")
                        .body(Body::empty()),
                    b"",
                )
                .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(head_object.status(), StatusCode::OK);
        assert_eq!(
            head_object
                .headers()
                .get(header::CONTENT_TYPE)
                .expect("HeadObject content type")
                .to_str()
                .expect("content type text"),
            "text/plain"
        );
        assert_eq!(
            head_object
                .headers()
                .get(header::CONTENT_LENGTH)
                .expect("HeadObject content length")
                .to_str()
                .expect("content length text"),
            object_body.len().to_string()
        );
        assert!(head_object.headers().contains_key("ETag"));
        assert!(head_object.headers().contains_key("Last-Modified"));
        assert!(head_object.headers().contains_key("x-amz-bucket-region"));

        let get_object = s3_app
            .clone()
            .oneshot(
                signed_s3_request(
                    Request::builder()
                        .uri("/compat-bucket/prefix/hello.txt")
                        .body(Body::empty()),
                    b"",
                )
                .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(get_object.status(), StatusCode::OK);
        assert_eq!(response_bytes(get_object).await, object_body.as_slice());

        let copy_object = s3_app
            .clone()
            .oneshot(
                signed_s3_request(
                    Request::builder()
                        .method(Method::PUT)
                        .uri("/compat-bucket/prefix/copied.txt")
                        .header("x-amz-copy-source", "/compat-bucket/prefix/hello.txt")
                        .body(Body::empty()),
                    b"",
                )
                .expect("valid copy object request"),
            )
            .await
            .expect("router response");
        assert_eq!(copy_object.status(), StatusCode::OK);
        let copy_object_body = response_text(copy_object).await;
        assert!(copy_object_body.contains("<CopyObjectResult"));
        assert!(copy_object_body.contains(&sha256_hex(object_body)));

        let get_copied = s3_app
            .clone()
            .oneshot(
                signed_s3_request(
                    Request::builder()
                        .uri("/compat-bucket/prefix/copied.txt")
                        .body(Body::empty()),
                    b"",
                )
                .expect("valid get copied object request"),
            )
            .await
            .expect("router response");
        assert_eq!(get_copied.status(), StatusCode::OK);
        assert_eq!(response_bytes(get_copied).await, object_body.as_slice());

        let presigned_get = s3_app
            .clone()
            .oneshot(
                presigned_s3_request("/compat-bucket/prefix/hello.txt", 120)
                    .body(Body::empty())
                    .expect("valid presigned request"),
            )
            .await
            .expect("router response");
        assert_eq!(presigned_get.status(), StatusCode::OK);
        assert_eq!(response_bytes(presigned_get).await, object_body.as_slice());

        let expired_presigned_get = s3_app
            .clone()
            .oneshot(
                expired_presigned_s3_request("/compat-bucket/prefix/hello.txt")
                    .body(Body::empty())
                    .expect("valid expired presigned request"),
            )
            .await
            .expect("router response");
        assert_eq!(expired_presigned_get.status(), StatusCode::FORBIDDEN);
        assert!(
            response_text(expired_presigned_get)
                .await
                .contains("presigned URL has expired")
        );

        let range_get = s3_app
            .clone()
            .oneshot(
                signed_s3_request(
                    Request::builder()
                        .uri("/compat-bucket/prefix/hello.txt")
                        .header(header::RANGE, "bytes=0-4")
                        .body(Body::empty()),
                    b"",
                )
                .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(range_get.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(
            range_get
                .headers()
                .get(header::CONTENT_RANGE)
                .expect("Content-Range")
                .to_str()
                .expect("content-range text"),
            format!("bytes 0-4/{}", object_body.len())
        );
        assert_eq!(response_text(range_get).await, "hello");

        let missing_object = s3_app
            .clone()
            .oneshot(
                signed_s3_request(
                    Request::builder()
                        .uri("/compat-bucket/missing.txt")
                        .body(Body::empty()),
                    b"",
                )
                .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(missing_object.status(), StatusCode::NOT_FOUND);
        let missing_object_body = response_text(missing_object).await;
        assert!(missing_object_body.contains("<Code>NoSuchKey</Code>"));
        assert!(missing_object_body.contains("<BucketName>compat-bucket</BucketName>"));
        assert!(missing_object_body.contains("<Key>missing.txt</Key>"));

        let delete_non_empty_bucket = s3_app
            .clone()
            .oneshot(
                signed_s3_request(
                    Request::builder()
                        .method(Method::DELETE)
                        .uri("/compat-bucket")
                        .body(Body::empty()),
                    b"",
                )
                .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(delete_non_empty_bucket.status(), StatusCode::CONFLICT);
        assert!(
            response_text(delete_non_empty_bucket)
                .await
                .contains("<Code>BucketNotEmpty</Code>")
        );

        let delete_object = s3_app
            .clone()
            .oneshot(
                signed_s3_request(
                    Request::builder()
                        .method(Method::DELETE)
                        .uri("/compat-bucket/prefix/hello.txt")
                        .body(Body::empty()),
                    b"",
                )
                .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(delete_object.status(), StatusCode::NO_CONTENT);
        assert!(delete_object.headers().contains_key("x-amz-request-id"));

        let delete_objects_body = "<Delete><Object><Key>prefix/copied.txt</Key></Object><Object><Key>missing-batch.txt</Key></Object></Delete>";
        let delete_objects = s3_app
            .clone()
            .oneshot(
                signed_s3_request(
                    Request::builder()
                        .method(Method::POST)
                        .uri("/compat-bucket?delete")
                        .body(Body::from(delete_objects_body)),
                    delete_objects_body.as_bytes(),
                )
                .expect("valid delete objects request"),
            )
            .await
            .expect("router response");
        assert_eq!(delete_objects.status(), StatusCode::OK);
        let delete_objects_body = response_text(delete_objects).await;
        assert!(delete_objects_body.contains("<DeleteResult"));
        assert!(delete_objects_body.contains("<Key>prefix/copied.txt</Key>"));
        assert!(delete_objects_body.contains("<Key>missing-batch.txt</Key>"));

        let get_batch_deleted = s3_app
            .clone()
            .oneshot(
                signed_s3_request(
                    Request::builder()
                        .uri("/compat-bucket/prefix/copied.txt")
                        .body(Body::empty()),
                    b"",
                )
                .expect("valid get batch deleted request"),
            )
            .await
            .expect("router response");
        assert_eq!(get_batch_deleted.status(), StatusCode::FORBIDDEN);

        let get_deleted_object = s3_app
            .clone()
            .oneshot(
                signed_s3_request(
                    Request::builder()
                        .uri("/compat-bucket/prefix/hello.txt")
                        .body(Body::empty()),
                    b"",
                )
                .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(get_deleted_object.status(), StatusCode::FORBIDDEN);
        assert!(
            response_text(get_deleted_object)
                .await
                .contains("<Code>InvalidObjectState</Code>")
        );

        let delete_empty_bucket = s3_app
            .oneshot(
                signed_s3_request(
                    Request::builder()
                        .method(Method::DELETE)
                        .uri("/compat-bucket")
                        .body(Body::empty()),
                    b"",
                )
                .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(delete_empty_bucket.status(), StatusCode::NO_CONTENT);
        assert!(
            delete_empty_bucket
                .headers()
                .contains_key("x-amz-request-id")
        );
    }

    #[tokio::test]
    async fn s3_put_overwrites_active_object_and_recreates_deleted_key() {
        let Some(ctx) = TestContext::new("s3-put-overwrite-and-recreate").await else {
            return;
        };
        let _guard = ctx.guard;
        let s3_app = ctx.s3_app.clone();

        let create_bucket = s3_app
            .clone()
            .oneshot(
                signed_s3_request(
                    Request::builder()
                        .method(Method::PUT)
                        .uri("/s3-reupload")
                        .body(Body::empty()),
                    b"",
                )
                .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(create_bucket.status(), StatusCode::OK);

        let first_body = b"primeiro";
        let first_put = s3_app
            .clone()
            .oneshot(
                signed_s3_request(
                    Request::builder()
                        .method(Method::PUT)
                        .uri("/s3-reupload/hello.txt")
                        .body(Body::from(first_body.as_slice())),
                    first_body,
                )
                .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(first_put.status(), StatusCode::OK);

        let overwrite_body = b"sobrescrito";
        let overwrite_put = s3_app
            .clone()
            .oneshot(
                signed_s3_request(
                    Request::builder()
                        .method(Method::PUT)
                        .uri("/s3-reupload/hello.txt")
                        .body(Body::from(overwrite_body.as_slice())),
                    overwrite_body,
                )
                .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(overwrite_put.status(), StatusCode::OK);

        let get_overwritten = s3_app
            .clone()
            .oneshot(
                signed_s3_request(
                    Request::builder()
                        .uri("/s3-reupload/hello.txt")
                        .body(Body::empty()),
                    b"",
                )
                .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(get_overwritten.status(), StatusCode::OK);
        assert_eq!(
            response_bytes(get_overwritten).await,
            overwrite_body.as_slice()
        );

        let delete = s3_app
            .clone()
            .oneshot(
                signed_s3_request(
                    Request::builder()
                        .method(Method::DELETE)
                        .uri("/s3-reupload/hello.txt")
                        .body(Body::empty()),
                    b"",
                )
                .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(delete.status(), StatusCode::NO_CONTENT);

        let recreated_body = b"segundo";
        let recreated_put = s3_app
            .clone()
            .oneshot(
                signed_s3_request(
                    Request::builder()
                        .method(Method::PUT)
                        .uri("/s3-reupload/hello.txt")
                        .body(Body::from(recreated_body.as_slice())),
                    recreated_body,
                )
                .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(recreated_put.status(), StatusCode::OK);

        let get_recreated = s3_app
            .oneshot(
                signed_s3_request(
                    Request::builder()
                        .uri("/s3-reupload/hello.txt")
                        .body(Body::empty()),
                    b"",
                )
                .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(get_recreated.status(), StatusCode::OK);
        assert_eq!(
            response_bytes(get_recreated).await,
            recreated_body.as_slice()
        );
    }

    #[tokio::test]
    async fn s3_multipart_upload_lifecycle_persists_completed_object_and_cleans_abort() {
        let Some(ctx) = TestContext::new("s3-multipart-lifecycle").await else {
            return;
        };
        let _guard = ctx.guard;
        let s3_app = ctx.s3_app.clone();

        let create_bucket = s3_app
            .clone()
            .oneshot(
                signed_s3_request(
                    Request::builder()
                        .method(Method::PUT)
                        .uri("/multipart-bucket")
                        .body(Body::empty()),
                    b"",
                )
                .expect("valid create bucket request"),
            )
            .await
            .expect("router response");
        assert_eq!(create_bucket.status(), StatusCode::OK);

        let initiate = s3_app
            .clone()
            .oneshot(
                signed_s3_request(
                    Request::builder()
                        .method(Method::POST)
                        .uri("/multipart-bucket/large.bin?uploads")
                        .header(header::CONTENT_TYPE, "application/octet-stream")
                        .body(Body::empty()),
                    b"",
                )
                .expect("valid initiate request"),
            )
            .await
            .expect("router response");
        assert_eq!(initiate.status(), StatusCode::OK);
        let initiate_body = response_text(initiate).await;
        assert!(initiate_body.contains("<InitiateMultipartUploadResult"));
        let upload_id = xml_value(&initiate_body, "UploadId");
        assert!(!upload_id.is_empty());

        let list_uploads = s3_app
            .clone()
            .oneshot(
                signed_s3_request(
                    Request::builder()
                        .uri("/multipart-bucket?uploads")
                        .body(Body::empty()),
                    b"",
                )
                .expect("valid list multipart uploads request"),
            )
            .await
            .expect("router response");
        assert_eq!(list_uploads.status(), StatusCode::OK);
        let list_uploads_body = response_text(list_uploads).await;
        assert!(list_uploads_body.contains("<ListMultipartUploadsResult"));
        assert!(list_uploads_body.contains("<Key>large.bin</Key>"));
        assert!(list_uploads_body.contains(&format!("<UploadId>{upload_id}</UploadId>")));

        let part_two = b"-part-two";
        let upload_part_two = s3_app
            .clone()
            .oneshot(
                signed_s3_request(
                    Request::builder()
                        .method(Method::PUT)
                        .uri(format!(
                            "/multipart-bucket/large.bin?partNumber=2&uploadId={upload_id}"
                        ))
                        .body(Body::from(part_two.as_slice())),
                    part_two,
                )
                .expect("valid upload part 2 request"),
            )
            .await
            .expect("router response");
        assert_eq!(upload_part_two.status(), StatusCode::OK);
        let part_two_etag = header_value(&upload_part_two, "ETag")
            .trim_matches('"')
            .to_owned();
        assert_eq!(part_two_etag, sha256_hex(part_two));

        let stale_part_one = b"stale";
        let upload_stale_part_one = s3_app
            .clone()
            .oneshot(
                signed_s3_request(
                    Request::builder()
                        .method(Method::PUT)
                        .uri(format!(
                            "/multipart-bucket/large.bin?partNumber=1&uploadId={upload_id}"
                        ))
                        .body(Body::from(stale_part_one.as_slice())),
                    stale_part_one,
                )
                .expect("valid upload stale part 1 request"),
            )
            .await
            .expect("router response");
        assert_eq!(upload_stale_part_one.status(), StatusCode::OK);

        let part_one = b"part-one";
        let upload_part_one = s3_app
            .clone()
            .oneshot(
                signed_s3_request(
                    Request::builder()
                        .method(Method::PUT)
                        .uri(format!(
                            "/multipart-bucket/large.bin?partNumber=1&uploadId={upload_id}"
                        ))
                        .body(Body::from(part_one.as_slice())),
                    part_one,
                )
                .expect("valid replacement part 1 request"),
            )
            .await
            .expect("router response");
        assert_eq!(upload_part_one.status(), StatusCode::OK);
        let part_one_etag = header_value(&upload_part_one, "ETag")
            .trim_matches('"')
            .to_owned();
        assert_eq!(part_one_etag, sha256_hex(part_one));

        let list_parts = s3_app
            .clone()
            .oneshot(
                signed_s3_request(
                    Request::builder()
                        .uri(format!("/multipart-bucket/large.bin?uploadId={upload_id}"))
                        .body(Body::empty()),
                    b"",
                )
                .expect("valid list parts request"),
            )
            .await
            .expect("router response");
        assert_eq!(list_parts.status(), StatusCode::OK);
        let list_parts_body = response_text(list_parts).await;
        assert!(list_parts_body.contains("<ListPartsResult"));
        assert!(list_parts_body.contains("<PartNumber>1</PartNumber>"));
        assert!(list_parts_body.contains("<PartNumber>2</PartNumber>"));
        assert!(list_parts_body.contains(&part_one_etag));
        assert!(!list_parts_body.contains(&sha256_hex(stale_part_one)));

        let invalid_complete_body = format!(
            "<CompleteMultipartUpload><Part><PartNumber>1</PartNumber><ETag>0000</ETag></Part><Part><PartNumber>2</PartNumber><ETag>{part_two_etag}</ETag></Part></CompleteMultipartUpload>"
        );
        let invalid_complete = s3_app
            .clone()
            .oneshot(
                signed_s3_request(
                    Request::builder()
                        .method(Method::POST)
                        .uri(format!("/multipart-bucket/large.bin?uploadId={upload_id}"))
                        .body(Body::from(invalid_complete_body.clone())),
                    invalid_complete_body.as_bytes(),
                )
                .expect("valid invalid complete request"),
            )
            .await
            .expect("router response");
        assert_eq!(invalid_complete.status(), StatusCode::BAD_REQUEST);
        assert!(
            response_text(invalid_complete)
                .await
                .contains("<Code>InvalidRequest</Code>")
        );

        let complete_body = format!(
            "<CompleteMultipartUpload><Part><PartNumber>1</PartNumber><ETag>\"{part_one_etag}\"</ETag></Part><Part><PartNumber>2</PartNumber><ETag>\"{part_two_etag}\"</ETag></Part></CompleteMultipartUpload>"
        );
        let complete = s3_app
            .clone()
            .oneshot(
                signed_s3_request(
                    Request::builder()
                        .method(Method::POST)
                        .uri(format!("/multipart-bucket/large.bin?uploadId={upload_id}"))
                        .body(Body::from(complete_body.clone())),
                    complete_body.as_bytes(),
                )
                .expect("valid complete request"),
            )
            .await
            .expect("router response");
        assert_eq!(complete.status(), StatusCode::OK);
        let complete_body_text = response_text(complete).await;
        assert!(complete_body_text.contains("<CompleteMultipartUploadResult"));

        let expected_body = [part_one.as_slice(), part_two.as_slice()].concat();
        let get_completed = s3_app
            .clone()
            .oneshot(
                signed_s3_request(
                    Request::builder()
                        .uri("/multipart-bucket/large.bin")
                        .body(Body::empty()),
                    b"",
                )
                .expect("valid get completed object request"),
            )
            .await
            .expect("router response");
        assert_eq!(get_completed.status(), StatusCode::OK);
        assert_eq!(response_bytes(get_completed).await, expected_body);

        let manifest_count: i64 =
            sqlx_core::query_scalar::query_scalar("SELECT COUNT(*)::bigint FROM object_manifests")
                .fetch_one(ctx.catalog.pool())
                .await
                .expect("manifest count");
        let active_uploads: i64 = sqlx_core::query_scalar::query_scalar(
            "SELECT COUNT(*)::bigint FROM s3_multipart_uploads WHERE completed_at IS NULL AND aborted_at IS NULL",
        )
        .fetch_one(ctx.catalog.pool())
        .await
        .expect("active multipart upload count");
        assert_eq!(manifest_count, 1);
        assert_eq!(active_uploads, 0);

        let list_parts_after_complete = s3_app
            .clone()
            .oneshot(
                signed_s3_request(
                    Request::builder()
                        .uri(format!("/multipart-bucket/large.bin?uploadId={upload_id}"))
                        .body(Body::empty()),
                    b"",
                )
                .expect("valid list parts after complete request"),
            )
            .await
            .expect("router response");
        assert_eq!(list_parts_after_complete.status(), StatusCode::OK);
        let list_parts_after_complete_body = response_text(list_parts_after_complete).await;
        assert!(!list_parts_after_complete_body.contains("<PartNumber>"));

        let abort_initiate = s3_app
            .clone()
            .oneshot(
                signed_s3_request(
                    Request::builder()
                        .method(Method::POST)
                        .uri("/multipart-bucket/abort.bin?uploads")
                        .body(Body::empty()),
                    b"",
                )
                .expect("valid abort initiate request"),
            )
            .await
            .expect("router response");
        assert_eq!(abort_initiate.status(), StatusCode::OK);
        let abort_upload_id = xml_value(&response_text(abort_initiate).await, "UploadId");
        let abort_part = b"abort-me";
        let uploaded_abort_part = s3_app
            .clone()
            .oneshot(
                signed_s3_request(
                    Request::builder()
                        .method(Method::PUT)
                        .uri(format!(
                            "/multipart-bucket/abort.bin?partNumber=1&uploadId={abort_upload_id}"
                        ))
                        .body(Body::from(abort_part.as_slice())),
                    abort_part,
                )
                .expect("valid abort part request"),
            )
            .await
            .expect("router response");
        assert_eq!(uploaded_abort_part.status(), StatusCode::OK);

        let abort = s3_app
            .clone()
            .oneshot(
                signed_s3_request(
                    Request::builder()
                        .method(Method::DELETE)
                        .uri(format!(
                            "/multipart-bucket/abort.bin?uploadId={abort_upload_id}"
                        ))
                        .body(Body::empty()),
                    b"",
                )
                .expect("valid abort request"),
            )
            .await
            .expect("router response");
        assert_eq!(abort.status(), StatusCode::NO_CONTENT);

        let complete_after_abort_body = format!(
            "<CompleteMultipartUpload><Part><PartNumber>1</PartNumber><ETag>{}</ETag></Part></CompleteMultipartUpload>",
            sha256_hex(abort_part)
        );
        let complete_after_abort = s3_app
            .oneshot(
                signed_s3_request(
                    Request::builder()
                        .method(Method::POST)
                        .uri(format!(
                            "/multipart-bucket/abort.bin?uploadId={abort_upload_id}"
                        ))
                        .body(Body::from(complete_after_abort_body.clone())),
                    complete_after_abort_body.as_bytes(),
                )
                .expect("valid complete after abort request"),
            )
            .await
            .expect("router response");
        assert_eq!(complete_after_abort.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn admin_can_create_list_and_revoke_s3_access_keys() {
        let Some(ctx) = TestContext::new("s3-access-key-admin").await else {
            return;
        };
        let _guard = ctx.guard;
        let app = ctx.app.clone();
        let admin_cookie = login_cookie(app.clone()).await;

        let created = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/admin/s3-access-keys")
                    .header(header::COOKIE, &admin_cookie)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from("{}"))
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(created.status(), StatusCode::CREATED);
        let created_body: serde_json::Value =
            serde_json::from_str(&response_text(created).await).expect("created key JSON");
        let access_key_id = created_body["accessKeyId"].as_str().expect("access key id");
        let secret_access_key = created_body["secretAccessKey"]
            .as_str()
            .expect("secret access key");
        let id = created_body["id"].as_str().expect("key id");
        assert!(access_key_id.starts_with("PMK"));
        assert!(!secret_access_key.is_empty());

        let list = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/admin/s3-access-keys")
                    .header(header::COOKIE, &admin_cookie)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(list.status(), StatusCode::OK);
        let list_body: serde_json::Value =
            serde_json::from_str(&response_text(list).await).expect("list key JSON");
        assert_eq!(list_body["page"], 1);
        assert_eq!(list_body["pageSize"], 10);
        assert_eq!(list_body["total"], 2);
        assert_eq!(list_body["totalPages"], 1);
        let keys = list_body["items"].as_array().expect("keys array");
        assert_eq!(keys.len(), 2);
        let listed_created = keys
            .iter()
            .find(|key| key["accessKeyId"] == access_key_id)
            .expect("created key listed");
        assert_eq!(listed_created["isActive"], true);
        assert!(listed_created.get("secretAccessKey").is_none());
        assert!(listed_created.get("secretKeyHash").is_none());

        let second_page = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/admin/s3-access-keys?page=2&pageSize=1")
                    .header(header::COOKIE, &admin_cookie)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(second_page.status(), StatusCode::OK);
        let second_page_body: serde_json::Value =
            serde_json::from_str(&response_text(second_page).await).expect("second page JSON");
        assert_eq!(second_page_body["page"], 2);
        assert_eq!(second_page_body["pageSize"], 1);
        assert_eq!(second_page_body["total"], 2);
        assert_eq!(second_page_body["totalPages"], 2);
        assert_eq!(
            second_page_body["items"]
                .as_array()
                .expect("second page keys")
                .len(),
            1
        );

        let row: (String, Option<String>, bool, String, Vec<u8>) = sqlx_core::query_as::query_as(
            r#"
            SELECT access_key_id, user_id::text, is_active, secret_key_hash, secret_key_ciphertext
            FROM s3_access_keys
            WHERE access_key_id = $1
            "#,
        )
        .bind(access_key_id)
        .fetch_one(ctx.catalog.pool())
        .await
        .expect("created S3 key row");
        assert_eq!(row.0, access_key_id);
        assert!(row.1.is_some());
        assert!(row.2);
        assert_ne!(row.3, secret_access_key);
        assert!(!row.4.is_empty());
        assert_ne!(row.4, secret_access_key.as_bytes());

        let revoke = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::DELETE)
                    .uri(format!("/api/admin/s3/access-keys/{id}"))
                    .header(header::COOKIE, &admin_cookie)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(revoke.status(), StatusCode::NO_CONTENT);

        let list_after_revoke = app
            .oneshot(
                Request::builder()
                    .uri("/api/admin/s3-access-keys")
                    .header(header::COOKIE, &admin_cookie)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(list_after_revoke.status(), StatusCode::OK);
        let list_after_revoke_body: serde_json::Value =
            serde_json::from_str(&response_text(list_after_revoke).await)
                .expect("list revoked key JSON");
        let revoked_key = list_after_revoke_body
            .get("items")
            .and_then(|items| items.as_array())
            .expect("keys array")
            .iter()
            .find(|key| key["accessKeyId"] == access_key_id)
            .expect("revoked key listed");
        assert_eq!(revoked_key["isActive"], false);
        assert!(revoked_key["revokedAt"].as_str().is_some());
        assert!(revoked_key.get("secretAccessKey").is_none());
        assert!(revoked_key.get("secretKeyHash").is_none());
    }

    #[tokio::test]
    async fn managed_s3_access_key_signs_s3_requests_and_fails_after_revocation() {
        let Some(ctx) = TestContext::new("managed-s3-key-auth").await else {
            return;
        };
        let _guard = ctx.guard;
        let app = ctx.app.clone();
        let s3_app = ctx.s3_app.clone();
        let admin_cookie = login_cookie(app.clone()).await;

        let created = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/admin/s3-access-keys")
                    .header(header::COOKIE, &admin_cookie)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from("{}"))
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(created.status(), StatusCode::CREATED);
        let created_body: serde_json::Value =
            serde_json::from_str(&response_text(created).await).expect("created key JSON");
        let access_key_id = created_body["accessKeyId"]
            .as_str()
            .expect("access key id")
            .to_owned();
        let id = created_body["id"].as_str().expect("key id").to_owned();
        let secret_access_key = created_body["secretAccessKey"]
            .as_str()
            .expect("secret access key")
            .to_owned();

        let create_bucket = s3_app
            .clone()
            .oneshot(
                signed_s3_request_with_credentials(
                    Request::builder()
                        .method(Method::PUT)
                        .uri("/managed-key-bucket")
                        .body(Body::empty()),
                    b"",
                    &access_key_id,
                    &secret_access_key,
                )
                .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(create_bucket.status(), StatusCode::OK);

        let last_used_at: Option<chrono::DateTime<chrono::Utc>> =
            sqlx_core::query_scalar::query_scalar(
                "SELECT last_used_at FROM s3_access_keys WHERE access_key_id = $1",
            )
            .bind(&access_key_id)
            .fetch_one(ctx.catalog.pool())
            .await
            .expect("last used at");
        assert!(last_used_at.is_some());

        let revoke = app
            .oneshot(
                Request::builder()
                    .method(Method::DELETE)
                    .uri(format!("/api/admin/s3/access-keys/{id}"))
                    .header(header::COOKIE, &admin_cookie)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(revoke.status(), StatusCode::NO_CONTENT);

        let after_revoke = s3_app
            .oneshot(
                signed_s3_request_with_credentials(
                    Request::builder()
                        .uri("/managed-key-bucket")
                        .body(Body::empty()),
                    b"",
                    &access_key_id,
                    &secret_access_key,
                )
                .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(after_revoke.status(), StatusCode::FORBIDDEN);
        assert!(
            response_text(after_revoke)
                .await
                .contains("<Code>InvalidAccessKeyId</Code>")
        );
    }

    #[tokio::test]
    async fn admin_sensitive_credentials_return_secret_only_on_create() {
        let Some(ctx) = TestContext::new("sensitive-admin-credentials").await else {
            return;
        };
        let _guard = ctx.guard;
        let app = ctx.app.clone();
        let admin_cookie = login_cookie(app.clone()).await;

        let create_application = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/admin/application-credentials")
                    .header(header::COOKIE, &admin_cookie)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"name":"sdk-mobile","scopes":["origin:objects:read"]}"#,
                    ))
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(create_application.status(), StatusCode::CREATED);
        let application_body = json_body(create_application).await;
        let application_id = application_body["credential"]["id"]
            .as_str()
            .expect("application id");
        let application_token = application_body["token"]
            .as_str()
            .expect("application token");
        assert!(application_token.starts_with("pm_app_"));

        let list_applications = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/admin/application-credentials")
                    .header(header::COOKIE, &admin_cookie)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(list_applications.status(), StatusCode::OK);
        let applications_body = json_body(list_applications).await;
        let listed_application = applications_body
            .as_array()
            .expect("applications array")
            .iter()
            .find(|credential| credential["id"] == application_id)
            .expect("application listed");
        assert!(listed_application.get("token").is_none());
        assert!(listed_application.get("tokenHash").is_none());

        let application_hash: String = sqlx_core::query_scalar::query_scalar(
            "SELECT token_hash FROM application_credentials WHERE id = $1::uuid",
        )
        .bind(application_id)
        .fetch_one(ctx.catalog.pool())
        .await
        .expect("application token hash");
        assert_ne!(application_hash, application_token);

        let create_replica = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/admin/replicas")
                    .header(header::COOKIE, &admin_cookie)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"name":"edge-secret-check","allowedBuckets":["media"]}"#,
                    ))
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(create_replica.status(), StatusCode::CREATED);
        let replica_body = json_body(create_replica).await;
        let replica_id = replica_body["replica"]["id"].as_str().expect("replica id");
        let replica_token = replica_body["token"].as_str().expect("replica token");
        assert!(replica_token.starts_with("pm_rep_"));

        let list_replicas = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/admin/replicas")
                    .header(header::COOKIE, &admin_cookie)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(list_replicas.status(), StatusCode::OK);
        let replicas_body = json_body(list_replicas).await;
        let listed_replica = replicas_body
            .as_array()
            .expect("replicas array")
            .iter()
            .find(|replica| replica["id"] == replica_id)
            .expect("replica listed");
        assert!(listed_replica.get("token").is_none());
        assert!(listed_replica.get("tokenHash").is_none());

        let replica_hash: String = sqlx_core::query_scalar::query_scalar(
            "SELECT token_hash FROM replica_credentials WHERE id = $1::uuid",
        )
        .bind(replica_id)
        .fetch_one(ctx.catalog.pool())
        .await
        .expect("replica token hash");
        assert_ne!(replica_hash, replica_token);

        let leaked_audit_events: i64 = sqlx_core::query_scalar::query_scalar(
            "SELECT COUNT(*)::bigint FROM audit_events WHERE metadata::text LIKE $1 OR metadata::text LIKE $2",
        )
        .bind(format!("%{application_token}%"))
        .bind(format!("%{replica_token}%"))
        .fetch_one(ctx.catalog.pool())
        .await
        .expect("audit leak count");
        assert_eq!(leaked_audit_events, 0);
    }

    #[tokio::test]
    async fn admin_object_routes_require_session_and_handle_multipart_lifecycle() {
        let Some(ctx) = TestContext::new("admin-object-routes").await else {
            return;
        };
        let _guard = ctx.guard;
        let app = ctx.app.clone();
        let admin_cookie = login_cookie(app.clone()).await;

        let create_bucket = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/admin/buckets")
                    .header(header::COOKIE, &admin_cookie)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"name":"admin-objects"}"#))
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(create_bucket.status(), StatusCode::CREATED);

        let unauthorized = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/admin/buckets/admin-objects/objects")
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

        let boundary = "pontemesh-test-boundary";
        let object_body = b"hello admin object routes";
        let multipart = multipart_body(boundary, "folder/hello world.txt", object_body);
        let upload = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/admin/buckets/admin-objects/objects")
                    .header(header::COOKIE, &admin_cookie)
                    .header(
                        header::CONTENT_TYPE,
                        format!("multipart/form-data; boundary={boundary}"),
                    )
                    .body(Body::from(multipart))
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(upload.status(), StatusCode::CREATED);
        let upload_body = response_text(upload).await;
        assert!(upload_body.contains("folder/hello world.txt"));

        let duplicate_multipart = multipart_body(boundary, "folder/hello world.txt", b"duplicate");
        let duplicate_upload = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/admin/buckets/admin-objects/objects")
                    .header(header::COOKIE, &admin_cookie)
                    .header(
                        header::CONTENT_TYPE,
                        format!("multipart/form-data; boundary={boundary}"),
                    )
                    .body(Body::from(duplicate_multipart))
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(duplicate_upload.status(), StatusCode::BAD_REQUEST);
        assert!(
            response_text(duplicate_upload)
                .await
                .contains("active object already exists in bucket")
        );

        let list = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/admin/buckets/admin-objects/objects?query=hello&page=1&pageSize=10")
                    .header(header::COOKIE, &admin_cookie)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(list.status(), StatusCode::OK);
        let list_body = response_text(list).await;
        assert!(list_body.contains("folder/hello world.txt"));
        assert!(list_body.contains(r#""totalItems":1"#));

        let download = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/admin/buckets/admin-objects/objects/folder/hello%20world.txt")
                    .header(header::COOKIE, &admin_cookie)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(download.status(), StatusCode::OK);
        assert_eq!(header_value(&download, header::CONTENT_TYPE), "text/plain");
        assert_eq!(response_bytes(download).await.as_ref(), object_body);

        let delete = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::DELETE)
                    .uri("/api/admin/buckets/admin-objects/objects/folder/hello%20world.txt")
                    .header(header::COOKIE, &admin_cookie)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(delete.status(), StatusCode::OK);

        let list_after_delete = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/admin/buckets/admin-objects/objects?query=hello&page=1&pageSize=10")
                    .header(header::COOKIE, &admin_cookie)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(list_after_delete.status(), StatusCode::OK);
        let list_after_delete_body = response_text(list_after_delete).await;
        assert!(list_after_delete_body.contains(r#""items":[]"#));
        assert!(list_after_delete_body.contains(r#""totalItems":0"#));

        let replacement_body = b"hello admin object routes replacement";
        let replacement_multipart =
            multipart_body(boundary, "folder/hello world.txt", replacement_body);
        let replacement_upload = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/admin/buckets/admin-objects/objects")
                    .header(header::COOKIE, &admin_cookie)
                    .header(
                        header::CONTENT_TYPE,
                        format!("multipart/form-data; boundary={boundary}"),
                    )
                    .body(Body::from(replacement_multipart))
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(replacement_upload.status(), StatusCode::CREATED);

        let list_after_reupload = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/admin/buckets/admin-objects/objects?query=hello&page=1&pageSize=10")
                    .header(header::COOKIE, &admin_cookie)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(list_after_reupload.status(), StatusCode::OK);
        let list_after_reupload_body = response_text(list_after_reupload).await;
        assert!(list_after_reupload_body.contains("folder/hello world.txt"));
        assert!(list_after_reupload_body.contains(r#""totalItems":1"#));

        let replacement_download = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/admin/buckets/admin-objects/objects/folder/hello%20world.txt")
                    .header(header::COOKIE, &admin_cookie)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(replacement_download.status(), StatusCode::OK);
        assert_eq!(
            response_bytes(replacement_download).await.as_ref(),
            replacement_body
        );

        let bucket_summary = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/admin/buckets/admin-objects")
                    .header(header::COOKIE, &admin_cookie)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(bucket_summary.status(), StatusCode::OK);
        let bucket_summary_body: serde_json::Value =
            serde_json::from_str(&response_text(bucket_summary).await).expect("bucket JSON");
        assert_eq!(bucket_summary_body["objectCount"], 1);
        assert_eq!(
            bucket_summary_body["totalBytes"],
            replacement_body.len() as i64
        );

        let audit_counts: (i64, i64) = sqlx_core::query_as::query_as(
            r#"
            SELECT
                COUNT(*) FILTER (WHERE event_type = 'object_uploaded')::bigint AS uploaded,
                COUNT(*) FILTER (WHERE event_type = 'object_deleted')::bigint AS deleted
            FROM audit_events
            "#,
        )
        .fetch_one(ctx.catalog.pool())
        .await
        .expect("audit counts");
        assert_eq!(audit_counts, (2, 1));
    }

    #[tokio::test]
    async fn replica_edge_instance_blocks_origin_exclusive_admin_routes_at_runtime() {
        let Some(ctx) =
            TestContext::new_with_role("replica-edge-runtime-role", InstanceRole::ReplicaEdge)
                .await
        else {
            return;
        };
        let _guard = ctx.guard;
        let app = ctx.app.clone();
        let admin_cookie = login_cookie(app.clone()).await;

        let create_bucket = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/admin/buckets")
                    .header(header::COOKIE, &admin_cookie)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"name":"replica-must-not-create-origin-bucket"}"#,
                    ))
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(create_bucket.status(), StatusCode::CONFLICT);
        assert!(
            response_text(create_bucket)
                .await
                .contains("operation requires instance role origin")
        );
    }

    #[tokio::test]
    async fn postgres_origin_catalog_policy_metrics_revocation_and_replica_flow() {
        let Some(ctx) = TestContext::new("postgres-origin-flow").await else {
            return;
        };
        let _guard = ctx.guard;
        let (app, token) = create_application(ctx.app.clone(), &ctx.catalog).await;
        let s3_app = ctx.s3_app.clone();
        let authorization = format!("Bearer {token}");

        assert_status(
            s3_app
                .clone()
                .oneshot(
                    signed_s3_request(
                        Request::builder()
                            .method(Method::PUT)
                            .uri("/test-bucket")
                            .body(Body::empty()),
                        b"",
                    )
                    .expect("valid request"),
                )
                .await
                .expect("router response"),
            StatusCode::OK,
        );

        let admin_cookie = login_cookie(app.clone()).await;
        let policy_update = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::PUT)
                    .uri("/api/admin/buckets/test-bucket/policy")
                    .header(header::COOKIE, &admin_cookie)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"accessPackageTtlSeconds":120,"fragmentSizeBytes":1024,"allowReplicaEdge":true,"allowPeerSharing":false,"sourceSelectionStrategy":"ORIGIN_REPLICA_EDGE","fragmentPriorityStrategy":"MANIFEST_ORDER","failureThreshold":3,"fallbackMode":"ORIGIN_RANGE"}"#,
                    ))
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(policy_update.status(), StatusCode::OK);

        assert_status(
            s3_app
                .clone()
                .oneshot(
                    signed_s3_request(
                        Request::builder()
                            .method(Method::PUT)
                            .uri("/test-bucket/folder/hello.txt")
                            .header(header::CONTENT_TYPE, "text/plain")
                            .body(Body::from("hello world")),
                        b"hello world",
                    )
                    .expect("valid request"),
                )
                .await
                .expect("router response"),
            StatusCode::OK,
        );

        let buckets: i64 =
            sqlx_core::query_scalar::query_scalar("SELECT COUNT(*)::bigint FROM buckets")
                .fetch_one(ctx.catalog.pool())
                .await
                .expect("count buckets");
        let objects: i64 =
            sqlx_core::query_scalar::query_scalar("SELECT COUNT(*)::bigint FROM objects")
                .fetch_one(ctx.catalog.pool())
                .await
                .expect("count objects");
        let versions: i64 =
            sqlx_core::query_scalar::query_scalar("SELECT COUNT(*)::bigint FROM object_versions")
                .fetch_one(ctx.catalog.pool())
                .await
                .expect("count object versions");
        let manifests: i64 =
            sqlx_core::query_scalar::query_scalar("SELECT COUNT(*)::bigint FROM object_manifests")
                .fetch_one(ctx.catalog.pool())
                .await
                .expect("count object manifests");
        let fragments: i64 = sqlx_core::query_scalar::query_scalar(
            "SELECT COUNT(*)::bigint FROM object_manifest_fragments",
        )
        .fetch_one(ctx.catalog.pool())
        .await
        .expect("count object manifest fragments");
        assert_eq!((buckets, objects, versions), (1, 1, 1));
        assert_eq!((manifests, fragments), (1, 1));

        let range_get = s3_app
            .clone()
            .oneshot(
                signed_s3_request(
                    Request::builder()
                        .uri("/test-bucket/folder/hello.txt")
                        .header(header::RANGE, "bytes=0-4")
                        .body(Body::empty()),
                    b"",
                )
                .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(range_get.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(response_text(range_get).await, "hello");

        let manifest = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/pontemesh/objects/test-bucket/manifest/folder/hello.txt")
                    .header(header::AUTHORIZATION, &authorization)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(manifest.status(), StatusCode::OK);
        let manifest_body: serde_json::Value =
            serde_json::from_str(&response_text(manifest).await).expect("manifest JSON");
        assert_eq!(manifest_body["fragmentSizeBytes"], 1024);
        let manifest_id = manifest_body["manifestId"]
            .as_str()
            .expect("manifest id")
            .to_owned();

        let package = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/pontemesh/access-packages")
                    .header(header::AUTHORIZATION, &authorization)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"bucket":"test-bucket","key":"folder/hello.txt","ttlSeconds":120}"#,
                    ))
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(package.status(), StatusCode::CREATED);
        let package_body: serde_json::Value =
            serde_json::from_str(&response_text(package).await).expect("access package JSON");
        assert_eq!(
            package_body["manifestId"].as_str(),
            Some(manifest_id.as_str())
        );
        assert_eq!(
            package_body["manifest"]["manifestId"].as_str(),
            Some(manifest_id.as_str())
        );
        let package_manifest_id: Option<String> = sqlx_core::query_scalar::query_scalar(
            "SELECT object_manifest_id::text FROM access_packages WHERE id = $1::uuid",
        )
        .bind(package_body["id"].as_str().expect("access package id"))
        .fetch_one(ctx.catalog.pool())
        .await
        .expect("access package manifest id");
        assert_eq!(package_manifest_id.as_deref(), Some(manifest_id.as_str()));

        let package_id = package_body["id"].as_str().expect("access package id");
        let package_token = package_body["packageToken"]
            .as_str()
            .expect("access package token");
        assert_eq!(
            package_body["sourceSelection"]["allowPeerSharing"],
            serde_json::json!(false)
        );
        assert_eq!(
            package_body["authorizedSources"]
                .as_array()
                .expect("initial package sources")
                .len(),
            1
        );
        let peer_without_policy = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!(
                        "/pontemesh/access-packages/{package_id}/peers/test-bucket/folder/hello.txt"
                    ))
                    .header(header::AUTHORIZATION, format!("Bearer {package_token}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"peerId":"client-a","endpoint":"https://peer-a.example.test/fragments","availableFragments":[0],"ttlSeconds":120}"#,
                    ))
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(peer_without_policy.status(), StatusCode::BAD_REQUEST);
        let package_object = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/pontemesh/access-packages/{package_id}/objects/test-bucket/folder/hello.txt"
                    ))
                    .header(header::AUTHORIZATION, format!("Bearer {package_token}"))
                    .header(header::RANGE, "bytes=6-10")
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(package_object.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(response_text(package_object).await, "world");

        let invalid_package_object = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/pontemesh/access-packages/{package_id}/objects/test-bucket/folder/hello.txt"
                    ))
                    .header(header::AUTHORIZATION, "Bearer invalid-token")
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(invalid_package_object.status(), StatusCode::UNAUTHORIZED);

        let metrics = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/admin/metrics/origin-traffic")
                    .header(header::COOKIE, &admin_cookie)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(metrics.status(), StatusCode::OK);
        let metrics_body: serde_json::Value =
            serde_json::from_str(&response_text(metrics).await).expect("metrics JSON");
        assert_eq!(metrics_body["totalRequests"], 2);
        assert_eq!(metrics_body["rangeRequests"], 2);
        assert_eq!(metrics_body["totalBytesServed"], 10);

        let create_replica = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/admin/replicas")
                    .header(header::COOKIE, &admin_cookie)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"name":"edge-1","allowedBuckets":["test-bucket"]}"#,
                    ))
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(create_replica.status(), StatusCode::CREATED);
        let replica_body: serde_json::Value =
            serde_json::from_str(&response_text(create_replica).await).expect("replica JSON");
        let replica_id = replica_body["replica"]["id"].as_str().expect("replica id");
        let replica_token = replica_body["token"].as_str().expect("replica token");

        let sync_plan = app
            .clone()
            .oneshot(
                signed_replica_request(
                    Request::builder()
                        .uri(format!("/pontemesh/replicas/{replica_id}/sync-plan"))
                        .body(Body::empty()),
                    replica_token,
                    "nonce-sync-plan-0001",
                )
                .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(sync_plan.status(), StatusCode::OK);
        let sync_plan_body: serde_json::Value =
            serde_json::from_str(&response_text(sync_plan).await).expect("sync plan JSON");
        assert_eq!(sync_plan_body["objects"][0]["key"], "folder/hello.txt");
        assert_eq!(
            sync_plan_body["objects"][0]["manifestId"].as_str(),
            Some(manifest_id.as_str())
        );
        assert_eq!(
            sync_plan_body["objects"][0]["fragments"][0]["fragmentId"].as_str(),
            manifest_body["fragments"][0]["fragmentId"].as_str()
        );

        let announce_availability = app
            .clone()
            .oneshot(
                signed_replica_request(
                    Request::builder()
                        .method(Method::POST)
                        .uri(format!("/pontemesh/replicas/{replica_id}/availability"))
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Body::from(
                            r#"{"bucket":"test-bucket","key":"folder/hello.txt","endpoint":"https://edge-1.example.test/test-bucket/folder/hello.txt","availableFragments":[0]}"#,
                        )),
                    replica_token,
                    "nonce-availability-0001",
                )
                .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(announce_availability.status(), StatusCode::OK);
        let availability_body: serde_json::Value =
            serde_json::from_str(&response_text(announce_availability).await)
                .expect("availability JSON");
        assert_eq!(availability_body["replicaId"].as_str(), Some(replica_id));
        assert_eq!(
            availability_body["endpoint"].as_str(),
            Some("https://edge-1.example.test/test-bucket/folder/hello.txt")
        );

        let listed_replicas = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/admin/replicas")
                    .header(header::COOKIE, &admin_cookie)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(listed_replicas.status(), StatusCode::OK);
        let listed_replicas_body: serde_json::Value =
            serde_json::from_str(&response_text(listed_replicas).await).expect("replicas JSON");
        assert_eq!(listed_replicas_body[0]["availableObjects"], 1);
        assert!(listed_replicas_body[0]["lastSeenAt"].as_str().is_some());

        let replica_health = app
            .clone()
            .oneshot(
                signed_replica_request(
                    Request::builder()
                        .method(Method::POST)
                        .uri(format!("/pontemesh/replicas/{replica_id}/health"))
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Body::from(
                            r#"{"status":"OK","version":"0.1.0","storageAvailableBytes":4096,"errorCount":0,"detail":{"node":"edge-1"}}"#,
                        )),
                    replica_token,
                    "nonce-health-0001",
                )
                .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(replica_health.status(), StatusCode::OK);
        let replica_health_body: serde_json::Value =
            serde_json::from_str(&response_text(replica_health).await).expect("health JSON");
        assert_eq!(replica_health_body["status"], "OK");

        let replica_metric_report = app
            .clone()
            .oneshot(
                signed_replica_request(
                    Request::builder()
                        .method(Method::POST)
                        .uri(format!("/pontemesh/replicas/{replica_id}/metrics"))
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Body::from(
                            r#"{"bytesSynced":3,"bytesServed":7,"fragmentsSynced":1,"fragmentsServed":2,"syncFailures":0,"authFailures":0,"avgLatencyMs":12}"#,
                        )),
                    replica_token,
                    "nonce-metrics-0001",
                )
                .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(replica_metric_report.status(), StatusCode::OK);

        let replica_sync_object = app
            .clone()
            .oneshot(
                signed_replica_request(
                    Request::builder()
                        .uri(format!(
                            "/pontemesh/replicas/{replica_id}/objects/test-bucket/folder/hello.txt"
                        ))
                        .header(header::RANGE, "bytes=0-4")
                        .body(Body::empty()),
                    replica_token,
                    "nonce-replica-object-0001",
                )
                .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(replica_sync_object.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(response_text(replica_sync_object).await, "hello");

        let fragment_id = manifest_body["fragments"][0]["fragmentId"]
            .as_str()
            .expect("fragment id");
        let replica_sync_fragment = app
            .clone()
            .oneshot(
                signed_replica_request(
                    Request::builder()
                        .uri(format!(
                            "/pontemesh/replicas/{replica_id}/manifests/{manifest_id}/fragments/{fragment_id}"
                        ))
                        .body(Body::empty()),
                    replica_token,
                    "nonce-replica-fragment-0001",
                )
                .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(replica_sync_fragment.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(response_text(replica_sync_fragment).await, "hello world");

        let replica_metrics = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/admin/metrics/replica-traffic")
                    .header(header::COOKIE, &admin_cookie)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(replica_metrics.status(), StatusCode::OK);
        let replica_metrics_body: serde_json::Value =
            serde_json::from_str(&response_text(replica_metrics).await)
                .expect("replica metrics JSON");
        assert_eq!(replica_metrics_body["activeReplicas"], 1);
        assert_eq!(replica_metrics_body["totalBytesSynced"], 19);
        assert_eq!(replica_metrics_body["totalFragmentsSynced"], 3);

        let replica_detail_metrics = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/admin/metrics/replicas/{replica_id}"))
                    .header(header::COOKIE, &admin_cookie)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(replica_detail_metrics.status(), StatusCode::OK);
        let replica_detail_metrics_body: serde_json::Value =
            serde_json::from_str(&response_text(replica_detail_metrics).await)
                .expect("replica detail metrics JSON");
        assert_eq!(replica_detail_metrics_body["fragmentEvents"], 1);

        let bucket_metrics = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/admin/metrics/buckets")
                    .header(header::COOKIE, &admin_cookie)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(bucket_metrics.status(), StatusCode::OK);
        assert!(
            response_text(bucket_metrics)
                .await
                .contains(r#""bucket":"test-bucket""#)
        );

        let object_metrics = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/admin/metrics/objects")
                    .header(header::COOKIE, &admin_cookie)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(object_metrics.status(), StatusCode::OK);
        assert!(
            response_text(object_metrics)
                .await
                .contains(r#""key":"folder/hello.txt""#)
        );

        let listed_replicas_after_health = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/admin/replicas")
                    .header(header::COOKIE, &admin_cookie)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(listed_replicas_after_health.status(), StatusCode::OK);
        let listed_replicas_after_health_body: serde_json::Value =
            serde_json::from_str(&response_text(listed_replicas_after_health).await)
                .expect("replicas after health JSON");
        assert_eq!(listed_replicas_after_health_body[0]["healthStatus"], "OK");
        assert!(
            listed_replicas_after_health_body[0]["healthReportedAt"]
                .as_str()
                .is_some()
        );

        let sources = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/pontemesh/objects/test-bucket/sources/folder/hello.txt")
                    .header(header::AUTHORIZATION, &authorization)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(sources.status(), StatusCode::OK);
        let sources_body: serde_json::Value =
            serde_json::from_str(&response_text(sources).await).expect("sources JSON");
        let authorized_sources = sources_body["authorizedSources"]
            .as_array()
            .expect("authorized sources");
        assert_eq!(authorized_sources.len(), 2);
        assert_eq!(authorized_sources[0]["sourceType"], "ORIGIN");
        assert_eq!(authorized_sources[1]["sourceType"], "REPLICA_EDGE");
        assert_eq!(authorized_sources[1]["id"].as_str(), Some(replica_id));

        let availability = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/pontemesh/objects/test-bucket/availability/folder/hello.txt")
                    .header(header::AUTHORIZATION, &authorization)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(availability.status(), StatusCode::OK);
        let availability_body: serde_json::Value =
            serde_json::from_str(&response_text(availability).await).expect("availability JSON");
        assert_eq!(
            availability_body["manifestId"].as_str(),
            Some(manifest_id.as_str())
        );
        assert_eq!(availability_body["objectState"], "AVAILABLE");
        assert_eq!(availability_body["originAvailable"], true);
        assert_eq!(availability_body["replicaSources"], 1);
        assert_eq!(availability_body["peerSources"], 0);
        assert_eq!(
            availability_body["fragments"][0]["replicaSourceIds"][0].as_str(),
            Some(replica_id)
        );
        assert!(
            availability_body["fragments"][0]["availableSourceTypes"]
                .as_array()
                .expect("available source types")
                .iter()
                .any(|source_type| source_type == "REPLICA_EDGE")
        );

        let object_policy = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/pontemesh/objects/test-bucket/policies/folder/hello.txt")
                    .header(header::AUTHORIZATION, &authorization)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(object_policy.status(), StatusCode::OK);
        let object_policy_body: serde_json::Value =
            serde_json::from_str(&response_text(object_policy).await).expect("policy JSON");
        assert_eq!(
            object_policy_body["manifestId"].as_str(),
            Some(manifest_id.as_str())
        );
        assert_eq!(object_policy_body["objectState"], "AVAILABLE");
        assert_eq!(object_policy_body["fragmentSizeBytes"], 1024);
        assert_eq!(object_policy_body["allowReplicaEdge"], true);
        assert_eq!(object_policy_body["fallbackMode"], "ORIGIN_RANGE");
        assert_eq!(object_policy_body["fallbackSupportsRange"], true);
        assert_eq!(object_policy_body["preserveValidatedFragments"], true);

        let package_with_replica = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/pontemesh/access-packages")
                    .header(header::AUTHORIZATION, &authorization)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"bucket":"test-bucket","key":"folder/hello.txt","ttlSeconds":120}"#,
                    ))
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(package_with_replica.status(), StatusCode::CREATED);
        let package_with_replica_body: serde_json::Value =
            serde_json::from_str(&response_text(package_with_replica).await)
                .expect("access package with replica JSON");
        assert_eq!(
            package_with_replica_body["authorizedSources"]
                .as_array()
                .expect("package sources")
                .len(),
            2
        );
        let package_with_replica_id = package_with_replica_body["id"]
            .as_str()
            .expect("package with replica id");
        let package_with_replica_token = package_with_replica_body["packageToken"]
            .as_str()
            .expect("package with replica token");

        let enable_peer_policy = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::PUT)
                    .uri("/api/admin/buckets/test-bucket/policy")
                    .header(header::COOKIE, &admin_cookie)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"accessPackageTtlSeconds":120,"fragmentSizeBytes":1024,"allowReplicaEdge":true,"allowPeerSharing":true,"sourceSelectionStrategy":"PEER_FIRST","fragmentPriorityStrategy":"INITIAL_FIRST","failureThreshold":2,"fallbackMode":"ORIGIN_RANGE"}"#,
                    ))
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(enable_peer_policy.status(), StatusCode::OK);
        let enable_peer_policy_body: serde_json::Value =
            serde_json::from_str(&response_text(enable_peer_policy).await)
                .expect("peer policy JSON");
        assert_eq!(enable_peer_policy_body["allowPeerSharing"], true);
        assert_eq!(
            enable_peer_policy_body["sourceSelectionStrategy"],
            "PEER_FIRST"
        );

        let peer_availability = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!(
                        "/pontemesh/access-packages/{package_with_replica_id}/peers/test-bucket/folder/hello.txt"
                    ))
                    .header(
                        header::AUTHORIZATION,
                        format!("Bearer {package_with_replica_token}"),
                    )
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"peerId":"client-a","endpoint":"https://peer-a.example.test/fragments","availableFragments":[0],"ttlSeconds":120}"#,
                    ))
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(peer_availability.status(), StatusCode::CREATED);
        let peer_availability_body: serde_json::Value =
            serde_json::from_str(&response_text(peer_availability).await)
                .expect("peer availability JSON");
        let peer_availability_id = peer_availability_body["id"]
            .as_str()
            .expect("peer availability id");
        assert_eq!(peer_availability_body["peerId"], "client-a");

        let sources_with_peer = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/pontemesh/objects/test-bucket/sources/folder/hello.txt")
                    .header(header::AUTHORIZATION, &authorization)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(sources_with_peer.status(), StatusCode::OK);
        let sources_with_peer_body: serde_json::Value =
            serde_json::from_str(&response_text(sources_with_peer).await)
                .expect("sources with peer JSON");
        let sources_with_peer_list = sources_with_peer_body["authorizedSources"]
            .as_array()
            .expect("sources with peer");
        assert_eq!(sources_with_peer_list[0]["sourceType"], "PEER");
        assert_eq!(
            sources_with_peer_body["sourceSelection"]["strategy"],
            "PEER_FIRST"
        );

        let availability_with_peer = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/pontemesh/objects/test-bucket/availability/folder/hello.txt")
                    .header(header::AUTHORIZATION, &authorization)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(availability_with_peer.status(), StatusCode::OK);
        let availability_with_peer_body: serde_json::Value =
            serde_json::from_str(&response_text(availability_with_peer).await)
                .expect("availability with peer JSON");
        assert_eq!(availability_with_peer_body["peerSources"], 1);
        assert_eq!(
            availability_with_peer_body["fragments"][0]["peerSourceIds"][0].as_str(),
            Some(peer_availability_id)
        );
        assert!(
            availability_with_peer_body["fragments"][0]["availableSourceTypes"]
                .as_array()
                .expect("available source types with peer")
                .iter()
                .any(|source_type| source_type == "PEER")
        );

        let valid_fragment_hash = manifest_body["fragments"][0]["sha256"]
            .as_str()
            .expect("fragment hash");
        let peer_fragment_event = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!(
                        "/pontemesh/access-packages/{package_with_replica_id}/events/test-bucket/folder/hello.txt"
                    ))
                    .header(
                        header::AUTHORIZATION,
                        format!("Bearer {package_with_replica_token}"),
                    )
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(format!(
                        r#"{{"sourceType":"PEER","peerAvailabilityId":"{peer_availability_id}","fragmentIndex":0,"fragmentHash":"{valid_fragment_hash}","eventType":"FRAGMENT_VALIDATED","bytesTransferred":11,"outcome":"SUCCESS","latencyMs":9,"detail":{{"sessionId":"sdk-session-1"}}}}"#
                    )))
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(peer_fragment_event.status(), StatusCode::CREATED);

        let fallback_event = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!(
                        "/pontemesh/access-packages/{package_with_replica_id}/events/test-bucket/folder/hello.txt"
                    ))
                    .header(
                        header::AUTHORIZATION,
                        format!("Bearer {package_with_replica_token}"),
                    )
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(format!(
                        r#"{{"sourceType":"REPLICA_EDGE","fragmentIndex":0,"fragmentHash":"{valid_fragment_hash}","eventType":"FALLBACK_DECISION","bytesTransferred":0,"outcome":"SUCCESS","latencyMs":12,"detail":{{"from":"REPLICA_EDGE","to":"ORIGIN","preservedFragments":[0]}}}}"#
                    )))
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(fallback_event.status(), StatusCode::CREATED);

        let invalid_hash_event = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!(
                        "/pontemesh/access-packages/{package_with_replica_id}/events/test-bucket/folder/hello.txt"
                    ))
                    .header(
                        header::AUTHORIZATION,
                        format!("Bearer {package_with_replica_token}"),
                    )
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(format!(
                        r#"{{"sourceType":"PEER","peerAvailabilityId":"{peer_availability_id}","fragmentIndex":0,"fragmentHash":"0000000000000000000000000000000000000000000000000000000000000000","eventType":"FRAGMENT_VALIDATED","bytesTransferred":11,"outcome":"SUCCESS","latencyMs":9,"detail":{{"sessionId":"sdk-session-1"}}}}"#
                    )))
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(invalid_hash_event.status(), StatusCode::BAD_REQUEST);

        sqlx_core::query::query(
            "UPDATE peer_fragment_availability SET expires_at = now() - interval '1 second'",
        )
        .execute(ctx.catalog.pool())
        .await
        .expect("expire peer availability");
        let sources_after_peer_expiry = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/pontemesh/objects/test-bucket/sources/folder/hello.txt")
                    .header(header::AUTHORIZATION, &authorization)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(sources_after_peer_expiry.status(), StatusCode::OK);
        let sources_after_peer_expiry_body: serde_json::Value =
            serde_json::from_str(&response_text(sources_after_peer_expiry).await)
                .expect("sources after peer expiry JSON");
        assert!(
            sources_after_peer_expiry_body["authorizedSources"]
                .as_array()
                .expect("sources after peer expiry")
                .iter()
                .all(|source| source["sourceType"] != "PEER")
        );

        let sdk_bucket_metrics = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/admin/metrics/buckets")
                    .header(header::COOKIE, &admin_cookie)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(sdk_bucket_metrics.status(), StatusCode::OK);
        let sdk_bucket_metrics_body: serde_json::Value =
            serde_json::from_str(&response_text(sdk_bucket_metrics).await)
                .expect("SDK bucket metrics JSON");
        assert_eq!(sdk_bucket_metrics_body[0]["peerBytesServed"], 11);
        assert_eq!(sdk_bucket_metrics_body[0]["originOffloadBytes"], 11);
        assert_eq!(sdk_bucket_metrics_body[0]["fallbackEvents"], 1);
        assert_eq!(sdk_bucket_metrics_body[0]["integrityFailures"], 1);

        let sdk_event_counts: (i64, i64) = sqlx_core::query_as::query_as(
            r#"
            SELECT
                COUNT(*) FILTER (WHERE event_type = 'FALLBACK_DECISION')::bigint AS fallback_count,
                COUNT(*) FILTER (WHERE event_type = 'HASH_MISMATCH' AND outcome = 'REJECTED')::bigint AS rejected_hash_count
            FROM fragment_transfer_events
            "#,
        )
        .fetch_one(ctx.catalog.pool())
        .await
        .expect("SDK event counts");
        assert_eq!(sdk_event_counts, (1, 1));

        let revalidate = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!(
                        "/pontemesh/access-packages/{package_with_replica_id}/revalidate/test-bucket/folder/hello.txt"
                    ))
                    .header(
                        header::AUTHORIZATION,
                        format!("Bearer {package_with_replica_token}"),
                    )
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(revalidate.status(), StatusCode::OK);
        let revalidate_body: serde_json::Value =
            serde_json::from_str(&response_text(revalidate).await).expect("revalidate JSON");
        assert_eq!(revalidate_body["valid"], true);
        assert_eq!(
            revalidate_body["authorizedSources"]
                .as_array()
                .expect("revalidated sources")
                .len(),
            2
        );

        let package_to_revoke = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/pontemesh/access-packages")
                    .header(header::AUTHORIZATION, &authorization)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"bucket":"test-bucket","key":"folder/hello.txt","ttlSeconds":120}"#,
                    ))
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(package_to_revoke.status(), StatusCode::CREATED);
        let package_to_revoke_body: serde_json::Value =
            serde_json::from_str(&response_text(package_to_revoke).await)
                .expect("access package to revoke JSON");
        let package_to_revoke_id = package_to_revoke_body["id"]
            .as_str()
            .expect("package to revoke id");
        let package_to_revoke_token = package_to_revoke_body["packageToken"]
            .as_str()
            .expect("package to revoke token");
        let revoke_access_package = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!(
                        "/api/admin/access-packages/{package_to_revoke_id}/revoke"
                    ))
                    .header(header::COOKIE, &admin_cookie)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(revoke_access_package.status(), StatusCode::NO_CONTENT);
        let revalidate_after_package_revoke = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!(
                        "/pontemesh/access-packages/{package_to_revoke_id}/revalidate/test-bucket/folder/hello.txt"
                    ))
                    .header(
                        header::AUTHORIZATION,
                        format!("Bearer {package_to_revoke_token}"),
                    )
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(
            revalidate_after_package_revoke.status(),
            StatusCode::UNAUTHORIZED
        );

        let filtered_audit = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/admin/audit-events?event=access_package_revoked&outcome=success")
                    .header(header::COOKIE, &admin_cookie)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(filtered_audit.status(), StatusCode::OK);
        assert!(
            response_text(filtered_audit)
                .await
                .contains(r#""event":"access_package_revoked""#)
        );

        let application_id: String = sqlx_core::query_scalar::query_scalar(
            "SELECT id::text FROM application_credentials WHERE name = 'test-sdk'",
        )
        .fetch_one(ctx.catalog.pool())
        .await
        .expect("application id");
        let revoke_application = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!(
                        "/api/admin/application-credentials/{application_id}/revoke"
                    ))
                    .header(header::COOKIE, &admin_cookie)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(revoke_application.status(), StatusCode::NO_CONTENT);

        let package_after_application_revoke = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/pontemesh/access-packages")
                    .header(header::AUTHORIZATION, &authorization)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"bucket":"test-bucket","key":"folder/hello.txt","ttlSeconds":120}"#,
                    ))
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(
            package_after_application_revoke.status(),
            StatusCode::UNAUTHORIZED
        );

        let revalidate_after_application_revoke = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!(
                        "/pontemesh/access-packages/{package_with_replica_id}/revalidate/test-bucket/folder/hello.txt"
                    ))
                    .header(
                        header::AUTHORIZATION,
                        format!("Bearer {package_with_replica_token}"),
                    )
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(
            revalidate_after_application_revoke.status(),
            StatusCode::UNAUTHORIZED
        );

        let revoke = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/admin/buckets/test-bucket/object-revocations/folder/hello.txt")
                    .header(header::COOKIE, &admin_cookie)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(revoke.status(), StatusCode::OK);

        let revoked_get = s3_app
            .clone()
            .oneshot(
                signed_s3_request(
                    Request::builder()
                        .uri("/test-bucket/folder/hello.txt")
                        .body(Body::empty()),
                    b"",
                )
                .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(revoked_get.status(), StatusCode::FORBIDDEN);

        let sync_plan_after_revocation = app
            .clone()
            .oneshot(
                signed_replica_request(
                    Request::builder()
                        .uri(format!("/pontemesh/replicas/{replica_id}/sync-plan"))
                        .body(Body::empty()),
                    replica_token,
                    "nonce-sync-plan-0002",
                )
                .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(sync_plan_after_revocation.status(), StatusCode::OK);
        assert!(
            !response_text(sync_plan_after_revocation)
                .await
                .contains(r#""key":"folder/hello.txt""#)
        );

        let policy_updates = app
            .clone()
            .oneshot(
                signed_replica_request(
                    Request::builder()
                        .uri(format!("/pontemesh/replicas/{replica_id}/policy-updates"))
                        .body(Body::empty()),
                    replica_token,
                    "nonce-policy-updates-0001",
                )
                .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(policy_updates.status(), StatusCode::OK);
        assert!(
            response_text(policy_updates)
                .await
                .contains(r#""updateType":"OBJECT_REVOKED""#)
        );

        let revalidate_after_object_revocation = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!(
                        "/pontemesh/access-packages/{package_with_replica_id}/revalidate/test-bucket/folder/hello.txt"
                    ))
                    .header(
                        header::AUTHORIZATION,
                        format!("Bearer {package_with_replica_token}"),
                    )
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(
            revalidate_after_object_revocation.status(),
            StatusCode::UNAUTHORIZED
        );

        let replayed_sync_plan = app
            .clone()
            .oneshot(
                signed_replica_request(
                    Request::builder()
                        .uri(format!("/pontemesh/replicas/{replica_id}/sync-plan"))
                        .body(Body::empty()),
                    replica_token,
                    "nonce-sync-plan-0002",
                )
                .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(replayed_sync_plan.status(), StatusCode::UNAUTHORIZED);

        let revoke_replica = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!("/api/admin/replicas/{replica_id}/revoke"))
                    .header(header::COOKIE, &admin_cookie)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(revoke_replica.status(), StatusCode::OK);

        let sync_plan_after_replica_revoke = app
            .oneshot(
                signed_replica_request(
                    Request::builder()
                        .uri(format!("/pontemesh/replicas/{replica_id}/sync-plan"))
                        .body(Body::empty()),
                    replica_token,
                    "nonce-sync-plan-0003",
                )
                .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(
            sync_plan_after_replica_revoke.status(),
            StatusCode::UNAUTHORIZED
        );
    }

    struct TestContext {
        guard: tokio::sync::MutexGuard<'static, ()>,
        app: Router,
        s3_app: Router,
        catalog: Catalog,
    }

    impl TestContext {
        async fn new(name: &str) -> Option<Self> {
            Self::new_with_role(name, InstanceRole::Origin).await
        }

        async fn new_with_role(name: &str, role: InstanceRole) -> Option<Self> {
            let database_url = match std::env::var("TEST_DATABASE_URL") {
                Ok(value) => value,
                Err(_) => {
                    eprintln!("skipping PostgreSQL integration test; TEST_DATABASE_URL is not set");
                    return None;
                }
            };
            let guard = TEST_DB_LOCK.get_or_init(|| Mutex::new(())).lock().await;
            reset_database(&database_url).await;
            let paths = test_home(name);
            paths.ensure_layout().expect("test home layout");
            write_test_config(&paths, role);
            fs::write(paths.setup_lock_file(), "completed_at = \"test\"\n").expect("setup lock");
            let catalog = Catalog::initialize_with_url(&database_url)
                .await
                .expect("catalog");
            unsafe {
                std::env::remove_var("PONTEMESH_S3_BOOTSTRAP_ACCESS_KEY_ID");
                std::env::remove_var("PONTEMESH_S3_BOOTSTRAP_SECRET_ACCESS_KEY");
            }
            let secret_encryption_key =
                s3_secret_encryption_key(&paths).expect("S3 secret encryption key");
            catalog
                .ensure_s3_access_key(
                    None,
                    Some("test-bootstrap-key"),
                    TEST_S3_ACCESS_KEY,
                    TEST_S3_SECRET_KEY,
                    &secret_encryption_key,
                )
                .await
                .expect("S3 access key");
            catalog
                .create_initial_admin_user(
                    "admin",
                    &hash_admin_password("correct-password").expect("password hash"),
                )
                .await
                .expect("admin user");
            let setup = setup::SetupState::new();
            let app = web_router(paths.clone(), setup.clone(), catalog.clone());
            let s3_app = s3_router(paths, setup, catalog.clone());
            Some(Self {
                guard,
                app,
                s3_app,
                catalog,
            })
        }
    }

    async fn reset_database(database_url: &str) {
        let pool = PgPool::connect(database_url)
            .await
            .expect("connect test database");
        sqlx_core::query::query("DROP SCHEMA public CASCADE")
            .execute(&pool)
            .await
            .expect("drop public schema");
        sqlx_core::query::query("CREATE SCHEMA public")
            .execute(&pool)
            .await
            .expect("create public schema");
        drop(pool);
    }

    async fn create_application(app: Router, catalog: &Catalog) -> (Router, String) {
        let created = catalog
            .create_application_credential(
                "test-sdk",
                vec![
                    "origin:objects:read".to_owned(),
                    "origin:objects:write".to_owned(),
                    "pontemesh:manifest:read".to_owned(),
                    "pontemesh:availability:read".to_owned(),
                    "pontemesh:policies:read".to_owned(),
                    "pontemesh:sources:read".to_owned(),
                    "pontemesh:access-package:create".to_owned(),
                ],
            )
            .await
            .expect("application credential");
        (app, created.token)
    }

    async fn login_cookie(app: Router) -> String {
        let login_response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
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
        login_response
            .headers()
            .get(header::SET_COOKIE)
            .expect("login must set cookie")
            .to_str()
            .expect("cookie must be valid header text")
            .split(';')
            .next()
            .expect("cookie has name/value")
            .to_owned()
    }

    fn assert_status(response: axum::response::Response, status: StatusCode) {
        assert_eq!(response.status(), status);
    }

    fn assert_s3_xml_content_type(response: &axum::response::Response) {
        assert_eq!(
            response
                .headers()
                .get(header::CONTENT_TYPE)
                .expect("S3 XML content type")
                .to_str()
                .expect("content type text"),
            "application/xml"
        );
        assert!(response.headers().contains_key(header::CONTENT_LENGTH));
        assert!(response.headers().contains_key("x-amz-request-id"));
    }

    fn assert_json_content_type(response: &axum::response::Response) {
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .expect("JSON content type")
            .to_str()
            .expect("content type text");
        assert!(
            content_type.starts_with("application/json"),
            "expected JSON content type, got {content_type}"
        );
    }

    fn assert_json_object_keys(value: &serde_json::Value, expected_keys: &[&str]) {
        let object = value.as_object().expect("JSON object");
        let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
        keys.sort_unstable();
        let mut expected = expected_keys.to_vec();
        expected.sort_unstable();
        assert_eq!(keys, expected);
    }

    fn assert_paginated_contract(value: &serde_json::Value) {
        assert_json_object_keys(
            value,
            &["items", "page", "pageSize", "totalItems", "totalPages"],
        );
        assert!(value["items"].is_array());
        assert!(value["page"].is_number());
        assert!(value["pageSize"].is_number());
        assert!(value["totalItems"].is_number());
        assert!(value["totalPages"].is_number());
    }

    async fn mcp_call(
        app: Router,
        token: &str,
        method: &str,
        params: serde_json::Value,
    ) -> serde_json::Value {
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/mcp")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "jsonrpc": "2.0",
                            "id": 1,
                            "method": method,
                            "params": params
                        })
                        .to_string(),
                    ))
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_json_content_type(&response);
        json_body(response).await
    }

    fn header_value<K>(response: &axum::response::Response, name: K) -> &str
    where
        K: axum::http::header::AsHeaderName,
    {
        response
            .headers()
            .get(name)
            .expect("header present")
            .to_str()
            .expect("header text")
    }

    async fn json_body(response: axum::response::Response) -> serde_json::Value {
        serde_json::from_str(&response_text(response).await).expect("JSON response")
    }

    fn xml_value(xml: &str, tag: &str) -> String {
        let open = format!("<{tag}>");
        let close = format!("</{tag}>");
        xml.split_once(&open)
            .and_then(|(_, rest)| rest.split_once(&close).map(|(value, _)| value))
            .unwrap_or_else(|| panic!("missing XML tag {tag}"))
            .to_owned()
    }

    fn signed_s3_request(
        request: Result<Request<Body>, axum::http::Error>,
        payload: &[u8],
    ) -> Result<Request<Body>, axum::http::Error> {
        signed_s3_request_with_credentials(request, payload, TEST_S3_ACCESS_KEY, TEST_S3_SECRET_KEY)
    }

    fn signed_s3_request_with_credentials(
        request: Result<Request<Body>, axum::http::Error>,
        payload: &[u8],
        access_key_id: &str,
        secret_access_key: &str,
    ) -> Result<Request<Body>, axum::http::Error> {
        let now = chrono::Utc::now();
        let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
        let date = now.format("%Y%m%d").to_string();
        signed_s3_request_with_date(
            request,
            payload,
            access_key_id,
            secret_access_key,
            &amz_date,
            &date,
        )
    }

    fn signed_s3_request_with_date(
        request: Result<Request<Body>, axum::http::Error>,
        payload: &[u8],
        access_key_id: &str,
        secret_access_key: &str,
        amz_date: &str,
        date: &str,
    ) -> Result<Request<Body>, axum::http::Error> {
        let mut request = request?;
        let payload_hash = sha256_hex(payload);
        {
            let headers = request.headers_mut();
            headers.insert(header::HOST, "localhost:9000".parse().expect("host header"));
            headers.insert("x-amz-date", amz_date.parse().expect("x-amz-date header"));
            headers.insert(
                "x-amz-content-sha256",
                payload_hash.parse().expect("payload hash header"),
            );
        }

        let mut signed_headers = vec!["host", "x-amz-content-sha256", "x-amz-date"];
        if request.headers().contains_key(header::RANGE) {
            signed_headers.insert(1, "range");
        }
        let signed_headers = signed_headers.join(";");
        let canonical_request = test_canonical_request(&request, &signed_headers, &payload_hash);
        let canonical_hash = sha256_hex(canonical_request.as_bytes());
        let credential_scope = format!("{date}/{TEST_REGION}/s3/aws4_request");
        let string_to_sign =
            format!("AWS4-HMAC-SHA256\n{amz_date}\n{credential_scope}\n{canonical_hash}");
        let signing_key = test_signing_key(secret_access_key, date, TEST_REGION);
        let signature = to_hex(&hmac_bytes(&signing_key, string_to_sign.as_bytes()));
        let authorization = format!(
            "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
            access_key_id, credential_scope, signed_headers, signature
        );
        request.headers_mut().insert(
            header::AUTHORIZATION,
            authorization.parse().expect("authorization header"),
        );
        Ok(request)
    }

    fn presigned_s3_request(path: &str, expires_seconds: i64) -> axum::http::request::Builder {
        let now = chrono::Utc::now();
        presigned_s3_request_with_time(path, expires_seconds, now)
    }

    fn expired_presigned_s3_request(path: &str) -> axum::http::request::Builder {
        presigned_s3_request_with_time(path, 60, chrono::Utc::now() - chrono::Duration::hours(1))
    }

    fn presigned_s3_request_with_time(
        path: &str,
        expires_seconds: i64,
        signing_time: chrono::DateTime<chrono::Utc>,
    ) -> axum::http::request::Builder {
        let amz_date = signing_time.format("%Y%m%dT%H%M%SZ").to_string();
        let date = signing_time.format("%Y%m%d").to_string();
        let credential_scope = format!("{date}/{TEST_REGION}/s3/aws4_request");
        let credential = format!(
            "{}%2F{}",
            TEST_S3_ACCESS_KEY,
            credential_scope.replace('/', "%2F")
        );
        let signed_headers = "host";
        let canonical_query = format!(
            "X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Credential={credential}&X-Amz-Date={amz_date}&X-Amz-Expires={expires_seconds}&X-Amz-SignedHeaders={signed_headers}"
        );
        let uri = format!("{path}?{canonical_query}");
        let canonical_request = format!(
            "GET\n{path}\n{canonical_query}\nhost:localhost:9000\n\n{signed_headers}\nUNSIGNED-PAYLOAD"
        );
        let canonical_hash = sha256_hex(canonical_request.as_bytes());
        let string_to_sign =
            format!("AWS4-HMAC-SHA256\n{amz_date}\n{credential_scope}\n{canonical_hash}");
        let signing_key = test_signing_key(TEST_S3_SECRET_KEY, &date, TEST_REGION);
        let signature = to_hex(&hmac_bytes(&signing_key, string_to_sign.as_bytes()));
        Request::builder()
            .method(Method::GET)
            .uri(format!("{uri}&X-Amz-Signature={signature}"))
            .header(header::HOST, "localhost:9000")
    }

    fn signed_replica_request(
        request: Result<Request<Body>, axum::http::Error>,
        replica_token: &str,
        nonce: &str,
    ) -> Result<Request<Body>, axum::http::Error> {
        let mut request = request?;
        let timestamp = chrono::Utc::now().to_rfc3339();
        let path_and_query = request
            .uri()
            .path_and_query()
            .map(|value| value.as_str())
            .unwrap_or(request.uri().path());
        let signing_payload = format!(
            "{}\n{}\n{}\n{}",
            request.method(),
            path_and_query,
            timestamp,
            nonce
        );
        let signature = to_hex(&hmac_bytes(
            replica_token.as_bytes(),
            signing_payload.as_bytes(),
        ));
        request.headers_mut().insert(
            header::AUTHORIZATION,
            format!("Bearer {replica_token}")
                .parse()
                .expect("authorization header"),
        );
        request.headers_mut().insert(
            "x-pontemesh-date",
            timestamp.parse().expect("replica date header"),
        );
        request.headers_mut().insert(
            "x-pontemesh-nonce",
            nonce.parse().expect("replica nonce header"),
        );
        request.headers_mut().insert(
            "x-pontemesh-signature",
            signature.parse().expect("replica signature header"),
        );
        Ok(request)
    }

    fn test_canonical_request(
        request: &Request<Body>,
        signed_headers: &str,
        payload_hash: &str,
    ) -> String {
        let canonical_query = request.uri().query().unwrap_or("");
        let mut canonical_headers = String::new();
        for header_name in signed_headers.split(';') {
            let value = request
                .headers()
                .get(header_name)
                .expect("signed header present")
                .to_str()
                .expect("signed header text");
            canonical_headers.push_str(header_name);
            canonical_headers.push(':');
            canonical_headers.push_str(value);
            canonical_headers.push('\n');
        }
        format!(
            "{}\n{}\n{}\n{}\n{}\n{}",
            request.method(),
            request.uri().path(),
            canonical_query,
            canonical_headers,
            signed_headers,
            payload_hash
        )
    }

    fn sha256_hex(bytes: &[u8]) -> String {
        format!("{:x}", Sha256::digest(bytes))
    }

    fn test_signing_key(secret: &str, date: &str, region: &str) -> Vec<u8> {
        let date_key = hmac_bytes(format!("AWS4{secret}").as_bytes(), date.as_bytes());
        let date_region_key = hmac_bytes(&date_key, region.as_bytes());
        let date_region_service_key = hmac_bytes(&date_region_key, b"s3");
        hmac_bytes(&date_region_service_key, b"aws4_request")
    }

    fn hmac_bytes(key: &[u8], data: &[u8]) -> Vec<u8> {
        let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts any key length");
        mac.update(data);
        mac.finalize().into_bytes().to_vec()
    }

    fn to_hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    fn multipart_body(boundary: &str, key: &str, bytes: &[u8]) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(b"Content-Disposition: form-data; name=\"key\"\r\n\r\n");
        body.extend_from_slice(key.as_bytes());
        body.extend_from_slice(b"\r\n");
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(
            b"Content-Disposition: form-data; name=\"file\"; filename=\"hello.txt\"\r\n",
        );
        body.extend_from_slice(b"Content-Type: text/plain\r\n\r\n");
        body.extend_from_slice(bytes);
        body.extend_from_slice(b"\r\n");
        body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
        body
    }

    fn test_home(name: &str) -> PontemeshHome {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("pontemesh-{name}-{nanos}"));
        PontemeshHome::from_path(root).expect("test home")
    }

    fn write_test_config(paths: &PontemeshHome, role: InstanceRole) {
        let config = InstanceConfig {
            instance: InstanceSection {
                name: "Test Origin".to_owned(),
                role,
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
            replica: (role == InstanceRole::ReplicaEdge).then(|| crate::config::ReplicaSection {
                origin_base_url: "https://origin.example.com".to_owned(),
                replica_id: "replica-test".to_owned(),
                replica_token: "replica-token".to_owned(),
                public_endpoint: "https://edge.example.com".to_owned(),
                sync_interval_seconds: Some(30),
                health_interval_seconds: Some(30),
            }),
        };
        let raw_config = toml::to_string(&config).expect("serialize config");
        fs::write(paths.config_file(), raw_config).expect("write config");
    }

    async fn response_text(response: axum::response::Response) -> String {
        let bytes = response_bytes(response).await;
        String::from_utf8(bytes.to_vec()).expect("utf8 response")
    }

    async fn response_bytes(response: axum::response::Response) -> axum::body::Bytes {
        to_bytes(response.into_body(), 1024 * 1024)
            .await
            .expect("read response body")
    }
}
