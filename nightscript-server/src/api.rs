use askama::Template;
use axum::{
    body::Body,
    extract::{Form, Path, Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::Response,
    routing::{delete, get, post},
    Json, Router,
};
use axum_extra::extract::Multipart;
use flate2::read::GzDecoder;
use pulldown_cmark::{html, Options, Parser};
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::{QueryBuilder, Row, Sqlite, SqlitePool, Transaction};
use std::fs::File;
use std::io::{Cursor, Read};
use tar::Archive;
use tokio_util::io::ReaderStream;

use crate::{
    auth::{hash_password, verify_password, AuthExtractor, JwtKeys},
    db::fetch_user_by_username,
    error::AppError,
    models::{
        now_string, AuthResponse, LoginRequest, Manifest, PackageDetail, PackageRow,
        PackageSummary, PackageVersion, PublishResponse, RegisterRequest, TargetsSection,
        UserResponse, VersionRow,
    },
    storage::Storage,
    templates::{
        ErrorTemplate, IndexTemplate, LoginTemplate, OwnerPackageView, OwnerTemplate,
        PackageDetailView, PackageListItem, PackageTemplate, PackageVersionView, PackagesTemplate,
        PaginationContext,
    },
};

const MAX_TARBALL_BYTES: usize = 50 * 1024 * 1024;
const MAX_MANIFEST_BYTES: usize = 256 * 1024;
const README_LIMIT_BYTES: usize = 256 * 1024;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub storage: Storage,
    pub jwt: JwtKeys,
}

#[derive(Default, serde::Deserialize)]
pub struct PackagesQuery {
    pub search: Option<String>,
    #[serde(rename = "q")]
    pub q: Option<String>,
    pub sort: Option<String>,
    pub page: Option<usize>,
    pub per_page: Option<usize>,
}

#[derive(serde::Deserialize)]
pub struct LoginFormData {
    pub username: String,
    pub password: String,
}

struct HtmlPage {
    body: String,
    last_modified: Option<String>,
    etag_seed: Option<String>,
    status: StatusCode,
}

#[derive(Debug, sqlx::FromRow)]
struct PackageListRow {
    id: i64,
    name: String,
    description: Option<String>,
    owner_name: String,
    latest_version: Option<String>,
    updated_at: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct OwnerPackageRow {
    name: String,
    description: Option<String>,
    latest_version: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct PackageVersionRow {
    version: String,
    checksum: String,
    created_at: String,
    yanked: i64,
    targets_json: Option<String>,
    #[sqlx(rename = "path?")]
    path: Option<String>,
}

#[derive(Serialize)]
struct PackageVersionsResponse {
    name: String,
    versions: Vec<PackageVersionDescriptor>,
}

#[derive(Serialize)]
struct PackageVersionDescriptor {
    version: String,
    checksum: String,
    created_at: String,
    yanked: bool,
    dependencies: std::collections::BTreeMap<String, String>,
    targets: TargetsSection,
}

#[derive(Serialize)]
struct OwnerListResponse {
    package: String,
    owners: Vec<String>,
}

#[derive(Deserialize)]
struct OwnerRequest {
    username: String,
}

impl HtmlPage {
    fn new(body: String) -> Self {
        Self {
            body,
            last_modified: None,
            etag_seed: None,
            status: StatusCode::OK,
        }
    }

    fn last_modified(mut self, value: Option<String>) -> Self {
        self.last_modified = value;
        self
    }

    fn etag_seed(mut self, seed: Option<String>) -> Self {
        self.etag_seed = seed;
        self
    }

    fn status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }

    fn into_response(self) -> Response {
        let mut response = Response::new(self.body.into());
        *response.status_mut() = self.status;
        if let Some(last_modified) = self.last_modified {
            if let Ok(value) = HeaderValue::from_str(&last_modified) {
                response.headers_mut().insert(header::LAST_MODIFIED, value);
            }
        }
        if let Some(seed) = self.etag_seed {
            let tag = Sha256::digest(seed.as_bytes());
            if let Ok(value) = HeaderValue::from_str(&format!("W/\"{:x}\"", tag)) {
                response.headers_mut().insert(header::ETAG, value);
            }
        }
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/html; charset=utf-8"),
        );
        response
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(index_html))
        .route("/packages", get(packages_html))
        .route("/package/:name", get(package_html))
        .route("/owner/:handle", get(owner_html))
        .route("/login", get(login_page).post(login_post))
        .route("/api/v1/register", post(register))
        .route("/api/v1/login", post(login))
        .route("/api/v1/me", get(me))
        .route("/api/v1/packages", get(list_packages))
        .route("/api/v1/package/:name", get(get_package))
        .route(
            "/api/v1/package/:name/versions",
            get(list_versions),
        )
        .route("/api/package/:name", get(get_package))
        .route(
            "/api/v1/package/:name/:version/download",
            get(download_package),
        )
        .route(
            "/api/v1/package/:name/:version/yank",
            post(yank_version),
        )
        .route(
            "/api/v1/package/:name/:version/unyank",
            post(unyank_version),
        )
        .route(
            "/api/v1/package/:name/owners",
            get(list_owners).post(add_owner),
        )
        .route(
            "/api/v1/package/:name/owners/:owner",
            delete(remove_owner),
        )
        .route("/api/v1/packages/publish", post(publish))
        .with_state(state)
}

