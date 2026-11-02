//! rubik-web: server axum mỏng. Logic giải nằm trong `rubik-api`; tài khoản và xếp hạng nằm trong `accounts`.
//!
//! Router nằm ở đây (thư viện) chứ không ở `main.rs` để `tests/http.rs` dựng được cả ứng dụng trong bộ nhớ và
//! kiểm thử tự động mọi route, header bảo mật, giới hạn tốc độ và cookie mà không cần mở cổng mạng.
pub mod accounts;

use accounts::{Accounts, ApiErr, SESSION_SECS};
use axum::{
    body::Bytes,
    extract::{ConnectInfo, DefaultBodyLimit, Path, Query, Request, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;
use std::path::{Path as FsPath, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use tower_http::{compression::CompressionLayer, services::ServeDir};

type Res = (StatusCode, Json<Value>);

fn reply(r: Result<Result<Value, String>, tokio::task::JoinError>) -> Res {
    match r {
        Ok(Ok(v)) => (StatusCode::OK, Json(v)),
        Ok(Err(e)) => (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("Lỗi nội bộ: {e}") })),
        ),
    }
}

/// Route tốn CPU (tìm kiếm / xử lý ảnh): bị giới hạn số tác vụ chạy đồng thời và số lần gọi mỗi IP.
fn is_heavy(name: &str) -> bool {
    matches!(name, "hint" | "solve" | "scan_resolve" | "scan_aggregate")
}

fn busy() -> Res {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({ "error": "Máy chủ đang bận, hãy thử lại sau ít giây" })),
    )
}

fn too_many() -> Res {
    (
        StatusCode::TOO_MANY_REQUESTS,
        Json(json!({ "error": "Bạn thao tác quá nhanh, hãy thử lại sau ít phút" })),
    )
}

/// Chờ tối đa 15 giây để có chỗ chạy; hết hạn thì trả 503 thay vì dồn hàng đợi vô hạn.
async fn run_limited(
    app: &App,
    ip: &str,
    f: impl FnOnce() -> Result<Value, String> + Send + 'static,
) -> Res {
    if !app.heavy_rl.allow(ip) {
        return too_many();
    }
    let Ok(Ok(permit)) =
        tokio::time::timeout(Duration::from_secs(15), app.jobs.clone().acquire_owned()).await
    else {
        return busy();
    };
    let r = tokio::task::spawn_blocking(f).await;
    drop(permit);
    match r {
        Ok(Ok(v)) => (StatusCode::OK, Json(v)),
        Ok(Err(e)) => (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("Lỗi nội bộ: {e}") })),
        ),
    }
}

/// Route nhẹ (apply, validate, scramble...) vẫn bị giới hạn theo IP, nhưng rộng tay: một lần
/// giải tự động ~100 nước gọi `apply` từng nước một, nên mức này đủ cho người thật.
async fn call_route(app: App, ip: String, name: String, body: String) -> Res {
    if name.len() > 32 {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Route không tồn tại" })),
        );
    }
    if is_heavy(&name) {
        return run_limited(&app, &ip, move || rubik_api::route(&name, &body)).await;
    }
    if !app.light_rl.allow(&ip) {
        return too_many();
    }
    reply(tokio::task::spawn_blocking(move || rubik_api::route(&name, &body)).await)
}

async fn call_post(
    State(app): State<App>,
    ConnectInfo(sa): ConnectInfo<SocketAddr>,
    h: HeaderMap,
    Path(name): Path<String>,
    body: String,
) -> Res {
    let ip = client_ip(&app, &h, sa);
    call_route(app, ip, name, body).await
}

async fn call_get(
    State(app): State<App>,
    ConnectInfo(sa): ConnectInfo<SocketAddr>,
    h: HeaderMap,
    Path(name): Path<String>,
) -> Res {
    let ip = client_ip(&app, &h, sa);
    call_route(app, ip, name, String::new()).await
}

#[derive(Deserialize)]
struct FrameQ {
    w: u32,
    h: u32,
    s: Option<f32>,
}

/// Khung hình lớn nhất nhận xử lý (đủ cho ảnh 1920x1080; trang web gửi tối đa 640px).
const MAX_FRAME_PIXELS: u64 = 1920 * 1080;

