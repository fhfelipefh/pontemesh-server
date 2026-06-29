use crate::{
    admin, auth, catalog::Catalog, config::PontemeshHome, mesh, origin, replica, s3_auth, setup,
    web_assets,
};
use axum::{
    Router, middleware,
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
            put(origin::create_bucket)
                .get(origin::list_objects)
                .head(origin::head_bucket)
                .delete(origin::delete_bucket),
        )
        .route(
            "/{bucket_name}/{*object_key}",
            put(origin::put_object)
                .head(origin::head_object)
                .get(origin::get_object)
                .delete(origin::delete_object),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            s3_auth::require_s3_signature,
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
        .route(
            "/api/admin/metrics/origin-traffic",
            get(admin::origin_traffic_metrics),
        )
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
            get(admin::list_objects).post(admin::upload_object),
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
            auth::require_admin_session,
        ))
}

fn pontemesh_routes(state: AppState) -> Router<AppState> {
    let replica_routes = Router::new()
        .route("/replicas/{replica_id}/sync-plan", get(replica::sync_plan))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_replica_credential,
        ));

    Router::new()
        .route("/access-packages", post(mesh::create_access_package))
        .route(
            "/objects/{bucket_name}/manifest/{*object_key}",
            get(mesh::get_manifest),
        )
        .route_layer(middleware::from_fn_with_state(
            state,
            auth::require_application_credential,
        ))
        .merge(replica_routes)
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
    const TEST_AMZ_DATE: &str = "20260629T120000Z";
    const TEST_DATE: &str = "20260629";
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
        assert_eq!((buckets, objects, versions), (1, 1, 1));

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
                        r#"{"accessPackageTtlSeconds":120,"fragmentSizeBytes":1024,"allowReplicaEdge":false,"allowPeerSharing":false}"#,
                    ))
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(policy_update.status(), StatusCode::OK);

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
        assert!(
            response_text(manifest)
                .await
                .contains(r#""fragmentSizeBytes":1024"#)
        );

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
        assert!(response_text(metrics).await.contains(r#""totalRequests""#));

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
        let replica_authorization = format!("Bearer {replica_token}");

        let sync_plan = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/pontemesh/replicas/{replica_id}/sync-plan"))
                    .header(header::AUTHORIZATION, &replica_authorization)
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(sync_plan.status(), StatusCode::OK);
        assert!(
            response_text(sync_plan)
                .await
                .contains(r#""key":"folder/hello.txt""#)
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
            .oneshot(
                Request::builder()
                    .uri(format!("/pontemesh/replicas/{replica_id}/sync-plan"))
                    .header(header::AUTHORIZATION, &replica_authorization)
                    .body(Body::empty())
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
    }

    struct TestContext {
        guard: tokio::sync::MutexGuard<'static, ()>,
        app: Router,
        s3_app: Router,
        catalog: Catalog,
    }

    impl TestContext {
        async fn new(name: &str) -> Option<Self> {
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
            write_test_config(&paths);
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
        let mut request = request?;
        let payload_hash = sha256_hex(payload);
        {
            let headers = request.headers_mut();
            headers.insert(header::HOST, "localhost:9000".parse().expect("host header"));
            headers.insert(
                "x-amz-date",
                TEST_AMZ_DATE.parse().expect("x-amz-date header"),
            );
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
        let credential_scope = format!("{TEST_DATE}/{TEST_REGION}/s3/aws4_request");
        let string_to_sign =
            format!("AWS4-HMAC-SHA256\n{TEST_AMZ_DATE}\n{credential_scope}\n{canonical_hash}");
        let signing_key = test_signing_key(secret_access_key, TEST_DATE, TEST_REGION);
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