async fn index_html(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let signed = identify_user(&headers, &state);
    let result = IndexTemplate {
        signed_in: signed.clone(),
    }
    .render()
    .map(HtmlPage::new)
    .map_err(|e| AppError::bad_request(e.to_string()));
    respond_html(result, signed)
}

async fn packages_html(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<PackagesQuery>,
) -> Response {
    let signed = identify_user(&headers, &state);
    let result = render_packages_view(&state, query, signed.clone()).await;
    respond_html(result, signed)
}

async fn package_html(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Response {
    let signed = identify_user(&headers, &state);
    let result = render_package_detail(&state, name, signed.clone()).await;
    respond_html(result, signed)
}

async fn render_package_detail(
    state: &AppState,
    name: String,
    signed_in: Option<String>,
) -> Result<HtmlPage, AppError> {
    let pkg = sqlx::query_as::<_, crate::models::PackageRow>(
        r#"
        SELECT p.id, p.name, p.description, p.metadata_json, p.owner_id, u.username as owner_name
        FROM packages p
        JOIN users u ON p.owner_id = u.id
        WHERE p.name = ?
        "#,
    )
    .bind(&name)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::not_found("package not found"))?;

    let version_rows = sqlx::query_as::<_, PackageVersionRow>(
        r#"
        SELECT v.version, v.checksum, v.created_at, v.yanked, v.targets_json, a.path as "path?"
        FROM versions v
        LEFT JOIN assets a ON a.version_id = v.id
        WHERE v.package_id = ?
        ORDER BY v.created_at DESC
        "#,
    )
    .bind(pkg.id)
    .fetch_all(&state.pool)
    .await?;

    let readme_html = version_rows
        .iter()
        .find_map(|row| row.path.as_ref())
        .map(|path| readme_html_from_tar(path))
        .transpose()?
        .flatten();

    let mut latest_targets = TargetsSection::default();
    let versions = version_rows
        .iter()
        .enumerate()
        .map(|(idx, row)| {
            if idx == 0 {
                latest_targets = targets_from_opt(row.targets_json.as_deref());
            }
            PackageVersionView {
                version: row.version.clone(),
                checksum: row.checksum.clone(),
                created_at: row.created_at.clone(),
                yanked: row.yanked != 0,
            }
        })
        .collect::<Vec<_>>();

    let latest_version = versions.first().map(|v| v.version.clone());
    let package_view = PackageDetailView {
        name: pkg.name.clone(),
        description: pkg.description.clone(),
        owner: pkg.owner_name.clone(),
        latest_version: latest_version.clone(),
        install_add: format!("apexrc add {}", pkg.name),
        install_install: "apexrc install".to_string(),
        targets: describe_targets(&latest_targets),
    };

    let template = PackageTemplate {
        signed_in,
        package: package_view,
        versions,
        readme_html,
    };
    let body = template
        .render()
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    let last_modified = version_rows.first().map(|row| row.created_at.clone());
    let etag_seed = format!(
        "package:{}:{}",
        pkg.name,
        last_modified.clone().unwrap_or_default()
    );
    Ok(HtmlPage::new(body)
        .last_modified(last_modified)
        .etag_seed(Some(etag_seed)))
}

async fn owner_html(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(handle): Path<String>,
) -> Response {
    let signed = identify_user(&headers, &state);
    let result = render_owner_page(&state, handle, signed.clone()).await;
    respond_html(result, signed)
}

async fn render_owner_page(
    state: &AppState,
    handle: String,
    signed_in: Option<String>,
) -> Result<HtmlPage, AppError> {
    let owner = sqlx::query("SELECT id, username FROM users WHERE username = ?")
        .bind(&handle)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::not_found("owner not found"))?;
    let owner_id: i64 = owner.get("id");
    let owner_name: String = owner.get("username");
    let packages = sqlx::query_as::<_, OwnerPackageRow>(
        r#"
        SELECT p.name,
               p.description,
               (SELECT version FROM versions v WHERE v.package_id = p.id ORDER BY v.created_at DESC LIMIT 1) as latest_version
        FROM packages p
        WHERE p.owner_id = ?
        ORDER BY p.name COLLATE NOCASE
        "#,
    )
    .bind(owner_id)
    .fetch_all(&state.pool)
    .await?;
    let package_views = packages
        .into_iter()
        .map(|row| OwnerPackageView {
            name: row.name,
            description: row.description,
            latest_version: row.latest_version,
        })
        .collect::<Vec<_>>();
    let template = OwnerTemplate {
        signed_in,
        owner_name,
        packages: package_views,
    };
    let body = template
        .render()
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(HtmlPage::new(body))
}

async fn login_page(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let signed = identify_user(&headers, &state);
    let template = LoginTemplate {
        signed_in: signed.clone(),
        token: None,
        error: None,
    };
    let result = template
        .render()
        .map(HtmlPage::new)
        .map_err(|e| AppError::bad_request(e.to_string()));
    respond_html(result, signed)
}