async fn frame(
    State(app): State<App>,
    ConnectInfo(sa): ConnectInfo<SocketAddr>,
    h: HeaderMap,
    Query(q): Query<FrameQ>,
    body: Bytes,
) -> Res {
    // Quét camera gửi ~10 khung hình mỗi giây: giới hạn theo IP rộng tay (chặn spam, không chặn người quét thật).
    if !app.frame_rl.allow(&client_ip(&app, &h, sa)) {
        return too_many();
    }
    if q.w as u64 * q.h as u64 > MAX_FRAME_PIXELS {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(json!({ "error": "Khung hình quá lớn" })),
        );
    }
    let Ok(Ok(permit)) =
        tokio::time::timeout(Duration::from_secs(15), app.jobs.clone().acquire_owned()).await
    else {
        return busy();
    };
    let r = tokio::task::spawn_blocking(move || {
        rubik_api::scan_frame(q.w, q.h, &body, q.s.unwrap_or(0.7))
    })
    .await;
    drop(permit);
    reply(r)
}

/// Giới hạn kiểu cửa sổ trượt: tối đa `max` lần trong `window` cho mỗi khoá (IP).
struct RateLimiter {
    max: usize,
    window: Duration,
    hits: Mutex<HashMap<String, VecDeque<Instant>>>,
}

impl RateLimiter {
    fn new(max: usize, window: Duration) -> Self {
        RateLimiter {
            max,
            window,
            hits: Mutex::new(HashMap::new()),
        }
    }

    fn allow(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut m = self.hits.lock().unwrap_or_else(|e| e.into_inner());
        if m.len() > 10_000 {
            m.retain(|_, q| {
                q.back()
                    .is_some_and(|t| now.duration_since(*t) < self.window)
            });
        }
        let q = m.entry(key.to_string()).or_default();
        while q
            .front()
            .is_some_and(|t| now.duration_since(*t) >= self.window)
        {
            q.pop_front();
        }
        if q.len() >= self.max {
            return false;
        }
        q.push_back(now);
        true
    }
}

/// IP của client. Sau reverse proxy (Render, nginx...) hãy đặt RUBIK_TRUST_PROXY=1 để lấy từ
/// X-Forwarded-For (mục CUỐI, do proxy gần nhất thêm vào); không đặt thì dùng địa chỉ kết nối trực tiếp.
fn client_ip(app: &App, h: &HeaderMap, sa: SocketAddr) -> String {
    if app.trust_proxy {
        if let Some(ip) = h
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.rsplit(',').next())
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            return ip.to_string();
        }
    }
    sa.ip().to_string()
}

/* ============ tài khoản & xếp hạng ============ */

/// Cấu hình chạy. `Config::from_env()` đọc biến môi trường; test dựng thẳng giá trị (ví dụ giới hạn rất nhỏ).
#[derive(Clone, Debug)]
pub struct Config {
    /// Thêm cờ `Secure` cho cookie (bật khi chạy sau HTTPS: RUBIK_SECURE_COOKIE=1).
    pub secure_cookie: bool,
    /// Tin header X-Forwarded-For (RUBIK_TRUST_PROXY=1) khi chạy sau reverse proxy.
    pub trust_proxy: bool,
    /// Số tác vụ nặng chạy cùng lúc (RUBIK_MAX_JOBS, mặc định 2).
    pub max_jobs: usize,
    /// (số lần tối đa, cửa sổ tính bằng giây) theo IP cho từng nhóm route.
    pub heavy_limit: (usize, u64),
    pub light_limit: (usize, u64),
    pub frame_limit: (usize, u64),
    pub login_limit: (usize, u64),
    pub register_limit: (usize, u64),
}

impl Default for Config {
    fn default() -> Self {
        Config {
            secure_cookie: false,
            trust_proxy: false,
            max_jobs: 2,
            heavy_limit: (120, 60),
            light_limit: (1200, 60),
            frame_limit: (1500, 60),
            login_limit: (30, 300),
            register_limit: (10, 3600),
        }
    }
}

impl Config {
    pub fn from_env() -> Self {
        let env_on = |k: &str| std::env::var(k).is_ok_and(|v| v == "1");
        Config {
            secure_cookie: env_on("RUBIK_SECURE_COOKIE"),
            trust_proxy: env_on("RUBIK_TRUST_PROXY"),
            max_jobs: std::env::var("RUBIK_MAX_JOBS")
                .ok()
                .and_then(|v| v.parse::<usize>().ok())
                .filter(|&n| n >= 1)
                .unwrap_or(2),
            ..Config::default()
        }
    }
}

