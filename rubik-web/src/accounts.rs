//! accounts.rs: tài khoản, phiên đăng nhập, ván xếp hạng và bảng xếp hạng (SQLite).
//!
//! Thành tích chỉ được tin khi do server tự đo: server phát đề xáo, tự bấm giờ
//! lúc phát và lúc nhận lời giải, rồi áp lại toàn bộ nước người chơi gửi lên
//! để chắc chắn khối đã giải xong thật. Trình duyệt không gửi thời gian.

use argon2::password_hash::{
    rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
};
use argon2::Argon2;
use rand::{rngs::StdRng, RngCore, SeedableRng};
use rubik_core::cube::{self, CubeState};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Phiên đăng nhập sống 30 ngày.
pub const SESSION_SECS: i64 = 30 * 24 * 3600;
/// Ván xếp hạng quá 1 giờ thì coi như bỏ dở, không nhận lời giải nữa.
const GAME_MAX_MS: i64 = 3600 * 1000;
/// Số nước xáo của ván xếp hạng.
const SCRAMBLE_LEN: usize = 25;
/// Sai mật khẩu quá số lần này thì khoá đăng nhập tên đó trong LOCK_SECS giây.
const MAX_FAILS: u32 = 5;
const LOCK_SECS: u64 = 60;
/// Chống gian lận: ván giải nhanh hơn mức này (ms) thì không được tính. Không có
/// ngưỡng này, ai cũng nộp ngay chuỗi đảo ngược của đề xáo để có thành tích ~0 giây.
pub const MIN_SOLVE_MS: i64 = 5000;
/// Tốc độ tối đa chấp nhận: mỗi nước tốn ít nhất chừng này ms (10 nước/giây).
pub const MIN_MS_PER_MOVE: i64 = 100;
/// Bảng đếm lần sai quá lớn thì dọn các mục đã hết hạn.
const FAILS_PRUNE_AT: usize = 10_000;
/// Ván bỏ dở/huỷ được giữ lại chừng này rồi xoá hẳn (ván đã giải xong thì giữ mãi, vì bảng xếp hạng cần).
const ABANDONED_KEEP_MS: i64 = 7 * 24 * 3600 * 1000;

/// Kết quả một lần dọn dẹp.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Cleanup {
    /// Ván "đang chơi" quá hạn (người chơi đóng tab) được chuyển thành bỏ dở.
    pub stale_games: usize,
    /// Ván bỏ dở quá cũ bị xoá.
    pub old_games: usize,
    /// Phiên đăng nhập hết hạn bị xoá.
    pub expired_sessions: usize,
}

pub struct Accounts {
    db: Mutex<Connection>,
    /// Đường dẫn file database (để mở kết nối riêng khi sao lưu); ":memory:" nếu chạy trong bộ nhớ.
    path: String,
    fails: Mutex<HashMap<String, (u32, Instant)>>,
}

/// Lỗi trả về cho client: mã HTTP + thông báo tiếng Việt.
#[derive(Debug)]
pub struct ApiErr(pub u16, pub String);

fn err<T>(code: u16, msg: impl Into<String>) -> Result<T, ApiErr> {
    Err(ApiErr(code, msg.into()))
}