async fn login_post(
    State(state): State<AppState>,
    headers: HeaderMap,
    Form(form): Form<LoginFormData>,
) -> Response {
    let signed = identify_user(&headers, &state);
    match issue_auth_token(&state, &form.username, &form.password).await {
        Ok(auth) => {
            let template = LoginTemplate {
                signed_in: Some(auth.username.clone()),
                token: Some(auth.token.clone()),
                error: None,
            };
            let body = match template.render() {
                Ok(body) => body,
                Err(err) => {
                    return html_error_response(AppError::bad_request(err.to_string()), signed)
                }
            };
            let mut response = HtmlPage::new(body).into_response();
            let cookie = format!("apex_token={}; HttpOnly; Path=/; Max-Age=86400", auth.token);
            response
                .headers_mut()
                .insert(header::SET_COOKIE, cookie.parse().unwrap());
            response
        }
        Err(err) => {
            if matches!(
                err,
                AppError::Http {
                    status: StatusCode::UNAUTHORIZED,
                    ..
                }
            ) {
                let template = LoginTemplate {
                    signed_in: signed.clone(),
                    token: None,
                    error: Some("Invalid credentials".into()),
                };
                let result = template
                    .render()
                    .map(HtmlPage::new)
                    .map_err(|e| AppError::bad_request(e.to_string()));
                respond_html(result, signed)
            } else {
                html_error_response(err, signed)
            }
        }
    }
}

async fn render_packages_view(
    state: &AppState,
    query: PackagesQuery,
    signed_in: Option<String>,
) -> Result<HtmlPage, AppError> {
    let per_page = query.per_page.unwrap_or(20).clamp(5, 100);
    let page = query.page.unwrap_or(1).max(1);
    let offset = (page - 1) * per_page;
    let search_term = query
        .q
        .or(query.search)
        .as_ref()
        .and_then(|s| {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        });
    let sort = match query.sort.as_deref() {
        Some("name") => "name",
        Some("updated") => "updated",
        _ => "updated",
    };

    let mut list_builder = QueryBuilder::<Sqlite>::new(
        r#"
        SELECT p.id,
               p.name,
               p.description,
               u.username as owner_name,
               (SELECT version FROM versions v WHERE v.package_id = p.id AND v.yanked = 0 ORDER BY v.created_at DESC LIMIT 1) as latest_version,
               (SELECT created_at FROM versions v WHERE v.package_id = p.id AND v.yanked = 0 ORDER BY v.created_at DESC LIMIT 1) as updated_at
        FROM packages p
        JOIN users u ON p.owner_id = u.id
        "#,
    );
    apply_search_filter(&mut list_builder, search_term.as_ref());
    match sort {
        "name" => list_builder.push(" ORDER BY p.name COLLATE NOCASE ASC "),
        _ => list_builder.push(
            " ORDER BY (updated_at IS NULL) ASC, updated_at DESC, p.name COLLATE NOCASE ASC ",
        ),
    };
    list_builder
        .push(" LIMIT ")
        .push_bind(per_page as i64)
        .push(" OFFSET ")
        .push_bind(offset as i64);
    let packages = list_builder
        .build_query_as::<PackageListRow>()
        .fetch_all(&state.pool)
        .await?;

    let mut count_builder =
        QueryBuilder::<Sqlite>::new("SELECT COUNT(*) as count FROM packages p ");
    apply_search_filter(&mut count_builder, search_term.as_ref());
    let total_count: i64 = count_builder
        .build_query_scalar()
        .fetch_one(&state.pool)
        .await?;
    let total_pages = std::cmp::max(
        1,
        ((total_count as f64) / (per_page as f64)).ceil() as usize,
    );

    let view_packages = packages
        .iter()
        .map(|row| PackageListItem {
            name: row.name.clone(),
            description: row.description.clone(),
            owner: row.owner_name.clone(),
            latest_version: row.latest_version.clone(),
            updated_at: row.updated_at.clone(),
        })
        .collect::<Vec<_>>();

    let pagination = PaginationContext {
        page,
        total_pages,
        has_prev: page > 1,
        has_next: page < total_pages,
    };

    let template = PackagesTemplate {
        signed_in,
        packages: view_packages,
        pagination,
        search_value: search_term.clone().unwrap_or_default(),
        search_param: search_term.clone(),
        sort: sort.to_string(),
        per_page,
    };
    let body = template
        .render()
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    let last_modified = packages.iter().filter_map(|p| p.updated_at.clone()).max();
    let etag_seed = format!("packages:{}:{}:{}:{:?}", page, per_page, sort, search_term);
    Ok(HtmlPage::new(body)
        .last_modified(last_modified)
        .etag_seed(Some(etag_seed)))
}

async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<UserResponse>, AppError> {
    let hash = hash_password(&payload.password)?;
    let created_at = now_string();
    let rec_id: i64 = sqlx::query_scalar(
        "INSERT INTO users(username, email, pwd_hash, created_at) VALUES(?,?,?,?) RETURNING id",
    )
    .bind(&payload.username)
    .bind(&payload.email)
    .bind(&hash)
    .bind(&created_at)
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(UserResponse {
        id: rec_id,
        username: payload.username,
        email: payload.email,
        created_at,
    }))
}

async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let auth = issue_auth_token(&state, &payload.username, &payload.password).await?;
    Ok(Json(auth))
}