#[derive(Clone)]
pub struct App {
    acc: Arc<Accounts>,
    secure: bool,
    trust_proxy: bool,
    jobs: Arc<Semaphore>,
    heavy_rl: Arc<RateLimiter>,
    light_rl: Arc<RateLimiter>,
    frame_rl: Arc<RateLimiter>,
    login_rl: Arc<RateLimiter>,
    register_rl: Arc<RateLimiter>,
}

impl App {
    pub fn new(acc: Arc<Accounts>, cfg: &Config) -> Self {
        let rl =
            |(max, secs): (usize, u64)| Arc::new(RateLimiter::new(max, Duration::from_secs(secs)));
        App {
            acc,
            secure: cfg.secure_cookie,
            trust_proxy: cfg.trust_proxy,
            jobs: Arc::new(Semaphore::new(cfg.max_jobs.max(1))),
            heavy_rl: rl(cfg.heavy_limit),
            light_rl: rl(cfg.light_limit),
            frame_rl: rl(cfg.frame_limit),
            login_rl: rl(cfg.login_limit),
            register_rl: rl(cfg.register_limit),
        }
    }
}

const COOKIE: &str = "rubik_sid";

fn token_of(h: &HeaderMap) -> Option<String> {
    h.get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .find_map(|kv| {
            kv.trim()
                .strip_prefix(COOKIE)
                .and_then(|r| r.strip_prefix('='))
                .map(str::to_string)
        })
        .filter(|t| !t.is_empty())
}

fn cookie(app: &App, token: &str, max_age: i64) -> HeaderValue {
    let secure = if app.secure { "; Secure" } else { "" };
    HeaderValue::from_str(&format!(
        "{COOKIE}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={max_age}{secure}"
    ))
    .unwrap()
}

fn ok(v: Value) -> Response {
    (StatusCode::OK, Json(v)).into_response()
}

fn fail(e: ApiErr) -> Response {
    (
        StatusCode::from_u16(e.0).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
        Json(json!({ "error": e.1 })),
    )
        .into_response()
}

/// Chạy việc database trên thread chặn, gộp lỗi về một kiểu.
async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, ApiErr> + Send + 'static,
) -> Result<T, ApiErr> {
    tokio::task::spawn_blocking(f)
        .await
        .unwrap_or_else(|e| Err(ApiErr(500, format!("Lỗi nội bộ: {e}"))))
}

/// Người dùng đang đăng nhập, hoặc lỗi 401.
async fn require_user(app: &App, h: &HeaderMap) -> Result<(i64, String), ApiErr> {
    let Some(tok) = token_of(h) else {
        return Err(ApiErr(401, "Bạn cần đăng nhập".into()));
    };
    let acc = app.acc.clone();
    blocking(move || acc.user_of(&tok))
        .await?
        .ok_or_else(|| ApiErr(401, "Phiên đăng nhập đã hết hạn, hãy đăng nhập lại".into()))
}

#[derive(Deserialize)]
struct Cred {
    username: String,
    password: String,
}

async fn auth_with(app: App, c: Cred, register: bool, ip: String) -> Response {
    let rl = if register {
        &app.register_rl
    } else {
        &app.login_rl
    };
    if !rl.allow(&ip) {
        return fail(ApiErr(
            429,
            "Bạn thao tác quá nhanh, hãy thử lại sau ít phút".into(),
        ));
    }
    let acc = app.acc.clone();
    match blocking(move || {
        if register {
            acc.register(&c.username, &c.password)
        } else {
            acc.login(&c.username, &c.password, &ip)
        }
    })
    .await
    {
        Ok((tok, user)) => {
            let mut r = ok(json!({ "user": user }));
            r.headers_mut()
                .insert(header::SET_COOKIE, cookie(&app, &tok, SESSION_SECS));
            r
        }
        Err(e) => fail(e),
    }
}

async fn register(
    State(app): State<App>,
    ConnectInfo(sa): ConnectInfo<SocketAddr>,
    h: HeaderMap,
    Json(c): Json<Cred>,
) -> Response {
    let ip = client_ip(&app, &h, sa);
    auth_with(app, c, true, ip).await
}

async fn login(
    State(app): State<App>,
    ConnectInfo(sa): ConnectInfo<SocketAddr>,
    h: HeaderMap,
    Json(c): Json<Cred>,
) -> Response {
    let ip = client_ip(&app, &h, sa);
    auth_with(app, c, false, ip).await
}