impl From<rusqlite::Error> for ApiErr {
    fn from(e: rusqlite::Error) -> Self {
        ApiErr(500, format!("Lỗi database: {e}"))
    }
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn encode(s: &CubeState) -> String {
    s.faces
        .iter()
        .flat_map(|f| f.iter().map(|&v| cube::face_char(v as usize)))
        .collect()
}

fn hash_token(token: &str) -> String {
    Sha256::digest(token.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Hash Argon2 của một mật khẩu bỏ đi, dùng để cân bằng thời gian khi tên đăng nhập không tồn tại.
fn dummy_hash() -> &'static str {
    static H: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    H.get_or_init(|| {
        Argon2::default()
            .hash_password(
                b"khong-ai-dung-mat-khau-nay",
                &SaltString::generate(&mut OsRng),
            )
            .map(|h| h.to_string())
            .unwrap_or_default()
    })
}

fn new_token() -> String {
    let mut b = [0u8; 32];
    OsRng.fill_bytes(&mut b);
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn check_username(name: &str) -> Result<(), ApiErr> {
    let n = name.chars().count();
    if !(3..=20).contains(&n) || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return err(
            400,
            "Tên tài khoản dài 3–20 ký tự, chỉ gồm chữ không dấu, số và dấu _",
        );
    }
    Ok(())
}

fn check_password(pw: &str) -> Result<(), ApiErr> {
    let n = pw.chars().count();
    if !(6..=128).contains(&n) {
        return err(400, "Mật khẩu dài 6–128 ký tự");
    }
    Ok(())
}

impl Accounts {
    pub fn open(path: &str) -> rusqlite::Result<Self> {
        let db = Connection::open(path)?;
        db.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;
             CREATE TABLE IF NOT EXISTS users (
                 id         INTEGER PRIMARY KEY,
                 username   TEXT NOT NULL UNIQUE COLLATE NOCASE,
                 pass_hash  TEXT NOT NULL,
                 created_at INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS sessions (
                 token_hash TEXT PRIMARY KEY,
                 user_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                 expires_at INTEGER NOT NULL
             );
             -- status: 'playing' (đang giải), 'solved' (giải xong, đã kiểm tra), 'abandoned' (bỏ dở/huỷ)
             CREATE TABLE IF NOT EXISTS games (
                 id          INTEGER PRIMARY KEY,
                 user_id     INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                 scramble    TEXT NOT NULL,
                 status      TEXT NOT NULL DEFAULT 'playing',
                 started_at  INTEGER NOT NULL,
                 finished_at INTEGER,
                 time_ms     INTEGER,
                 move_count  INTEGER
             );
             CREATE INDEX IF NOT EXISTS games_user ON games(user_id, status);
             -- Phục vụ việc dọn dẹp định kỳ (tìm ván quá hạn / phiên hết hạn mà không quét cả bảng).
             CREATE INDEX IF NOT EXISTS games_status_started ON games(status, started_at);
             CREATE INDEX IF NOT EXISTS sessions_expires ON sessions(expires_at);",
        )?;
        Ok(Accounts {
            db: Mutex::new(db),
            path: path.to_string(),
            fails: Mutex::new(HashMap::new()),
        })
    }

    /// Dọn dữ liệu cũ. An toàn khi gọi bất kỳ lúc nào (server nên gọi lúc khởi động và định kỳ mỗi giờ):
    /// - ván "đang chơi" quá `GAME_MAX_MS` (người chơi đóng tab giữa chừng) → "bỏ dở";
    /// - ván bỏ dở quá 7 ngày → xoá hẳn;
    /// - phiên đăng nhập hết hạn → xoá. Ván đã giải xong và tài khoản không bao giờ bị đụng tới.
    pub fn cleanup(&self) -> Result<Cleanup, ApiErr> {
        let now = now_ms();
        let db = self.conn();
        let stale_games = db.execute(
            "UPDATE games SET status = 'abandoned' WHERE status = 'playing' AND started_at < ?1",
            [now - GAME_MAX_MS],
        )?;
        let old_games = db.execute(
            "DELETE FROM games WHERE status = 'abandoned' AND started_at < ?1",
            [now - ABANDONED_KEEP_MS],
        )?;
        let expired_sessions =
            db.execute("DELETE FROM sessions WHERE expires_at < ?1", [now / 1000])?;
        Ok(Cleanup {
            stale_games,
            old_games,
            expired_sessions,
        })
    }

    /// Sao lưu database thành một file mới bằng `VACUUM INTO`, KHÔNG cần tắt server: SQLite ở chế độ WAL cho phép
    /// đọc một ảnh chụp nhất quán trong lúc các yêu cầu khác vẫn ghi. Mở kết nối riêng nên không chặn server.
    /// Từ chối ghi đè file đã có. File kết quả là một database SQLite hoàn chỉnh, đã được nén gọn.
    pub fn backup_to(&self, dest: &std::path::Path) -> Result<(), ApiErr> {
        if dest.exists() {
            return err(
                409,
                format!("File {} đã tồn tại, không ghi đè", dest.display()),
            );
        }
        let dest_s = dest
            .to_str()
            .ok_or_else(|| ApiErr(400, "Đường dẫn sao lưu không hợp lệ".into()))?;
        if self.path == ":memory:" {
            self.conn().execute("VACUUM INTO ?1", [dest_s])?;
        } else {
            let c = Connection::open(&self.path)?;
            c.busy_timeout(Duration::from_secs(30))?;
            c.execute("VACUUM INTO ?1", [dest_s])?;
        }
        Ok(())
    }

    fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.db.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn create_session(&self, db: &Connection, user_id: i64) -> Result<String, ApiErr> {
        let token = new_token();
        let now = now_ms() / 1000;
        db.execute("DELETE FROM sessions WHERE expires_at < ?1", [now])?;
        db.execute(
            "INSERT INTO sessions (token_hash, user_id, expires_at) VALUES (?1, ?2, ?3)",
            params![hash_token(&token), user_id, now + SESSION_SECS],
        )?;
        Ok(token)
    }

    /// Trả về (token phiên, thông tin người dùng).
    pub fn register(&self, username: &str, password: &str) -> Result<(String, Value), ApiErr> {
        let username = username.trim();
        check_username(username)?;
        check_password(password)?;
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| ApiErr(500, format!("Không băm được mật khẩu: {e}")))?
            .to_string();
        let db = self.conn();
        let taken: bool = db.query_row(
            "SELECT EXISTS(SELECT 1 FROM users WHERE username = ?1)",
            [username],
            |r| r.get(0),
        )?;
        if taken {
            return err(409, "Tên tài khoản đã có người dùng");
        }
        if let Err(e) = db.execute(
            "INSERT INTO users (username, pass_hash, created_at) VALUES (?1, ?2, ?3)",
            params![username, hash, now_ms()],
        ) {
            return match e {
                rusqlite::Error::SqliteFailure(f, _)
                    if f.code == rusqlite::ErrorCode::ConstraintViolation =>
                {
                    err(409, "Tên tài khoản đã có người dùng")
                }
                e => Err(e.into()),
            };
        }
        let id = db.last_insert_rowid();
        let token = self.create_session(&db, id)?;
        Ok((token, json!({ "id": id, "username": username })))
    }

    /// `ip` dùng để khoá theo cặp (ip, tên): kẻ xấu không khoá được tài khoản người khác từ máy khác.
    pub fn login(
        &self,
        username: &str,
        password: &str,
        ip: &str,
    ) -> Result<(String, Value), ApiErr> {
        let key = format!("{ip}|{}", username.trim().to_lowercase());
        if let Some((n, since)) = self
            .fails
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&key)
        {
            if *n >= MAX_FAILS && since.elapsed() < Duration::from_secs(LOCK_SECS) {
                return err(
                    429,
                    format!("Sai mật khẩu quá nhiều lần, thử lại sau {LOCK_SECS} giây"),
                );
            }
        }
        // Chỉ giữ khoá database lúc truy vấn; băm Argon2 (chậm) làm ở ngoài khoá.
        let row: Option<(i64, String, String)> = {
            let db = self.conn();
            db.query_row(
                "SELECT id, username, pass_hash FROM users WHERE username = ?1",
                [username.trim()],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?
        };
        // Tên không tồn tại vẫn xác minh với một hash giả để thời gian phản hồi không lộ tên nào có thật.
        let hash_str = row
            .as_ref()
            .map(|(_, _, h)| h.as_str())
            .unwrap_or_else(|| dummy_hash());
        let verified = PasswordHash::new(hash_str).is_ok_and(|p| {
            Argon2::default()
                .verify_password(password.as_bytes(), &p)
                .is_ok()
        });
        let ok = verified && row.is_some();
        let mut fails = self.fails.lock().unwrap_or_else(|e| e.into_inner());
        if !ok {
            if fails.len() >= FAILS_PRUNE_AT {
                fails.retain(|_, (_, t)| t.elapsed() < Duration::from_secs(LOCK_SECS));
            }
            let e = fails.entry(key).or_insert((0, Instant::now()));
            if e.1.elapsed() >= Duration::from_secs(LOCK_SECS) {
                *e = (0, Instant::now());
            }
            e.0 += 1;
            e.1 = Instant::now();
            return err(401, "Sai tên tài khoản hoặc mật khẩu");
        }
        fails.remove(&key);
        drop(fails);
        let (id, name, _) = row.unwrap();
        let db = self.conn();
        let token = self.create_session(&db, id)?;
        Ok((token, json!({ "id": id, "username": name })))
    }

    /// Xác minh mật khẩu hiện tại của người đang đăng nhập (dùng cho đổi mật khẩu / xoá tài khoản). Dùng chung cơ chế khoá
    /// tạm theo (ip, tên) với đăng nhập, để kẻ cầm được cookie phiên cũng không dò mật khẩu bằng các route này.
    fn verify_current(&self, user_id: i64, password: &str, ip: &str) -> Result<(), ApiErr> {
        let (name, hash): (String, String) = self
            .conn()
            .query_row(
                "SELECT username, pass_hash FROM users WHERE id = ?1",
                [user_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?
            .ok_or_else(|| ApiErr(401, "Tài khoản không còn tồn tại".into()))?;
        let key = format!("{ip}|{}", name.to_lowercase());
        if let Some((n, since)) = self
            .fails
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&key)
        {
            if *n >= MAX_FAILS && since.elapsed() < Duration::from_secs(LOCK_SECS) {
                return err(
                    429,
                    format!("Sai mật khẩu quá nhiều lần, thử lại sau {LOCK_SECS} giây"),
                );
            }
        }
        let ok = PasswordHash::new(&hash).is_ok_and(|p| {
            Argon2::default()
                .verify_password(password.as_bytes(), &p)
                .is_ok()
        });
        let mut fails = self.fails.lock().unwrap_or_else(|e| e.into_inner());
        if !ok {
            let e = fails.entry(key).or_insert((0, Instant::now()));
            if e.1.elapsed() >= Duration::from_secs(LOCK_SECS) {
                *e = (0, Instant::now());
            }
            e.0 += 1;
            e.1 = Instant::now();
            return err(401, "Mật khẩu hiện tại không đúng");
        }
        fails.remove(&key);
        Ok(())
    }

    /// Đổi mật khẩu: cần mật khẩu cũ. Mọi phiên khác của người này bị đăng xuất (phiên hiện tại `keep_token` được giữ),
    /// để kẻ đã lấy được một phiên cũ không dùng tiếp được sau khi đổi mật khẩu.
    pub fn change_password(
        &self,
        user_id: i64,
        keep_token: &str,
        old: &str,
        new: &str,
        ip: &str,
    ) -> Result<(), ApiErr> {
        check_password(new)?;
        self.verify_current(user_id, old, ip)?;
        let hash = Argon2::default()
            .hash_password(new.as_bytes(), &SaltString::generate(&mut OsRng))
            .map_err(|e| ApiErr(500, format!("Không băm được mật khẩu: {e}")))?
            .to_string();
        let db = self.conn();
        db.execute(
            "UPDATE users SET pass_hash = ?1 WHERE id = ?2",
            params![hash, user_id],
        )?;
        db.execute(
            "DELETE FROM sessions WHERE user_id = ?1 AND token_hash <> ?2",
            params![user_id, hash_token(keep_token)],
        )?;
        Ok(())
    }

    /// Xoá tài khoản vĩnh viễn (cần mật khẩu): xoá luôn phiên đăng nhập và mọi ván của người này, kể cả ván đã giải
    /// (người đó biến mất khỏi bảng xếp hạng). Không hoàn tác được.
    pub fn delete_account(&self, user_id: i64, password: &str, ip: &str) -> Result<(), ApiErr> {
        self.verify_current(user_id, password, ip)?;
        let db = self.conn();
        // foreign_keys = ON nên games/sessions tự xoá theo; xoá tường minh để không phụ thuộc cấu hình kết nối.
        db.execute("DELETE FROM games WHERE user_id = ?1", [user_id])?;
        db.execute("DELETE FROM sessions WHERE user_id = ?1", [user_id])?;
        db.execute("DELETE FROM users WHERE id = ?1", [user_id])?;
        Ok(())
    }

    pub fn logout(&self, token: &str) -> Result<(), ApiErr> {
        self.conn().execute(
            "DELETE FROM sessions WHERE token_hash = ?1",
            [hash_token(token)],
        )?;
        Ok(())
    }

    /// Người dùng của phiên (id, tên), hoặc None nếu phiên không hợp lệ/hết hạn.
    pub fn user_of(&self, token: &str) -> Result<Option<(i64, String)>, ApiErr> {
        Ok(self
            .conn()
            .query_row(
                "SELECT u.id, u.username FROM sessions s JOIN users u ON u.id = s.user_id
                 WHERE s.token_hash = ?1 AND s.expires_at >= ?2",
                params![hash_token(token), now_ms() / 1000],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?)
    }

    /// Bắt đầu ván xếp hạng: huỷ ván đang dở (nếu có), phát đề xáo mới và bấm giờ.
    pub fn start_game(&self, user_id: i64) -> Result<Value, ApiErr> {
        let mut s = CubeState::solved();
        let mut rng = StdRng::seed_from_u64(OsRng.next_u64());
        let moves = cube::random_scramble_moves(&mut s, SCRAMBLE_LEN, &mut rng).join(" ");
        let db = self.conn();
        db.execute(
            "UPDATE games SET status = 'abandoned' WHERE user_id = ?1 AND status = 'playing'",
            [user_id],
        )?;
        db.execute(
            "INSERT INTO games (user_id, scramble, started_at) VALUES (?1, ?2, ?3)",
            params![user_id, moves, now_ms()],
        )?;
        Ok(json!({ "game_id": db.last_insert_rowid(), "scramble": moves, "state": encode(&s) }))
    }

    /// Nhận lời giải: áp lại đề xáo + các nước người chơi gửi, chỉ ghi nhận khi khối đã giải xong.
    pub fn finish_game(&self, user_id: i64, game_id: i64, moves: &str) -> Result<Value, ApiErr> {
        let now = now_ms();
        let db = self.conn();
        let game: Option<(String, String, i64)> = db
            .query_row(
                "SELECT scramble, status, started_at FROM games WHERE id = ?1 AND user_id = ?2",
                params![game_id, user_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        let Some((scramble, status, started)) = game else {
            return err(404, "Không tìm thấy ván này");
        };
        if status != "playing" {
            return err(409, "Ván này đã kết thúc");
        }
        if now - started > GAME_MAX_MS {
            db.execute(
                "UPDATE games SET status = 'abandoned' WHERE id = ?1",
                [game_id],
            )?;
            return err(409, "Ván đã quá 1 giờ nên không được tính");
        }
        let solution = cube::parse_singmaster(moves).map_err(|e| ApiErr(400, e))?;
        if solution.len() > 2000 {
            return err(400, "Lời giải quá dài");
        }
        let mut s = CubeState::solved();
        for m in cube::parse_singmaster(&scramble).map_err(|e| ApiErr(500, e))? {
            s.do_move(&m);
        }
        for m in &solution {
            s.do_move(m);
        }
        if !s.is_solved() {
            return err(400, "Các nước gửi lên chưa giải xong khối");
        }
        let time_ms = now - started;
        let n = solution.len() as i64;
        if time_ms < MIN_SOLVE_MS.max(n * MIN_MS_PER_MOVE) {
            // Nhanh bất khả thi với người thật: huỷ ván để không thể dò ngưỡng bằng cách nộp lại.
            db.execute(
                "UPDATE games SET status = 'abandoned' WHERE id = ?1",
                [game_id],
            )?;
            return err(400, "Thời gian giải nhanh bất thường nên không được tính");
        }
        db.execute(
            "UPDATE games SET status = 'solved', finished_at = ?1, time_ms = ?2, move_count = ?3 WHERE id = ?4",
            params![now, time_ms, n, game_id],
        )?;
        let best: i64 = db.query_row(
            "SELECT MIN(time_ms) FROM games WHERE user_id = ?1 AND status = 'solved'",
            [user_id],
            |r| r.get(0),
        )?;
        Ok(json!({ "time_ms": time_ms, "move_count": n, "personal_best": best == time_ms }))
    }

    pub fn abandon_game(&self, user_id: i64, game_id: i64) -> Result<(), ApiErr> {
        self.conn().execute(
            "UPDATE games SET status = 'abandoned' WHERE id = ?1 AND user_id = ?2 AND status = 'playing'",
            params![game_id, user_id],
        )?;
        Ok(())
    }

    /// Bảng xếp hạng. `by`: "time" (nhanh nhất), "moves" (ít nước nhất), "solves" (giải nhiều nhất).
    pub fn leaderboard(&self, by: &str, limit: i64) -> Result<Value, ApiErr> {
        let order = match by {
            "moves" => "best_moves ASC, best_time ASC",
            "solves" => "solves DESC, best_time ASC",
            _ => "best_time ASC, best_moves ASC",
        };
        let sql = format!(
            "SELECT u.username, COUNT(*) AS solves, MIN(g.time_ms) AS best_time, MIN(g.move_count) AS best_moves,
                    CAST(AVG(g.time_ms) AS INTEGER) AS avg_time
             FROM games g JOIN users u ON u.id = g.user_id
             WHERE g.status = 'solved'
             GROUP BY g.user_id ORDER BY {order}, u.username LIMIT ?1"
        );
        let db = self.conn();
        let mut st = db.prepare(&sql)?;
        let rows = st
            .query_map([limit.clamp(1, 100)], |r| {
                Ok(json!({
                    "username": r.get::<_, String>(0)?,
                    "solves": r.get::<_, i64>(1)?,
                    "best_time_ms": r.get::<_, i64>(2)?,
                    "best_moves": r.get::<_, i64>(3)?,
                    "avg_time_ms": r.get::<_, i64>(4)?,
                }))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let rows: Vec<Value> = rows
            .into_iter()
            .enumerate()
            .map(|(i, mut v)| {
                v["rank"] = json!(i + 1);
                v
            })
            .collect();
        Ok(json!({ "by": by, "rows": rows }))
    }

    /// Hồ sơ: số liệu tổng hợp + 10 ván gần nhất.
    pub fn profile(&self, user_id: i64, username: &str) -> Result<Value, ApiErr> {
        let db = self.conn();
        let (started, solves, best_time, best_moves, avg_time): (
            i64,
            i64,
            Option<i64>,
            Option<i64>,
            Option<i64>,
        ) = db.query_row(
            "SELECT COUNT(*), COUNT(CASE WHEN status = 'solved' THEN 1 END),
                    MIN(time_ms), MIN(move_count), CAST(AVG(time_ms) AS INTEGER)
             FROM games WHERE user_id = ?1",
            [user_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )?;
        let mut st = db.prepare(
            "SELECT finished_at, time_ms, move_count FROM games
             WHERE user_id = ?1 AND status = 'solved' ORDER BY finished_at DESC LIMIT 10",
        )?;
        let recent = st
            .query_map([user_id], |r| {
                Ok(json!({ "finished_at": r.get::<_, i64>(0)?, "time_ms": r.get::<_, i64>(1)?, "move_count": r.get::<_, i64>(2)? }))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(json!({
            "username": username,
            "games": started,
            "solves": solves,
            "best_time_ms": best_time,
            "best_moves": best_moves,
            "avg_time_ms": avg_time,
            "recent": recent,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mem() -> Accounts {
        Accounts::open(":memory:").unwrap()
    }

    /// Lùi giờ bắt đầu ván để giả lập người chơi đã chơi đủ lâu.
    fn backdate(a: &Accounts, game_id: i64, ms: i64) {
        a.conn()
            .execute(
                "UPDATE games SET started_at = started_at - ?1 WHERE id = ?2",
                params![ms, game_id],
            )
            .unwrap();
    }

    fn inverse(seq: &str) -> String {
        seq.split_whitespace()
            .rev()
            .map(|m| {
                if m.ends_with('\'') {
                    m.trim_end_matches('\'').to_string()
                } else if m.ends_with('2') {
                    m.to_string()
                } else {
                    format!("{m}'")
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn dang_ky_dang_nhap_dang_xuat() {
        let a = mem();
        let (tok, u) = a.register("lien_01", "matkhau1").unwrap();
        assert_eq!(u["username"], "lien_01");
        assert_eq!(a.user_of(&tok).unwrap().unwrap().1, "lien_01");
        assert_eq!(
            a.register("LIEN_01", "khac123").err().unwrap().0,
            409,
            "trùng tên không phân biệt hoa thường"
        );
        assert_eq!(a.login("lien_01", "sai", "ip1").err().unwrap().0, 401);
        let (tok2, _) = a.login("lien_01", "matkhau1", "ip1").unwrap();
        a.logout(&tok2).unwrap();
        assert!(a.user_of(&tok2).unwrap().is_none());
        assert!(
            a.user_of(&tok).unwrap().is_some(),
            "đăng xuất phiên này không ảnh hưởng phiên khác"
        );
    }

    #[test]
    fn tu_choi_ten_va_mat_khau_khong_hop_le() {
        let a = mem();
        assert_eq!(a.register("ab", "matkhau1").err().unwrap().0, 400);
        assert_eq!(a.register("có dấu", "matkhau1").err().unwrap().0, 400);
        assert_eq!(a.register("hople", "123").err().unwrap().0, 400);
    }

    #[test]
    fn khoa_tam_sau_nhieu_lan_sai() {
        let a = mem();
        a.register("nguoichoi", "matkhau1").unwrap();
        for _ in 0..MAX_FAILS {
            assert_eq!(a.login("nguoichoi", "sai", "ip1").err().unwrap().0, 401);
        }
        assert_eq!(
            a.login("nguoichoi", "matkhau1", "ip1").err().unwrap().0,
            429
        );
    }

    #[test]
    fn van_xep_hang_chi_tinh_khi_giai_xong_that() {
        let a = mem();
        let (tok, _) = a.register("nguoichoi", "matkhau1").unwrap();
        let uid = a.user_of(&tok).unwrap().unwrap().0;
        let g = a.start_game(uid).unwrap();
        let id = g["game_id"].as_i64().unwrap();
        let scramble = g["scramble"].as_str().unwrap();
        backdate(&a, id, 60_000);
        assert_eq!(
            a.finish_game(uid, id, "R U").err().unwrap().0,
            400,
            "chưa giải xong thì không ghi nhận"
        );
        let r = a.finish_game(uid, id, &inverse(scramble)).unwrap();
        assert_eq!(r["move_count"], 25);
        assert_eq!(
            a.finish_game(uid, id, &inverse(scramble)).err().unwrap().0,
            409,
            "không nộp lại được"
        );

        let (tok2, _) = a.register("nguoikhac", "matkhau1").unwrap();
        let uid2 = a.user_of(&tok2).unwrap().unwrap().0;
        assert_eq!(
            a.finish_game(uid2, id, &inverse(scramble)).err().unwrap().0,
            404,
            "không nộp hộ ván người khác"
        );

        let lb = a.leaderboard("time", 10).unwrap();
        assert_eq!(lb["rows"].as_array().unwrap().len(), 1);
        assert_eq!(lb["rows"][0]["username"], "nguoichoi");
        assert_eq!(lb["rows"][0]["rank"], 1);
        let p = a.profile(uid, "nguoichoi").unwrap();
        assert_eq!(p["solves"], 1);
        assert_eq!(p["games"], 1);
    }

    #[test]
    fn bat_dau_van_moi_thi_huy_van_dang_do() {
        let a = mem();
        let (tok, _) = a.register("nguoichoi", "matkhau1").unwrap();
        let uid = a.user_of(&tok).unwrap().unwrap().0;
        let g1 = a.start_game(uid).unwrap();
        a.start_game(uid).unwrap();
        let s = g1["scramble"].as_str().unwrap();
        assert_eq!(
            a.finish_game(uid, g1["game_id"].as_i64().unwrap(), &inverse(s))
                .err()
                .unwrap()
                .0,
            409
        );
    }

    #[test]
    fn nop_ngay_chuoi_dao_nguoc_de_xao_bi_tu_choi() {
        let a = mem();
        let (tok, _) = a.register("nguoichoi", "matkhau1").unwrap();
        let uid = a.user_of(&tok).unwrap().unwrap().0;
        let g = a.start_game(uid).unwrap();
        let id = g["game_id"].as_i64().unwrap();
        let sol = inverse(g["scramble"].as_str().unwrap());
        assert_eq!(
            a.finish_game(uid, id, &sol).err().unwrap().0,
            400,
            "nộp ngay lập tức phải bị từ chối"
        );
        assert_eq!(
            a.finish_game(uid, id, &sol).err().unwrap().0,
            409,
            "ván bị huỷ, không dò ngưỡng được"
        );
        assert_eq!(
            a.leaderboard("time", 10).unwrap()["rows"]
                .as_array()
                .unwrap()
                .len(),
            0
        );
        // Đủ thời gian nhưng tốc độ nước vẫn phải hợp lý: 25 nước cần ≥ 2,5 giây, ngưỡng sàn 5 giây áp dụng.
        let g = a.start_game(uid).unwrap();
        let id = g["game_id"].as_i64().unwrap();
        backdate(&a, id, MIN_SOLVE_MS + 500);
        assert!(a
            .finish_game(uid, id, &inverse(g["scramble"].as_str().unwrap()))
            .is_ok());
    }

    #[test]
    fn khoa_dang_nhap_theo_ip_khong_khoa_nguoi_khac() {
        let a = mem();
        a.register("nan_nhan", "matkhau1").unwrap();
        for _ in 0..MAX_FAILS {
            assert_eq!(a.login("nan_nhan", "sai", "ke_xau").err().unwrap().0, 401);
        }
        assert_eq!(
            a.login("nan_nhan", "matkhau1", "ke_xau").err().unwrap().0,
            429
        );
        assert!(
            a.login("nan_nhan", "matkhau1", "may_that").is_ok(),
            "chủ tài khoản ở IP khác vẫn đăng nhập được"
        );
    }

    #[test]
    fn don_dep_van_bo_do_va_phien_het_han() {
        let a = mem();
        let (tok, _) = a.register("nguoichoi", "matkhau1").unwrap();
        let uid = a.user_of(&tok).unwrap().unwrap().0;
        // Ba ván: đang chơi còn mới, đang chơi đã quá hạn (đóng tab), bỏ dở đã rất cũ.
        let fresh = a.start_game(uid).unwrap()["game_id"].as_i64().unwrap();
        let ins = |status: &str, age_ms: i64| {
            a.conn()
                .execute("INSERT INTO games (user_id, scramble, status, started_at) VALUES (?1, 'R', ?2, ?3)", params![uid, status, now_ms() - age_ms])
                .unwrap();
            a.conn().last_insert_rowid()
        };
        let stale = ins("playing", GAME_MAX_MS + 1000);
        let old = ins("abandoned", ABANDONED_KEEP_MS + 1000);
        let recent_abandoned = ins("abandoned", 1000);
        let solved_old = ins("solved", ABANDONED_KEEP_MS * 10);
        // Một phiên đã hết hạn.
        a.conn()
            .execute(
                "INSERT INTO sessions (token_hash, user_id, expires_at) VALUES ('het_han', ?1, 1)",
                [uid],
            )
            .unwrap();

        let r = a.cleanup().unwrap();
        assert_eq!(
            r,
            Cleanup {
                stale_games: 1,
                old_games: 1,
                expired_sessions: 1
            }
        );
        let status = |id: i64| -> Option<String> {
            a.conn()
                .query_row("SELECT status FROM games WHERE id = ?1", [id], |r| r.get(0))
                .optional()
                .unwrap()
        };
        assert_eq!(
            status(fresh).as_deref(),
            Some("playing"),
            "ván còn mới không bị đụng"
        );
        assert_eq!(
            status(stale).as_deref(),
            Some("abandoned"),
            "ván quá hạn thành bỏ dở"
        );
        assert_eq!(status(old), None, "ván bỏ dở rất cũ bị xoá");
        assert_eq!(status(recent_abandoned).as_deref(), Some("abandoned"));
        assert_eq!(
            status(solved_old).as_deref(),
            Some("solved"),
            "ván đã giải xong giữ mãi"
        );
        assert!(
            a.user_of(&tok).unwrap().is_some(),
            "phiên còn hạn không bị xoá"
        );
        assert_eq!(
            a.cleanup().unwrap(),
            Cleanup::default(),
            "chạy lại lần nữa không còn gì để dọn"
        );
    }

    #[test]
    fn sao_luu_khi_dang_chay_va_khong_ghi_de() {
        let dir = std::env::temp_dir().join(format!(
            "rubik_backup_test_{}_{}",
            std::process::id(),
            now_ms()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("rubik.db");
        let a = Accounts::open(db_path.to_str().unwrap()).unwrap();
        a.register("nguoichoi", "matkhau1").unwrap();
        let out = dir.join("sao_luu.db");
        a.backup_to(&out).unwrap(); // `a` vẫn đang mở: không cần tắt gì cả
        assert_eq!(
            a.backup_to(&out).err().unwrap().0,
            409,
            "không ghi đè bản sao lưu cũ"
        );
        // Ghi thêm sau khi sao lưu, bản sao lưu phải giữ nguyên trạng thái lúc chụp.
        a.register("nguoikhac", "matkhau1").unwrap();
        let copy = Accounts::open(out.to_str().unwrap()).unwrap();
        let n: i64 = copy
            .conn()
            .query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1);
        assert!(
            copy.login("nguoichoi", "matkhau1", "ip").is_ok(),
            "đăng nhập được bằng bản sao lưu"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn doi_mat_khau_can_mat_khau_cu_va_dang_xuat_phien_khac() {
        let a = mem();
        let (tok1, _) = a.register("nguoichoi", "matkhau1").unwrap();
        let (tok2, _) = a.login("nguoichoi", "matkhau1", "ip1").unwrap();
        let uid = a.user_of(&tok1).unwrap().unwrap().0;
        assert_eq!(
            a.change_password(uid, &tok1, "sai_roi", "matkhau2", "ip1")
                .err()
                .unwrap()
                .0,
            401,
            "sai mật khẩu cũ"
        );
        assert_eq!(
            a.change_password(uid, &tok1, "matkhau1", "123", "ip1")
                .err()
                .unwrap()
                .0,
            400,
            "mật khẩu mới quá ngắn"
        );
        a.change_password(uid, &tok1, "matkhau1", "matkhau2", "ip1")
            .unwrap();
        assert!(
            a.user_of(&tok1).unwrap().is_some(),
            "phiên đang dùng được giữ"
        );
        assert!(
            a.user_of(&tok2).unwrap().is_none(),
            "phiên khác bị đăng xuất"
        );
        assert_eq!(
            a.login("nguoichoi", "matkhau1", "ip2").err().unwrap().0,
            401,
            "mật khẩu cũ hết hiệu lực"
        );
        assert!(a.login("nguoichoi", "matkhau2", "ip2").is_ok());
    }

    #[test]
    fn doi_mat_khau_sai_nhieu_lan_bi_khoa_tam() {
        let a = mem();
        let (tok, _) = a.register("nguoichoi", "matkhau1").unwrap();
        let uid = a.user_of(&tok).unwrap().unwrap().0;
        for _ in 0..MAX_FAILS {
            assert_eq!(
                a.change_password(uid, &tok, "sai", "matkhau2", "ip1")
                    .err()
                    .unwrap()
                    .0,
                401
            );
        }
        assert_eq!(
            a.change_password(uid, &tok, "matkhau1", "matkhau2", "ip1")
                .err()
                .unwrap()
                .0,
            429,
            "đúng mật khẩu cũng bị khoá tạm"
        );
        assert_eq!(
            a.login("nguoichoi", "matkhau1", "ip1").err().unwrap().0,
            429,
            "khoá chung với đăng nhập"
        );
    }

    #[test]
    fn xoa_tai_khoan_xoa_sach_du_lieu() {
        let a = mem();
        let (tok, _) = a.register("nguoichoi", "matkhau1").unwrap();
        let uid = a.user_of(&tok).unwrap().unwrap().0;
        let g = a.start_game(uid).unwrap();
        let id = g["game_id"].as_i64().unwrap();
        backdate(&a, id, 60_000);
        a.finish_game(uid, id, &inverse(g["scramble"].as_str().unwrap()))
            .unwrap();
        let (tok_b, _) = a.register("nguoikhac", "matkhau1").unwrap();
        let uid_b = a.user_of(&tok_b).unwrap().unwrap().0;
        assert_eq!(
            a.delete_account(uid, "sai", "ip1").err().unwrap().0,
            401,
            "cần đúng mật khẩu"
        );
        assert_eq!(
            a.leaderboard("time", 10).unwrap()["rows"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        a.delete_account(uid, "matkhau1", "ip1").unwrap();
        assert!(a.user_of(&tok).unwrap().is_none());
        assert_eq!(
            a.login("nguoichoi", "matkhau1", "ip1").err().unwrap().0,
            401
        );
        assert_eq!(
            a.leaderboard("time", 10).unwrap()["rows"]
                .as_array()
                .unwrap()
                .len(),
            0,
            "biến mất khỏi bảng xếp hạng"
        );
        let games: i64 = a
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM games WHERE user_id = ?1",
                [uid],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(games, 0);
        assert!(
            a.user_of(&tok_b).unwrap().is_some(),
            "người khác không bị ảnh hưởng"
        );
        assert!(
            a.register("nguoichoi", "matkhau9").is_ok(),
            "tên được giải phóng, đăng ký lại được"
        );
        let _ = uid_b;
    }
}