async fn me(
    State(state): State<AppState>,
    AuthExtractor(user): AuthExtractor,
) -> Result<Json<UserResponse>, AppError> {
    let row = fetch_user_by_username(&state.pool, &user.username)
        .await?
        .ok_or_else(|| AppError::unauthorized("unknown user"))?;
    Ok(Json(UserResponse {
        id: row.id,
        username: row.username,
        email: row.email,
        created_at: row.created_at,
    }))
}

async fn issue_auth_token(
    state: &AppState,
    username: &str,
    password: &str,
) -> Result<AuthResponse, AppError> {
    let user = fetch_user_by_username(&state.pool, username)
        .await?
        .ok_or_else(|| AppError::unauthorized("invalid credentials"))?;
    verify_password(&user.pwd_hash, password)?;
    let token = state.jwt.token(user.id, &user.username)?;
    Ok(AuthResponse {
        token,
        username: user.username,
    })
}

async fn list_packages(
    State(state): State<AppState>,
) -> Result<Json<Vec<PackageSummary>>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT p.name,
               p.description,
               (SELECT version FROM versions v WHERE v.package_id = p.id ORDER BY v.created_at DESC LIMIT 1) as latest_version
        FROM packages p
        ORDER BY p.name
        "#
    )
    .fetch_all(&state.pool)
    .await?;
    let summaries = rows
        .into_iter()
        .map(|row| PackageSummary {
            name: row.get::<String, _>("name"),
            description: row.get::<Option<String>, _>("description"),
            latest_version: row.get::<Option<String>, _>("latest_version"),
        })
        .collect();
    Ok(Json(summaries))
}

async fn get_package(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<PackageDetail>, AppError> {
    let pkg = sqlx::query_as::<_, PackageRow>(
        r#"
        SELECT p.id, p.name, p.description, p.metadata_json, p.owner_id, u.username as owner_name
        FROM packages p
        JOIN users u ON p.owner_id = u.id
        WHERE p.name = ?
        "#,
    )
    .bind(&name)
    .fetch_one(&state.pool)
    .await?;
    let versions: Vec<PackageVersion> = sqlx::query_as::<_, VersionRow>(
        "SELECT id, version, checksum, created_at, targets_json FROM versions WHERE package_id = ? ORDER BY created_at DESC",
    )
    .bind(pkg.id)
    .fetch_all(&state.pool)
    .await?
    .into_iter()
    .map(|v| {
        let targets = targets_from_opt(v.targets_json.as_deref());
        PackageVersion {
            version: v.version,
            checksum: v.checksum,
            created_at: v.created_at,
            targets,
        }
    })
    .collect();
    let package_targets = versions
        .first()
        .map(|v| v.targets.clone())
        .unwrap_or_default();
    Ok(Json(PackageDetail {
        name: pkg.name,
        description: pkg.description,
        owner: pkg.owner_name,
        versions,
        targets: package_targets,
    }))
}

async fn list_versions(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<PackageVersionsResponse>, AppError> {
    let pkg = sqlx::query("SELECT id, name FROM packages WHERE name = ?")
        .bind(&name)
        .fetch_optional(&state.pool)
        .await?;
    let pkg = pkg.ok_or_else(|| AppError::not_found("package not found"))?;
    let package_id: i64 = pkg.get("id");
    let versions = sqlx::query(
        r#"
        SELECT version, checksum, created_at, yanked, manifest_json, targets_json
        FROM versions
        WHERE package_id = ?
        ORDER BY created_at DESC
        "#,
    )
    .bind(package_id)
    .fetch_all(&state.pool)
    .await?;
    if versions.is_empty() {
        return Err(AppError::not_found("package has no versions"));
    }
    let mut response_versions = Vec::new();
    for row in versions {
        let manifest_json: String = row.get("manifest_json");
        let targets_text: String = row.get("targets_json");
        let targets = targets_from_opt(Some(targets_text.as_str()));
        let manifest: Manifest =
            serde_json::from_str(&manifest_json).map_err(|e| AppError::bad_request(e.to_string()))?;
        let dependencies = manifest.dependencies;
        response_versions.push(PackageVersionDescriptor {
            version: row.get::<String, _>("version"),
            checksum: row.get::<String, _>("checksum"),
            created_at: row.get::<String, _>("created_at"),
            yanked: row.get::<i64, _>("yanked") != 0,
            dependencies,
            targets,
        });
    }
    Ok(Json(PackageVersionsResponse {
        name,
        versions: response_versions,
    }))
}

async fn download_package(
    State(state): State<AppState>,
    Path((name, version)): Path<(String, String)>,
) -> Result<Response, AppError> {
    let pkg = sqlx::query(
        r#"
        SELECT a.path as "path?", v.checksum
        FROM packages p
        JOIN versions v ON v.package_id = p.id
        JOIN assets a ON a.version_id = v.id
        WHERE p.name = ? AND v.version = ?
        "#,
    )
    .bind(&name)
    .bind(&version)
    .fetch_optional(&state.pool)
    .await?;
    let rec = pkg.ok_or_else(|| AppError::not_found("package not found"))?;
    let path: Option<String> = rec.get("path?");
    let checksum: String = rec.get("checksum");
    let path = path.ok_or_else(|| AppError::not_found("asset missing"))?;
    let file = tokio::fs::File::open(&path).await?;
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);
    let mut response = Response::new(body);
    *response.status_mut() = StatusCode::OK;
    response.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        "application/gzip".parse().unwrap(),
    );
    response
        .headers_mut()
        .insert("X-Checksum", checksum.parse().unwrap());
    Ok(response)
}