async fn logout(State(app): State<App>, h: HeaderMap) -> Response {
    if let Some(tok) = token_of(&h) {
        let acc = app.acc.clone();
        if let Err(e) = blocking(move || acc.logout(&tok)).await {
            return fail(e);
        }
    }
    let mut r = ok(json!({}));
    r.headers_mut()
        .insert(header::SET_COOKIE, cookie(&app, "", 0));
    r
}

async fn me(State(app): State<App>, h: HeaderMap) -> Response {
    match require_user(&app, &h).await {
        Ok((id, name)) => ok(json!({ "user": { "id": id, "username": name } })),
        Err(ApiErr(401, _)) => ok(json!({ "user": null })),
        Err(e) => fail(e),
    }
}

#[derive(Deserialize)]
struct ChangePw {
    old_password: String,
    new_password: String,
}

async fn change_password(
    State(app): State<App>,
    ConnectInfo(sa): ConnectInfo<SocketAddr>,
    h: HeaderMap,
    Json(c): Json<ChangePw>,
) -> Response {
    let ip = client_ip(&app, &h, sa);
    if !app.login_rl.allow(&ip) {
        return fail(ApiErr(
            429,
            "Bạn thao tác quá nhanh, hãy thử lại sau ít phút".into(),
        ));
    }
    let r = async {
        let (uid, _) = require_user(&app, &h).await?;
        let tok = token_of(&h).unwrap_or_default();
        let acc = app.acc.clone();
        blocking(move || acc.change_password(uid, &tok, &c.old_password, &c.new_password, &ip))
            .await
    };
    r.await.map_or_else(fail, |_| ok(json!({})))
}

#[derive(Deserialize)]
struct DeleteAcc {
    password: String,
}

async fn delete_account(
    State(app): State<App>,
    ConnectInfo(sa): ConnectInfo<SocketAddr>,
    h: HeaderMap,
    Json(d): Json<DeleteAcc>,
) -> Response {
    let ip = client_ip(&app, &h, sa);
    if !app.login_rl.allow(&ip) {
        return fail(ApiErr(
            429,
            "Bạn thao tác quá nhanh, hãy thử lại sau ít phút".into(),
        ));
    }
    let r = async {
        let (uid, _) = require_user(&app, &h).await?;
        let acc = app.acc.clone();
        blocking(move || acc.delete_account(uid, &d.password, &ip)).await
    };
    match r.await {
        Ok(()) => {
            let mut res = ok(json!({}));
            res.headers_mut()
                .insert(header::SET_COOKIE, cookie(&app, "", 0));
            res
        }
        Err(e) => fail(e),
    }
}

async fn game_start(State(app): State<App>, h: HeaderMap) -> Response {
    let r = async {
        let (uid, _) = require_user(&app, &h).await?;
        let acc = app.acc.clone();
        blocking(move || acc.start_game(uid)).await
    };
    r.await.map_or_else(fail, ok)
}

#[derive(Deserialize)]
struct Finish {
    game_id: i64,
    moves: String,
}

async fn game_finish(State(app): State<App>, h: HeaderMap, Json(f): Json<Finish>) -> Response {
    let r = async {
        let (uid, _) = require_user(&app, &h).await?;
        let acc = app.acc.clone();
        blocking(move || acc.finish_game(uid, f.game_id, &f.moves)).await
    };
    r.await.map_or_else(fail, ok)
}

#[derive(Deserialize)]
struct Abandon {
    game_id: i64,
}

async fn game_abandon(State(app): State<App>, h: HeaderMap, Json(a): Json<Abandon>) -> Response {
    let r = async {
        let (uid, _) = require_user(&app, &h).await?;
        let acc = app.acc.clone();
        blocking(move || acc.abandon_game(uid, a.game_id)).await
    };
    r.await.map_or_else(fail, |_| ok(json!({})))
}

#[derive(Deserialize)]
struct BoardQ {
    by: Option<String>,
    limit: Option<i64>,
}

async fn leaderboard(State(app): State<App>, Query(q): Query<BoardQ>) -> Response {
    let acc = app.acc.clone();
    blocking(move || acc.leaderboard(q.by.as_deref().unwrap_or("time"), q.limit.unwrap_or(50)))
        .await
        .map_or_else(fail, ok)
}

