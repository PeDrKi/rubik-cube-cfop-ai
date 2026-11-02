//! Test HTTP cho toàn bộ máy chủ: route, header bảo mật, giới hạn tốc độ, cookie, luồng ván xếp hạng.
//! Gọi thẳng router trong bộ nhớ (không mở cổng mạng), database SQLite trong bộ nhớ.
use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{header, Method, Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use rubik_web::accounts::Accounts;
use rubik_web::{router, App, Config};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::Arc;
use tower::ServiceExt;

const SOLVED: &str = "UUUUUUUUUDDDDDDDDDFFFFFFFFFBBBBBBBBBLLLLLLLLLRRRRRRRRR";

struct Resp {
    status: StatusCode,
    headers: axum::http::HeaderMap,
    body: Vec<u8>,
}

impl Resp {
    fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap_or_else(|e| {
            panic!(
                "không phải JSON ({e}): {}",
                String::from_utf8_lossy(&self.body)
            )
        })
    }
    fn header(&self, k: &str) -> &str {
        self.headers
            .get(k)
            .map(|v| v.to_str().unwrap())
            .unwrap_or("")
    }
    /// Giá trị `rubik_sid=...` từ Set-Cookie (để gửi lại như trình duyệt).
    fn session_cookie(&self) -> String {
        self.header("set-cookie")
            .split(';')
            .next()
            .unwrap()
            .to_string()
    }
}

struct Site {
    app: Router,
    dir: std::path::PathBuf,
}

impl Drop for Site {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn site(cfg: Config) -> Site {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "rubik_http_test_{}_{}",
        std::process::id(),
        N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("index.html"),
        format!("<!doctype html><title>Rubik</title>{}", "x".repeat(500)),
    )
    .unwrap();
    let acc = Arc::new(Accounts::open(":memory:").unwrap());
    Site {
        app: router(App::new(acc, &cfg), &dir),
        dir,
    }
}

async fn send(
    s: &Site,
    method: Method,
    uri: &str,
    body: Option<Value>,
    cookie: Option<&str>,
    extra: &[(&str, &str)],
    ip: &str,
) -> Resp {
    let mut b = Request::builder().method(method).uri(uri);
    if body.is_some() {
        b = b.header(header::CONTENT_TYPE, "application/json");
    }
    if let Some(c) = cookie {
        b = b.header(header::COOKIE, c);
    }
    for (k, v) in extra {
        b = b.header(*k, *v);
    }
    let mut req = b
        .body(Body::from(body.map(|v| v.to_string()).unwrap_or_default()))
        .unwrap();
    req.extensions_mut()
        .insert(ConnectInfo(SocketAddr::new(ip.parse().unwrap(), 5555)));
    let res = s.app.clone().oneshot(req).await.unwrap();
    let (parts, body) = res.into_parts();
    Resp {
        status: parts.status,
        headers: parts.headers,
        body: body.collect().await.unwrap().to_bytes().to_vec(),
    }
}

async fn get(s: &Site, uri: &str) -> Resp {
    send(s, Method::GET, uri, None, None, &[], "10.0.0.1").await
}

async fn post(s: &Site, uri: &str, body: Value) -> Resp {
    send(s, Method::POST, uri, Some(body), None, &[], "10.0.0.1").await
}

async fn register(s: &Site, name: &str) -> String {
    let r = post(
        s,
        "/api/auth/register",
        json!({ "username": name, "password": "matkhau1" }),
    )
    .await;
    assert_eq!(
        r.status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&r.body)
    );
    r.session_cookie()
}

/* ---------- giao diện tĩnh & header ---------- */

#[tokio::test]
async fn phuc_vu_giao_dien_va_health() {
    let s = site(Config::default());
    let r = get(&s, "/").await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(String::from_utf8_lossy(&r.body).contains("<title>Rubik</title>"));
    assert!(r.header("content-type").starts_with("text/html"));
    assert_eq!(get(&s, "/khong-co.js").await.status, StatusCode::NOT_FOUND);
    let h = get(&s, "/api/health").await;
    assert_eq!(
        (h.status, h.json()["ok"].clone()),
        (StatusCode::OK, json!(true))
    );
}