async fn publish(
    State(state): State<AppState>,
    AuthExtractor(user): AuthExtractor,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<PublishResponse>, AppError> {
    let checksum_header = headers
        .get("X-Checksum")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::bad_request("missing X-Checksum header"))?;
    let mut manifest_text = None;
    let mut manifest_json_override = None;
    let mut tarball = None;
    while let Some(field) = multipart.next_field().await? {
        let name = field.name().unwrap_or("").to_string();
        let data = field.bytes().await?;
        match name.as_str() {
            "manifest" => {
                if data.len() > MAX_MANIFEST_BYTES {
                    return Err(AppError::payload_too_large(
                        "manifest field exceeds size limit",
                    ));
                }
                manifest_text = Some(
                    String::from_utf8(data.to_vec())
                        .map_err(|_| AppError::bad_request("manifest must be utf8"))?,
                )
            }
            "tarball" => {
                if data.len() > MAX_TARBALL_BYTES {
                    return Err(AppError::payload_too_large("tarball exceeds 50MB limit"));
                }
                tarball = Some(data.to_vec())
            }
            "manifest_json" => {
                if data.len() > MAX_MANIFEST_BYTES {
                    return Err(AppError::payload_too_large(
                        "manifest_json field exceeds size limit",
                    ));
                }
                manifest_json_override = Some(
                    String::from_utf8(data.to_vec())
                        .map_err(|_| AppError::bad_request("manifest_json must be utf8"))?,
                );
            }
            _ => {}
        }
    }
    let manifest_text = manifest_text.ok_or_else(|| AppError::bad_request("manifest missing"))?;
    let tarball = tarball.ok_or_else(|| AppError::bad_request("tarball missing"))?;
    let computed = hex::encode(Sha256::digest(&tarball));
    if computed != checksum_header {
        return Err(AppError::bad_request("checksum mismatch"));
    }
    let manifest_from_toml: Manifest = toml::from_str(&manifest_text)?;
    let manifest = if let Some(ref json_raw) = manifest_json_override {
        serde_json::from_str::<Manifest>(json_raw)
            .map_err(|e| AppError::bad_request(format!("invalid manifest_json: {e}")))?
    } else {
        manifest_from_toml.clone()
    };
    if manifest.package.language.to_lowercase() != "afml" {
        return Err(AppError::bad_request(
            "only AFML packages are supported in phase 1",
        ));
    }
    let version = Version::parse(&manifest.package.version)
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    validate_tarball(&tarball)?;
    let manifest_json_to_store = if let Some(raw) = manifest_json_override {
        raw
    } else {
        serde_json::to_string(&manifest_from_toml)?
    };
    let targets_json = serde_json::to_string(&manifest.targets)?;
    let package_metadata_json = serde_json::to_string(&manifest.package)?;
    let mut tx = state.pool.begin().await?;
    let package_id = ensure_package(
        &mut tx,
        &manifest.package.name,
        &manifest.package.description,
        &package_metadata_json,
        user.id,
    )
    .await?;
    ensure_version_unique(&mut tx, package_id, &version).await?;
    let created_at = now_string();
    let version_rec = sqlx::query(
        "INSERT INTO versions(package_id, version, checksum, manifest_json, targets_json, created_at) VALUES(?,?,?,?,?,?) RETURNING id",
    )
    .bind(package_id)
    .bind(version.to_string())
    .bind(&computed)
    .bind(&manifest_json_to_store)
    .bind(&targets_json)
    .bind(&created_at)
    .fetch_one(&mut *tx)
    .await?;
    let version_id: i64 = version_rec.get("id");
    let asset_path = state
        .storage
        .save_package(&manifest.package.name, &version.to_string(), &tarball)
        .await?;
    sqlx::query("INSERT INTO assets(version_id, filename, path, size) VALUES(?,?,?,?)")
        .bind(version_id)
        .bind(format!("{}.apkg", version))
        .bind(asset_path.to_string_lossy().to_string())
        .bind(tarball.len() as i64)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(PublishResponse {
        name: manifest.package.name,
        version: version.to_string(),
    }))
}

async fn ensure_package(
    tx: &mut Transaction<'_, sqlx::Sqlite>,
    name: &str,
    description: &Option<String>,
    metadata_json: &str,
    owner_id: i64,
) -> Result<i64, AppError> {
    if let Some(existing) = sqlx::query("SELECT id FROM packages WHERE name = ?")
        .bind(name)
        .fetch_optional(&mut **tx)
        .await?
    {
        let package_id: i64 = existing.get("id");
        ensure_owner_membership(tx, package_id, owner_id).await?;
        sqlx::query(
            "UPDATE packages SET description = COALESCE(?, description), metadata_json = ? WHERE id = ?",
        )
        .bind(description)
        .bind(metadata_json)
        .bind(package_id)
        .execute(&mut **tx)
        .await?;
        Ok(package_id)
    } else {
        let created_at = now_string();
        let record = sqlx::query(
            "INSERT INTO packages(name, owner_id, description, metadata_json, created_at) VALUES(?,?,?,?,?) RETURNING id",
        )
        .bind(name)
        .bind(owner_id)
        .bind(description)
        .bind(metadata_json)
        .bind(created_at.clone())
        .fetch_one(&mut **tx)
        .await?;
        let package_id: i64 = record.get("id");
        sqlx::query(
            "INSERT INTO package_owners(package_id, user_id, role, created_at) VALUES(?,?,?,?)",
        )
        .bind(package_id)
        .bind(owner_id)
        .bind("owner")
        .bind(created_at)
        .execute(&mut **tx)
        .await?;
        Ok(package_id)
    }
}

