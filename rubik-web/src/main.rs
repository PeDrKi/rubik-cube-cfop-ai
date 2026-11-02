//! rubik-web: chạy server. Toàn bộ router và logic nằm trong thư viện (`lib.rs`).
//!
//!   rubik-web                     chạy server
//!   rubik-web backup <file.db>    sao lưu database ngay (KHÔNG cần tắt server đang chạy)
use rubik_web::accounts::Accounts;
use rubik_web::{listen_addr, router, shutdown_signal, spawn_maintenance, static_dir, App, Config};
use std::net::SocketAddr;
use std::sync::Arc;

fn open_db() -> (Accounts, String) {
    let db_path = std::env::var("RUBIK_DB").unwrap_or_else(|_| "rubik.db".into());
    (
        Accounts::open(&db_path)
            .unwrap_or_else(|e| panic!("Không mở được database {db_path}: {e}")),
        db_path,
    )
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("backup") {
        let Some(dest) = args.get(1) else {
            eprintln!("Cách dùng: rubik-web backup <file-sao-luu.db>");
            std::process::exit(2);
        };
        let (acc, db_path) = open_db();
        match acc.backup_to(std::path::Path::new(dest)) {
            Ok(()) => println!("Đã sao lưu {db_path} → {dest}"),
            Err(e) => {
                eprintln!("Sao lưu thất bại: {}", e.1);
                std::process::exit(1);
            }
        }
        return;
    }
    if !args.is_empty() {
        eprintln!(
            "Tham số không hợp lệ: {}. Dùng `rubik-web` hoặc `rubik-web backup <file.db>`",
            args.join(" ")
        );
        std::process::exit(2);
    }
    std::thread::spawn(rubik_api::warmup);
    let (acc, db_path) = open_db();
    let acc = Arc::new(acc);
    spawn_maintenance(acc.clone());
    let static_dir = static_dir();
    let app = router(App::new(acc, &Config::from_env()), &static_dir);
    let addr = listen_addr();
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| {
            panic!("Không mở được cổng {addr}: {e} (cổng đang bị chương trình khác dùng?)")
        });
    println!(
        "Rubik web chạy tại http://{addr} (database: {db_path}, giao diện: {})",
        static_dir.display()
    );
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .unwrap_or_else(|e| eprintln!("Server dừng do lỗi: {e}"));
}