async fn profile(State(app): State<App>, h: HeaderMap) -> Response {
    let r = async {
        let (uid, name) = require_user(&app, &h).await?;
        let acc = app.acc.clone();
        blocking(move || acc.profile(uid, &name)).await
    };
    r.await.map_or_else(fail, ok)
}

async fn health() -> Response {
    ok(json!({ "ok": true, "version": env!("CARGO_PKG_VERSION") }))
}

/// Header bảo mật cho mọi phản hồi + chống cache cũ cho giao diện.
/// CSP cho phép script/style nội tuyến (trang là một file HTML duy nhất); font nằm cùng nguồn, không gọi Google; chặn nhúng trang
/// vào iframe của site khác (clickjacking) và chỉ cho gọi API cùng nguồn.
async fn security_headers(req: Request, next: Next) -> Response {
    let is_api = req.uri().path().starts_with("/api/");
    let mut res = next.run(req).await;
    let h = res.headers_mut();
    let set = |h: &mut HeaderMap, k: &'static str, v: &'static str| {
        h.entry(k).or_insert(HeaderValue::from_static(v));
    };
    set(h, "x-content-type-options", "nosniff");
    set(h, "referrer-policy", "same-origin");
    set(h, "x-frame-options", "DENY");
    set(
        h,
        "permissions-policy",
        "camera=(self), microphone=(), geolocation=()",
    );
    set(
        h,
        "content-security-policy",
        "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; \
         font-src 'self'; img-src 'self' data: blob:; media-src 'self' blob: mediastream:; \
         connect-src 'self'; worker-src 'self' blob:; object-src 'none'; base-uri 'self'; frame-ancestors 'none'; form-action 'self'",
    );
    // API: không bao giờ cache. Giao diện: luôn hỏi lại server (ETag/Last-Modified) để bản cập nhật hiện ra ngay.
    set(
        h,
        "cache-control",
        if is_api { "no-store" } else { "no-cache" },
    );
    res
}

/// Thư mục giao diện. Thứ tự: biến RUBIK_STATIC → thư mục `static` cạnh file chạy → thư mục lúc build
/// → `rubik-web/static` hoặc `static` trong thư mục hiện tại. Nhờ vậy chép riêng file chạy + thư mục
/// `static` sang máy khác (Docker, VPS) vẫn chạy được, không bắt buộc build ngay trên máy đó.
pub fn static_dir() -> PathBuf {
    let has_index = |p: &FsPath| p.join("index.html").is_file();
    if let Some(p) = std::env::var_os("RUBIK_STATIC").map(PathBuf::from) {
        if !has_index(&p) {
            eprintln!(
                "CẢNH BÁO: RUBIK_STATIC={} không chứa index.html",
                p.display()
            );
        }
        return p;
    }
    let mut cands = Vec::new();
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|d| d.to_path_buf()))
    {
        cands.push(dir.join("static"));
    }
    cands.push(PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/static"
    )));
    cands.push(PathBuf::from("rubik-web/static"));
    cands.push(PathBuf::from("static"));
    cands.iter().find(|p| has_index(p)).cloned().unwrap_or_else(|| {
        eprintln!("CẢNH BÁO: không tìm thấy thư mục giao diện (index.html). Đặt RUBIK_STATIC=<đường dẫn tới rubik-web/static>.");
        cands[0].clone()
    })
}

/// Chờ Ctrl+C hoặc SIGTERM (Render/systemd/Docker gửi khi dừng) để tắt êm: hoàn tất các yêu cầu đang chạy.
pub async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let term = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut s) => {
                s.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = term => {} }
    println!("Đang tắt server…");
}

/// Dựng toàn bộ ứng dụng: /api/*, giao diện tĩnh, header bảo mật, nén gzip.
pub fn router(app: App, static_dir: &FsPath) -> Router {
    let accounts = Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/me", get(me))
        .route("/auth/change_password", post(change_password))
        .route("/auth/delete", post(delete_account))
        .route("/game/start", post(game_start))
        .route("/game/finish", post(game_finish))
        .route("/game/abandon", post(game_abandon))
        .route("/leaderboard", get(leaderboard))
        .route("/profile", get(profile));
    let api = Router::new()
        .route("/health", get(health))
        .route("/scan_frame", post(frame))
        .route("/:name", get(call_get).post(call_post))
        .merge(accounts)
        .with_state(app)
        .layer(DefaultBodyLimit::max(16 << 20));
    Router::new()
        .nest("/api", api)
        .fallback_service(ServeDir::new(static_dir))
        .layer(middleware::from_fn(security_headers))
        // Nén gzip: trang HTML ~70 KB còn ~20 KB, tải nhanh hơn hẳn trên điện thoại. Không nén khung hình camera (gửi lên, không phải tải về).
        .layer(CompressionLayer::new())
}