async fn ensure_version_unique(
    tx: &mut Transaction<'_, sqlx::Sqlite>,
    package_id: i64,
    version: &Version,
) -> Result<(), AppError> {
    let exists = sqlx::query("SELECT id FROM versions WHERE package_id = ? AND version = ?")
        .bind(package_id)
        .bind(version.to_string())
        .fetch_optional(&mut **tx)
        .await?;
    if exists.is_some() {
        return Err(AppError::conflict("version already exists"));
    }
    Ok(())
}

async fn ensure_owner_membership(
    tx: &mut Transaction<'_, sqlx::Sqlite>,
    package_id: i64,
    owner_id: i64,
) -> Result<(), AppError> {
    let exists = sqlx::query_scalar::<_, i64>(
        "SELECT 1 FROM package_owners WHERE package_id = ? AND user_id = ?",
    )
    .bind(package_id)
    .bind(owner_id)
    .fetch_optional(&mut **tx)
    .await?;
    if exists.is_some() {
        Ok(())
    } else {
        Err(AppError::conflict("package owned by another user"))
    }
}

fn validate_tarball(data: &[u8]) -> Result<(), AppError> {
    let cursor = Cursor::new(data);
    let decoder = GzDecoder::new(cursor);
    let mut archive = Archive::new(decoder);
    let entries = archive
        .entries()
        .map_err(|e| AppError::bad_request(format!("invalid archive: {e}")))?;
    for entry in entries {
        let entry = entry.map_err(|e| AppError::bad_request(format!("archive error: {e}")))?;
        let path = entry
            .path()
            .map_err(|_| AppError::bad_request("invalid entry path"))?
            .into_owned();
        if !is_safe_tar_path(&path) {
            return Err(AppError::bad_request(format!(
                "unsafe path in archive: {}",
                path.display()
            )));
        }
    }
    Ok(())
}

fn is_safe_tar_path(path: &std::path::Path) -> bool {
    if path.is_absolute() {
        return false;
    }
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => return false,
            std::path::Component::Normal(_) | std::path::Component::CurDir => {}
            _ => {}
        }
    }
    true
}

fn respond_html(result: Result<HtmlPage, AppError>, signed_in: Option<String>) -> Response {
    match result {
        Ok(page) => page.into_response(),
        Err(err) => html_error_response(err, signed_in),
    }
}

fn html_error_response(err: AppError, signed_in: Option<String>) -> Response {
    let status = match &err {
        AppError::Http { status, .. } => *status,
        AppError::Sqlx(_) | AppError::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
        _ => StatusCode::BAD_REQUEST,
    };
    let template = ErrorTemplate {
        signed_in,
        title: format!(
            "{} {}",
            status.as_u16(),
            status.canonical_reason().unwrap_or("Error")
        ),
        message: err.to_string(),
    };
    let body = template
        .render()
        .unwrap_or_else(|_| "internal error".to_string());
    HtmlPage::new(body).status(status).into_response()
}

fn identify_user(headers: &HeaderMap, state: &AppState) -> Option<String> {
    if let Some(token) = token_from_headers(headers) {
        if let Ok(claims) = state.jwt.verify(&token) {
            return Some(claims.username);
        }
    }
    None
}

fn token_from_headers(headers: &HeaderMap) -> Option<String> {
    if let Some(value) = headers.get(header::AUTHORIZATION) {
        if let Ok(text) = value.to_str() {
            if let Some(rest) = text.strip_prefix("Bearer ") {
                return Some(rest.to_string());
            }
        }
    }
    if let Some(cookie) = headers.get(header::COOKIE) {
        if let Ok(text) = cookie.to_str() {
            for part in text.split(';') {
                let trimmed = part.trim();
                if let Some(rest) = trimmed.strip_prefix("apex_token=") {
                    return Some(rest.to_string());
                }
            }
        }
    }
    None
}

fn readme_html_from_tar(path: &str) -> Result<Option<String>, AppError> {
    let file = File::open(path)?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);
    let mut entries = archive
        .entries()
        .map_err(|e| AppError::bad_request(format!("invalid tarball: {e}")))?;
    while let Some(entry) = entries.next() {
        let entry = entry.map_err(|e| AppError::bad_request(format!("archive error: {e}")))?;
        let path = entry
            .path()
            .map_err(|_| AppError::bad_request("archive entry path error"))?
            .into_owned();
        let lower = path
            .file_name()
            .and_then(|f| f.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();
        if lower.starts_with("readme") {
            let mut buffer = Vec::new();
            let mut limited = entry.take(README_LIMIT_BYTES as u64);
            std::io::copy(&mut limited, &mut buffer)
                .map_err(|e| AppError::bad_request(format!("failed to read README: {e}")))?;
            let text = String::from_utf8_lossy(&buffer);
            let mut options = Options::empty();
            options.insert(Options::ENABLE_TABLES);
            options.insert(Options::ENABLE_FOOTNOTES);
            let parser = Parser::new_ext(&text, options);
            let mut html_output = String::new();
            html::push_html(&mut html_output, parser);
            return Ok(Some(html_output));
        }
    }
    Ok(None)
}