#[tokio::test]
async fn header_bao_mat_tren_moi_phan_hoi() {
    let s = site(Config::default());
    for uri in ["/", "/api/health", "/khong-co.js"] {
        let r = get(&s, uri).await;
        assert_eq!(r.header("x-content-type-options"), "nosniff", "{uri}");
        assert_eq!(r.header("x-frame-options"), "DENY", "{uri}");
        assert_eq!(r.header("referrer-policy"), "same-origin", "{uri}");
        assert!(
            r.header("permissions-policy").contains("camera=(self)"),
            "{uri}"
        );
        let csp = r.header("content-security-policy");
        assert!(
            csp.contains("frame-ancestors 'none'")
                && csp.contains("object-src 'none'")
                && csp.contains("default-src 'self'"),
            "{uri}: {csp}"
        );
    }
    assert_eq!(
        get(&s, "/api/health").await.header("cache-control"),
        "no-store",
        "API không bao giờ cache"
    );
    assert_eq!(
        get(&s, "/").await.header("cache-control"),
        "no-cache",
        "giao diện luôn hỏi lại server"
    );
}

#[tokio::test]
async fn nen_gzip_khi_trinh_duyet_chap_nhan() {
    let s = site(Config::default());
    let r = send(
        &s,
        Method::GET,
        "/",
        None,
        None,
        &[("accept-encoding", "gzip")],
        "10.0.0.1",
    )
    .await;
    assert_eq!(r.header("content-encoding"), "gzip");
    assert_eq!(
        get(&s, "/").await.header("content-encoding"),
        "",
        "không gửi Accept-Encoding thì không nén"
    );
}

/* ---------- route bộ giải ---------- */