/// Địa chỉ nghe. Render/Heroku đặt biến PORT: tự nghe trên 0.0.0.0:PORT nếu chưa đặt RUBIK_ADDR.
pub fn listen_addr() -> String {
    std::env::var("RUBIK_ADDR")
        .ok()
        .or_else(|| std::env::var("PORT").ok().map(|p| format!("0.0.0.0:{p}")))
        .unwrap_or_else(|| "127.0.0.1:8080".into())
}

/* ============ việc nền: dọn dữ liệu cũ và sao lưu định kỳ ============ */

/// Thời gian dạng `20261031-154100` (UTC) từ mili giây kể từ epoch, để đặt tên file sao lưu. Không cần thư viện ngày giờ.
pub fn stamp(ms: i64) -> String {
    let secs = ms.div_euclid(1000);
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Thuật toán civil_from_days (Howard Hinnant).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}{m:02}{d:02}-{:02}{:02}{:02}",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

/// Sao lưu vào `dir` thành `rubik-<thời gian>.db`, rồi chỉ giữ `keep` bản mới nhất.
pub fn backup_into_dir(acc: &Accounts, dir: &FsPath, keep: usize) -> Result<PathBuf, ApiErr> {
    std::fs::create_dir_all(dir)
        .map_err(|e| ApiErr(500, format!("Không tạo được thư mục sao lưu: {e}")))?;
    let dest = dir.join(format!("rubik-{}.db", stamp(accounts::now_ms())));
    acc.backup_to(&dest)?;
    // Tên có thời gian ở dạng sắp xếp được nên xếp theo tên là xếp theo thời gian.
    let mut old: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.starts_with("rubik-") && n.ends_with(".db"))
                })
                .collect()
        })
        .unwrap_or_default();
    old.sort();
    let extra = old.len().saturating_sub(keep.max(1));
    for p in old.into_iter().take(extra) {
        let _ = std::fs::remove_file(p);
    }
    Ok(dest)
}

/// Khởi chạy việc nền (gọi trong runtime tokio): dọn dữ liệu cũ lúc khởi động rồi mỗi giờ; nếu đặt `RUBIK_BACKUP_DIR`
/// thì sao lưu mỗi `RUBIK_BACKUP_HOURS` giờ (mặc định 24), giữ `RUBIK_BACKUP_KEEP` bản (mặc định 7).
pub fn spawn_maintenance(acc: Arc<Accounts>) {
    let a = acc.clone();
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(3600));
        loop {
            tick.tick().await; // lần đầu chạy ngay
            let a = a.clone();
            match tokio::task::spawn_blocking(move || a.cleanup()).await {
                Ok(Ok(c)) if c != accounts::Cleanup::default() => println!(
                    "Dọn dẹp: {} ván quá hạn → bỏ dở, {} ván cũ đã xoá, {} phiên hết hạn đã xoá",
                    c.stale_games, c.old_games, c.expired_sessions
                ),
                Ok(Err(e)) => eprintln!("Dọn dẹp thất bại: {}", e.1),
                _ => {}
            }
        }
    });
    if let Some(dir) = std::env::var_os("RUBIK_BACKUP_DIR").map(PathBuf::from) {
        let env_num = |k: &str, d: u64| {
            std::env::var(k)
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .filter(|&n| n >= 1)
                .unwrap_or(d)
        };
        let (hours, keep) = (
            env_num("RUBIK_BACKUP_HOURS", 24),
            env_num("RUBIK_BACKUP_KEEP", 7) as usize,
        );
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_secs(hours * 3600));
            loop {
                tick.tick().await;
                let (acc, dir) = (acc.clone(), dir.clone());
                match tokio::task::spawn_blocking(move || backup_into_dir(&acc, &dir, keep)).await {
                    Ok(Ok(p)) => println!("Đã sao lưu database: {}", p.display()),
                    Ok(Err(e)) => eprintln!("Sao lưu thất bại: {}", e.1),
                    Err(e) => eprintln!("Sao lưu thất bại: {e}"),
                }
            }
        });
    }
}