fn apply_search_filter(builder: &mut QueryBuilder<Sqlite>, term: Option<&String>) {
    if let Some(value) = term {
        let pattern = format!("%{}%", value);
        builder.push(" WHERE (p.name LIKE ");
        builder.push_bind(pattern.clone());
        builder.push(" OR p.description LIKE ");
        builder.push_bind(pattern);
        builder.push(") ");
    }
}

async fn yank_version(
    State(state): State<AppState>,
    AuthExtractor(user): AuthExtractor,
    Path((name, version)): Path<(String, String)>,
) -> Result<Json<PublishResponse>, AppError> {
    set_yank_state(&state, user.id, name, version, true).await
}

async fn unyank_version(
    State(state): State<AppState>,
    AuthExtractor(user): AuthExtractor,
    Path((name, version)): Path<(String, String)>,
) -> Result<Json<PublishResponse>, AppError> {
    set_yank_state(&state, user.id, name, version, false).await
}

async fn set_yank_state(
    state: &AppState,
    user_id: i64,
    name: String,
    version: String,
    yank: bool,
) -> Result<Json<PublishResponse>, AppError> {
    let package_id = require_package_owner(&state.pool, &name, user_id).await?;
    let result = sqlx::query(
        "UPDATE versions SET yanked = ? WHERE package_id = ? AND version = ?",
    )
    .bind(if yank { 1 } else { 0 })
    .bind(package_id)
    .bind(&version)
    .execute(&state.pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::not_found("version not found"));
    }
    Ok(Json(PublishResponse { name, version }))
}

async fn list_owners(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<OwnerListResponse>, AppError> {
    let package_id = fetch_package_id(&state.pool, &name).await?;
    let owners = fetch_owner_names(&state.pool, package_id).await?;
    Ok(Json(OwnerListResponse {
        package: name,
        owners,
    }))
}

async fn add_owner(
    State(state): State<AppState>,
    AuthExtractor(user): AuthExtractor,
    Path(name): Path<String>,
    Json(payload): Json<OwnerRequest>,
) -> Result<Json<OwnerListResponse>, AppError> {
    let package_id = require_package_owner(&state.pool, &name, user.id).await?;
    let target = fetch_user_by_username(&state.pool, &payload.username)
        .await?
        .ok_or_else(|| AppError::not_found("user not found"))?;
    sqlx::query(
        "INSERT OR IGNORE INTO package_owners(package_id, user_id, role, created_at) VALUES(?,?,?,?)",
    )
    .bind(package_id)
    .bind(target.id)
    .bind("owner")
    .bind(now_string())
    .execute(&state.pool)
    .await?;
    let owners = fetch_owner_names(&state.pool, package_id).await?;
    Ok(Json(OwnerListResponse {
        package: name,
        owners,
    }))
}

async fn remove_owner(
    State(state): State<AppState>,
    AuthExtractor(user): AuthExtractor,
    Path((name, owner)): Path<(String, String)>,
) -> Result<Json<OwnerListResponse>, AppError> {
    let package_id = require_package_owner(&state.pool, &name, user.id).await?;
    let target = fetch_user_by_username(&state.pool, &owner)
        .await?
        .ok_or_else(|| AppError::not_found("user not found"))?;
    let owner_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM package_owners WHERE package_id = ?",
    )
    .bind(package_id)
    .fetch_one(&state.pool)
    .await?;
    if owner_count <= 1 {
        return Err(AppError::bad_request("cannot remove the last owner"));
    }
    let result = sqlx::query(
        "DELETE FROM package_owners WHERE package_id = ? AND user_id = ?",
    )
    .bind(package_id)
    .bind(target.id)
    .execute(&state.pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::bad_request("specified user is not an owner"));
    }
    let owners = fetch_owner_names(&state.pool, package_id).await?;
    Ok(Json(OwnerListResponse {
        package: name,
        owners,
    }))
}

async fn fetch_package_id(pool: &SqlitePool, name: &str) -> Result<i64, AppError> {
    let row = sqlx::query("SELECT id FROM packages WHERE name = ?")
        .bind(name)
        .fetch_optional(pool)
        .await?;
    row.map(|r| r.get("id"))
        .ok_or_else(|| AppError::not_found("package not found"))
}

async fn require_package_owner(
    pool: &SqlitePool,
    name: &str,
    user_id: i64,
) -> Result<i64, AppError> {
    let package_id = fetch_package_id(pool, name).await?;
    let owns = sqlx::query_scalar::<_, i64>(
        "SELECT 1 FROM package_owners WHERE package_id = ? AND user_id = ?",
    )
    .bind(package_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    if owns.is_some() {
        Ok(package_id)
    } else {
        Err(AppError::unauthorized("not an owner of this package"))
    }
}

async fn fetch_owner_names(pool: &SqlitePool, package_id: i64) -> Result<Vec<String>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT u.username
        FROM package_owners o
        JOIN users u ON o.user_id = u.id
        WHERE o.package_id = ?
        ORDER BY u.username COLLATE NOCASE
        "#,
    )
    .bind(package_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| row.get::<String, _>("username"))
        .collect())
}