#[tokio::test]
async fn route_apply_va_loi_tieng_viet() {
    let s = site(Config::default());
    let r = post(
        &s,
        "/api/apply",
        json!({ "state": SOLVED, "moves": "R U R' U'" }),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json()["solved"], json!(false));
    assert_eq!(post(&s, "/api/apply", json!({ "state": SOLVED, "moves": "R U R' U' R U R' U' R U R' U' R U R' U' R U R' U' R U R' U'" })).await.json()["solved"], json!(true));
    let bad = post(&s, "/api/apply", json!({ "state": SOLVED, "moves": "Q" })).await;
    assert_eq!(bad.status, StatusCode::BAD_REQUEST);
    assert!(bad.json()["error"]
        .as_str()
        .unwrap()
        .contains("Ký hiệu không hợp lệ"));
    let bad = post(&s, "/api/apply", json!({ "state": "ngan", "moves": "R" })).await;
    assert_eq!(bad.status, StatusCode::BAD_REQUEST);
    assert!(bad.json()["error"].as_str().unwrap().contains("54 ký tự"));
    // JSON hỏng cũng trả lỗi gọn, không sập.
    let r = send(&s, Method::POST, "/api/apply", None, None, &[], "10.0.0.1").await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn route_khong_ton_tai_va_ten_qua_dai() {
    let s = site(Config::default());
    let r = get(&s, "/api/khong_co").await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert!(r.json()["error"]
        .as_str()
        .unwrap()
        .contains("Route không tồn tại"));
    assert_eq!(
        get(&s, &format!("/api/{}", "a".repeat(40))).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(get(&s, "/api/solved").await.json()["state"], json!(SOLVED));
}

#[tokio::test]
async fn chan_cong_thuc_doc_hai_qua_http() {
    let s = site(Config::default());
    let r = post(
        &s,
        "/api/apply",
        json!({ "state": SOLVED, "moves": "(R U R' U')999999999" }),
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    let r = post(
        &s,
        "/api/apply",
        json!({ "state": SOLVED, "moves": "(R)99999999999999999999" }),
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn khung_hinh_qua_lon_hoac_sai_kich_thuoc() {
    let s = site(Config::default());
    let big = send(
        &s,
        Method::POST,
        "/api/scan_frame?w=4000&h=3000",
        None,
        None,
        &[],
        "10.0.0.1",
    )
    .await;
    assert_eq!(big.status, StatusCode::PAYLOAD_TOO_LARGE);
    let bad = send(
        &s,
        Method::POST,
        "/api/scan_frame?w=10&h=10",
        None,
        None,
        &[],
        "10.0.0.1",
    )
    .await;
    assert_eq!(
        bad.status,
        StatusCode::BAD_REQUEST,
        "dữ liệu không khớp kích thước"
    );
    assert!(bad.json()["error"].as_str().unwrap().contains("không khớp"));
    assert_eq!(
        send(
            &s,
            Method::POST,
            "/api/scan_frame",
            None,
            None,
            &[],
            "10.0.0.1"
        )
        .await
        .status,
        StatusCode::BAD_REQUEST,
        "thiếu w, h"
    );
}

/* ---------- giới hạn tốc độ ---------- */

#[tokio::test]
async fn gioi_han_toc_do_route_nhe() {
    let s = site(Config {
        light_limit: (3, 60),
        ..Config::default()
    });
    for _ in 0..3 {
        assert_eq!(get(&s, "/api/solved").await.status, StatusCode::OK);
    }
    let r = get(&s, "/api/solved").await;
    assert_eq!(r.status, StatusCode::TOO_MANY_REQUESTS);
    assert!(r.json()["error"].as_str().unwrap().contains("quá nhanh"));
    // IP khác có hạn mức riêng.
    assert_eq!(
        send(&s, Method::GET, "/api/solved", None, None, &[], "10.0.0.2")
            .await
            .status,
        StatusCode::OK
    );
    // Health và giao diện tĩnh không bị tính vào hạn mức.
    assert_eq!(get(&s, "/api/health").await.status, StatusCode::OK);
    assert_eq!(get(&s, "/").await.status, StatusCode::OK);
}

#[tokio::test]
async fn gioi_han_toc_do_route_nang_rieng_voi_route_nhe() {
    let s = site(Config {
        heavy_limit: (2, 60),
        ..Config::default()
    });
    for _ in 0..2 {
        // Trạng thái sai nên trả 400 nhưng vẫn tính một lượt.
        assert_eq!(
            post(&s, "/api/hint", json!({ "state": "x" })).await.status,
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(
        post(&s, "/api/hint", json!({ "state": "x" })).await.status,
        StatusCode::TOO_MANY_REQUESTS
    );
    assert_eq!(
        post(&s, "/api/solve", json!({ "state": "x" })).await.status,
        StatusCode::TOO_MANY_REQUESTS,
        "chung hạn mức nhóm nặng"
    );
    assert_eq!(
        get(&s, "/api/solved").await.status,
        StatusCode::OK,
        "route nhẹ không bị ảnh hưởng"
    );
}

#[tokio::test]
async fn gioi_han_khung_hinh_va_dang_ky() {
    let s = site(Config {
        frame_limit: (1, 60),
        register_limit: (2, 3600),
        ..Config::default()
    });
    let f = || {
        send(
            &s,
            Method::POST,
            "/api/scan_frame?w=10&h=10",
            None,
            None,
            &[],
            "10.0.0.1",
        )
    };
    assert_eq!(f().await.status, StatusCode::BAD_REQUEST);
    assert_eq!(f().await.status, StatusCode::TOO_MANY_REQUESTS);
    register(&s, "ban_a").await;
    register(&s, "ban_b").await;
    let r = post(
        &s,
        "/api/auth/register",
        json!({ "username": "ban_c", "password": "matkhau1" }),
    )
    .await;
    assert_eq!(r.status, StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn ip_theo_x_forwarded_for_chi_khi_tin_proxy() {
    // Không tin proxy: header bị bỏ qua, mọi yêu cầu cùng một IP kết nối.
    let s = site(Config {
        light_limit: (1, 60),
        ..Config::default()
    });
    let g = |xff: &'static str| async {
        send(
            &s,
            Method::GET,
            "/api/solved",
            None,
            None,
            &[("x-forwarded-for", xff)],
            "10.0.0.1",
        )
        .await
    };
    assert_eq!(g("1.1.1.1").await.status, StatusCode::OK);
    assert_eq!(
        g("2.2.2.2").await.status,
        StatusCode::TOO_MANY_REQUESTS,
        "đổi X-Forwarded-For không né được giới hạn"
    );
    // Tin proxy: lấy mục CUỐI (do proxy gần nhất thêm); mục giả do client chèn ở đầu không có tác dụng.
    let s = site(Config {
        light_limit: (1, 60),
        trust_proxy: true,
        ..Config::default()
    });
    let g = |xff: &'static str| async {
        send(
            &s,
            Method::GET,
            "/api/solved",
            None,
            None,
            &[("x-forwarded-for", xff)],
            "10.0.0.1",
        )
        .await
    };
    assert_eq!(g("9.9.9.9, 1.1.1.1").await.status, StatusCode::OK);
    assert_eq!(
        g("8.8.8.8, 1.1.1.1").await.status,
        StatusCode::TOO_MANY_REQUESTS,
        "cùng IP cuối = cùng một người"
    );
    assert_eq!(
        g("8.8.8.8, 2.2.2.2").await.status,
        StatusCode::OK,
        "IP cuối khác = người khác"
    );
}

/* ---------- cookie & phiên ---------- */

#[tokio::test]
async fn cookie_phien_dung_thuoc_tinh_bao_mat() {
    let s = site(Config::default());
    let r = post(
        &s,
        "/api/auth/register",
        json!({ "username": "lien_01", "password": "matkhau1" }),
    )
    .await;
    let c = r.header("set-cookie");
    assert!(c.starts_with("rubik_sid="), "{c}");
    assert!(
        c.contains("HttpOnly")
            && c.contains("SameSite=Lax")
            && c.contains("Path=/")
            && c.contains("Max-Age=2592000"),
        "{c}"
    );
    assert!(
        !c.contains("Secure"),
        "chạy http thường thì không đặt Secure: {c}"
    );
    assert_eq!(r.json()["user"]["username"], json!("lien_01"));
    assert!(
        r.session_cookie().len() > "rubik_sid=".len() + 32,
        "token đủ dài"
    );

    let s = site(Config {
        secure_cookie: true,
        ..Config::default()
    });
    assert!(post(
        &s,
        "/api/auth/register",
        json!({ "username": "lien_01", "password": "matkhau1" })
    )
    .await
    .header("set-cookie")
    .contains("; Secure"));
}

#[tokio::test]
async fn dang_ky_dang_nhap_me_dang_xuat() {
    let s = site(Config::default());
    assert_eq!(
        get(&s, "/api/auth/me").await.json()["user"],
        Value::Null,
        "chưa đăng nhập: user = null, không phải lỗi"
    );
    let c = register(&s, "lien_01").await;
    let me = send(
        &s,
        Method::GET,
        "/api/auth/me",
        None,
        Some(&c),
        &[],
        "10.0.0.1",
    )
    .await;
    assert_eq!(me.json()["user"]["username"], json!("lien_01"));
    // Cookie nằm lẫn với cookie khác vẫn nhận ra.
    let mixed = format!("a=b; {c}; z=1");
    assert_eq!(
        send(
            &s,
            Method::GET,
            "/api/auth/me",
            None,
            Some(&mixed),
            &[],
            "10.0.0.1"
        )
        .await
        .json()["user"]["username"],
        json!("lien_01")
    );
    // Trùng tên (không phân biệt hoa thường) → 409; sai mật khẩu → 401; tên không hợp lệ → 400.
    let r = post(
        &s,
        "/api/auth/register",
        json!({ "username": "LIEN_01", "password": "khac1234" }),
    )
    .await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    let r = post(
        &s,
        "/api/auth/login",
        json!({ "username": "lien_01", "password": "sai" }),
    )
    .await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
    assert!(
        r.headers.get("set-cookie").is_none(),
        "đăng nhập hỏng không được cấp cookie"
    );
    assert_eq!(
        post(
            &s,
            "/api/auth/register",
            json!({ "username": "ab", "password": "matkhau1" })
        )
        .await
        .status,
        StatusCode::BAD_REQUEST
    );
    let login = post(
        &s,
        "/api/auth/login",
        json!({ "username": "lien_01", "password": "matkhau1" }),
    )
    .await;
    assert_eq!(login.status, StatusCode::OK);
    // Đăng xuất: xoá cookie phía trình duyệt VÀ vô hiệu phiên phía server.
    let out = send(
        &s,
        Method::POST,
        "/api/auth/logout",
        Some(json!({})),
        Some(&c),
        &[],
        "10.0.0.1",
    )
    .await;
    assert!(out.header("set-cookie").contains("Max-Age=0"));
    assert_eq!(
        send(
            &s,
            Method::GET,
            "/api/auth/me",
            None,
            Some(&c),
            &[],
            "10.0.0.1"
        )
        .await
        .json()["user"],
        Value::Null
    );
    assert_eq!(
        send(
            &s,
            Method::GET,
            "/api/auth/me",
            None,
            Some(&login.session_cookie()),
            &[],
            "10.0.0.1"
        )
        .await
        .json()["user"]["username"],
        json!("lien_01"),
        "phiên khác vẫn sống"
    );
}

#[tokio::test]
async fn khoa_tam_sau_nhieu_lan_sai_qua_http() {
    let s = site(Config::default());
    register(&s, "nguoichoi").await;
    for _ in 0..5 {
        assert_eq!(
            post(
                &s,
                "/api/auth/login",
                json!({ "username": "nguoichoi", "password": "sai" })
            )
            .await
            .status,
            StatusCode::UNAUTHORIZED
        );
    }
    assert_eq!(
        post(
            &s,
            "/api/auth/login",
            json!({ "username": "nguoichoi", "password": "matkhau1" })
        )
        .await
        .status,
        StatusCode::TOO_MANY_REQUESTS
    );
    let other = send(
        &s,
        Method::POST,
        "/api/auth/login",
        Some(json!({ "username": "nguoichoi", "password": "matkhau1" })),
        None,
        &[],
        "10.0.0.99",
    )
    .await;
    assert_eq!(
        other.status,
        StatusCode::OK,
        "chủ tài khoản ở IP khác vẫn vào được"
    );
}

/* ---------- ván xếp hạng ---------- */

#[tokio::test]
async fn route_can_dang_nhap_tra_401() {
    let s = site(Config::default());
    for (m, uri) in [
        (Method::POST, "/api/game/start"),
        (Method::POST, "/api/game/finish"),
        (Method::POST, "/api/game/abandon"),
        (Method::GET, "/api/profile"),
        (Method::POST, "/api/auth/change_password"),
        (Method::POST, "/api/auth/delete"),
    ] {
        let r = send(&s, m, uri, Some(json!({ "game_id": 1, "moves": "", "old_password": "x", "new_password": "yyyyyy", "password": "x" })), None, &[], "10.0.0.1").await;
        assert_eq!(r.status, StatusCode::UNAUTHORIZED, "{uri}");
    }
    let r = send(
        &s,
        Method::GET,
        "/api/profile",
        None,
        Some("rubik_sid=gia_mao"),
        &[],
        "10.0.0.1",
    )
    .await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
    assert!(r.json()["error"].as_str().unwrap().contains("hết hạn"));
    assert_eq!(
        get(&s, "/api/leaderboard").await.status,
        StatusCode::OK,
        "bảng xếp hạng công khai"
    );
}

#[tokio::test]
async fn luong_van_xep_hang_qua_http() {
    let s = site(Config::default());
    let c = register(&s, "nguoichoi").await;
    let call = |m: Method, uri: &'static str, b: Option<Value>| {
        let c = c.clone();
        let s = &s;
        async move { send(s, m, uri, b, Some(&c), &[], "10.0.0.1").await }
    };
    let g = call(Method::POST, "/api/game/start", Some(json!({}))).await;
    assert_eq!(g.status, StatusCode::OK);
    let g = g.json();
    let (id, scramble) = (
        g["game_id"].as_i64().unwrap(),
        g["scramble"].as_str().unwrap().to_string(),
    );
    assert_eq!(scramble.split(' ').count(), 25);
    assert_eq!(g["state"].as_str().unwrap().len(), 54);
    // Chưa giải xong → 400.
    assert_eq!(
        call(
            Method::POST,
            "/api/game/finish",
            Some(json!({ "game_id": id, "moves": "R" }))
        )
        .await
        .status,
        StatusCode::BAD_REQUEST
    );
    // Nộp ngay lời giải đảo ngược đề xáo: nhanh bất thường → bị từ chối và ván bị huỷ.
    let inv: Vec<String> = scramble
        .split(' ')
        .rev()
        .map(|m| {
            if let Some(b) = m.strip_suffix('\'') {
                b.to_string()
            } else if m.ends_with('2') {
                m.to_string()
            } else {
                format!("{m}'")
            }
        })
        .collect();
    let r = call(
        Method::POST,
        "/api/game/finish",
        Some(json!({ "game_id": id, "moves": inv.join(" ") })),
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert!(r.json()["error"]
        .as_str()
        .unwrap()
        .contains("nhanh bất thường"));
    assert_eq!(
        call(
            Method::POST,
            "/api/game/finish",
            Some(json!({ "game_id": id, "moves": inv.join(" ") }))
        )
        .await
        .status,
        StatusCode::CONFLICT,
        "ván đã huỷ"
    );
    // Ván mới → huỷ chủ động → không nộp được nữa.
    let id2 = call(Method::POST, "/api/game/start", Some(json!({})))
        .await
        .json()["game_id"]
        .as_i64()
        .unwrap();
    assert_eq!(
        call(
            Method::POST,
            "/api/game/abandon",
            Some(json!({ "game_id": id2 }))
        )
        .await
        .status,
        StatusCode::OK
    );
    assert_eq!(
        call(
            Method::POST,
            "/api/game/finish",
            Some(json!({ "game_id": id2, "moves": "R" }))
        )
        .await
        .status,
        StatusCode::CONFLICT
    );
    // Không nộp/huỷ hộ ván của người khác.
    let c2 = register(&s, "nguoikhac").await;
    let r = send(
        &s,
        Method::POST,
        "/api/game/finish",
        Some(json!({ "game_id": id2, "moves": "R" })),
        Some(&c2),
        &[],
        "10.0.0.1",
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    // Hồ sơ & bảng xếp hạng.
    let p = call(Method::GET, "/api/profile", None).await.json();
    assert_eq!(
        (
            p["username"].clone(),
            p["games"].clone(),
            p["solves"].clone()
        ),
        (json!("nguoichoi"), json!(2), json!(0))
    );
    assert_eq!(
        get(&s, "/api/leaderboard?by=moves&limit=5").await.json()["rows"],
        json!([])
    );
}

/* ---------- body lớn ---------- */

#[tokio::test]
async fn tu_choi_body_qua_16mb() {
    let s = site(Config::default());
    let big = vec![b'a'; (16 << 20) + 1024];
    let mut req = Request::builder()
        .method(Method::POST)
        .uri("/api/apply")
        .body(Body::from(big))
        .unwrap();
    req.extensions_mut()
        .insert(ConnectInfo(SocketAddr::new("10.0.0.1".parse().unwrap(), 1)));
    let res = s.app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

/* ---------- việc nền ---------- */

#[test]
fn stamp_dung_ngay_gio_utc() {
    assert_eq!(rubik_web::stamp(0), "19700101-000000");
    assert_eq!(
        rubik_web::stamp(951_782_400_000 + 1_000),
        "20000229-000001",
        "ngày nhuận 29/2/2000"
    );
    assert_eq!(rubik_web::stamp(1_793_461_260_000), "20261031-154100");
}

#[test]
fn sao_luu_theo_thu_muc_giu_dung_so_ban() {
    let dir = std::env::temp_dir().join(format!("rubik_backup_dir_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let acc = Accounts::open(":memory:").unwrap();
    // Các bản cũ có sẵn (tên theo thời gian) và một file lạ không được đụng tới.
    std::fs::create_dir_all(&dir).unwrap();
    for n in [
        "rubik-20200101-000000.db",
        "rubik-20200102-000000.db",
        "rubik-20200103-000000.db",
        "ghi_chu.txt",
    ] {
        std::fs::write(dir.join(n), "x").unwrap();
    }
    let made = rubik_web::backup_into_dir(&acc, &dir, 2).unwrap();
    assert!(made.exists());
    let mut left: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    left.sort();
    assert_eq!(left.len(), 3, "2 bản sao lưu mới nhất + file lạ: {left:?}");
    assert!(left.contains(&"ghi_chu.txt".to_string()));
    assert!(
        left.contains(&"rubik-20200103-000000.db".to_string())
            && !left.contains(&"rubik-20200101-000000.db".to_string())
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/* ---------- đổi mật khẩu & xoá tài khoản ---------- */

#[tokio::test]
async fn doi_mat_khau_qua_http() {
    let s = site(Config::default());
    let c = register(&s, "nguoichoi").await;
    let other = post(
        &s,
        "/api/auth/login",
        json!({ "username": "nguoichoi", "password": "matkhau1" }),
    )
    .await
    .session_cookie();
    let call = |b: Value| {
        send(
            &s,
            Method::POST,
            "/api/auth/change_password",
            Some(b),
            Some(&c),
            &[],
            "10.0.0.1",
        )
    };
    let r = call(json!({ "old_password": "sai", "new_password": "matkhau2" })).await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
    assert!(r.json()["error"]
        .as_str()
        .unwrap()
        .contains("Mật khẩu hiện tại"));
    assert_eq!(
        call(json!({ "old_password": "matkhau1", "new_password": "ab" }))
            .await
            .status,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        call(json!({ "old_password": "matkhau1", "new_password": "matkhau2" }))
            .await
            .status,
        StatusCode::OK
    );
    let me = |ck: String| {
        let s = &s;
        async move {
            send(
                s,
                Method::GET,
                "/api/auth/me",
                None,
                Some(&ck),
                &[],
                "10.0.0.1",
            )
            .await
        }
    };
    assert_eq!(
        me(c.clone()).await.json()["user"]["username"],
        json!("nguoichoi"),
        "phiên hiện tại còn sống"
    );
    assert_eq!(
        me(other.clone()).await.json()["user"],
        Value::Null,
        "phiên khác bị đăng xuất"
    );
    assert_eq!(
        post(
            &s,
            "/api/auth/login",
            json!({ "username": "nguoichoi", "password": "matkhau2" })
        )
        .await
        .status,
        StatusCode::OK
    );
}

#[tokio::test]
async fn xoa_tai_khoan_qua_http() {
    let s = site(Config::default());
    let c = register(&s, "nguoichoi").await;
    let del = |pw: &str| {
        send(
            &s,
            Method::POST,
            "/api/auth/delete",
            Some(json!({ "password": pw })),
            Some(&c),
            &[],
            "10.0.0.1",
        )
    };
    assert_eq!(del("sai").await.status, StatusCode::UNAUTHORIZED);
    let r = del("matkhau1").await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(
        r.header("set-cookie").contains("Max-Age=0"),
        "xoá cookie phía trình duyệt"
    );
    assert_eq!(
        send(
            &s,
            Method::GET,
            "/api/auth/me",
            None,
            Some(&c),
            &[],
            "10.0.0.1"
        )
        .await
        .json()["user"],
        Value::Null
    );
    assert_eq!(
        post(
            &s,
            "/api/auth/login",
            json!({ "username": "nguoichoi", "password": "matkhau1" })
        )
        .await
        .status,
        StatusCode::UNAUTHORIZED
    );
}