fn targets_from_opt(raw: Option<&str>) -> TargetsSection {
    raw.and_then(|text| serde_json::from_str::<TargetsSection>(text).ok())
        .unwrap_or_default()
}

fn describe_targets(targets: &TargetsSection) -> Vec<String> {
    let mut labels = Vec::new();
    if targets.afml.is_some() {
        labels.push("AFML".to_string());
    }
    if targets.rust.is_some() {
        labels.push("Rust".to_string());
    }
    if targets.java.is_some() {
        labels.push("Java".to_string());
    }
    labels
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{write::GzEncoder, Compression};
    use reqwest::multipart;
    use reqwest::Client;
    use tar::Builder;
    use tempfile::tempdir;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn publish_list_download_flow() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        let storage_path = dir.path().join("storage");
        let db_path = dir.path().join("registry.db");
        std::fs::File::create(&db_path)?;
        let database_url = format!("sqlite://{}", db_path.display());
        let pool = SqlitePool::connect(&database_url).await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        let storage = Storage::new(&storage_path).await?;
        let state = AppState {
            pool: pool.clone(),
            storage,
            jwt: JwtKeys::new("integration-secret"),
        };
        let app = router(state);
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let handle = tokio::spawn(async move {
            if let Err(err) = axum::serve(listener, app.into_make_service()).await {
                eprintln!("test server error: {err}");
            }
        });

        let base = format!("http://{}", addr);
        let client = Client::new();

        client
            .post(format!("{base}/api/v1/register"))
            .json(&json!({
                "username": "tester",
                "email": "tester@example.com",
                "password": "secret123"
            }))
            .send()
            .await?
            .error_for_status()?;

        let login_resp = client
            .post(format!("{base}/api/v1/login"))
            .json(&json!({"username":"tester","password":"secret123"}))
            .send()
            .await?
            .error_for_status()?;
        let auth: AuthResponse = login_resp.json().await?;

        let manifest = r#"
[package]
name = "hello-afml"
version = "0.1.0"
language = "afml"
description = "demo"
[targets.afml]
entry = "src/lib.afml"
[dependencies]
[registry]
url = "http://localhost"
"#;
        let manifest_json = json!({
            "package": {
                "name": "hello-afml",
                "version": "0.1.0",
                "language": "afml",
                "description": "demo",
                "license": "MIT",
                "keywords": ["demo"],
                "readme": "README.md",
                "min_runtime": ">=1.0.0",
                "authors": []
            },
            "dependencies": {},
            "registry": { "url": "http://localhost" },
            "targets": {
                "afml": { "entry": "src/lib.afml" }
            }
        });
        let tarball = sample_tarball()?;
        let checksum = hex::encode(Sha256::digest(&tarball));
        let form = multipart::Form::new()
            .part("manifest", multipart::Part::text(manifest.to_string()))
            .part(
                "manifest_json",
                multipart::Part::text(manifest_json.to_string()),
            )
            .part(
                "tarball",
                multipart::Part::bytes(tarball.clone())
                    .file_name("hello-afml-0.1.0.apkg")
                    .mime_str("application/gzip")?,
            );

        client
            .post(format!("{base}/api/v1/packages/publish"))
            .bearer_auth(&auth.token)
            .header("X-Checksum", checksum)
            .multipart(form)
            .send()
            .await?
            .error_for_status()?;

        let packages: Vec<PackageSummary> = client
            .get(format!("{base}/api/v1/packages"))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].name, "hello-afml");

        let download_resp = client
            .get(format!("{base}/api/v1/package/hello-afml/0.1.0/download"))
            .send()
            .await?
            .error_for_status()?;
        let body = download_resp.bytes().await?;
        assert!(body.len() > 10);

        let index_html = client.get(format!("{base}/")).send().await?.text().await?;
        assert!(index_html.contains("ApexForge Registry"));

        let packages_resp = client.get(format!("{base}/packages")).send().await?;
        let packages_status = packages_resp.status();
        let packages_html = packages_resp.text().await?;
        assert!(
            packages_status.is_success(),
            "packages page status {packages_status} body {packages_html}"
        );
        assert!(packages_html.contains("hello-afml"));

        let package_resp = client
            .get(format!("{base}/package/hello-afml"))
            .send()
            .await?;
        let package_status = package_resp.status();
        let package_html = package_resp.text().await?;
        assert!(
            package_status.is_success(),
            "package page status {package_status} body {package_html}"
        );
        assert!(package_html.contains("hello-afml"));

        handle.abort();
        Ok(())
    }

    fn sample_tarball() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        {
            let mut builder = Builder::new(&mut encoder);
            let contents = b"fun greet() { }";
            let mut header = tar::Header::new_gnu();
            header.set_path("src/lib.afml")?;
            header.set_size(contents.len() as u64);
            header.set_mode(0o644);
            header.set_uid(0);
            header.set_gid(0);
            header.set_mtime(0);
            header.set_cksum();
            builder.append(&header, &contents[..])?;
            builder.finish()?;
        }
        Ok(encoder.finish()?)
    }
}
