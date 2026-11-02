//! rubik-wasm: lớp mỏng bọc `rubik-api` cho trình duyệt. Trả JSON `{ok:true,data}` hoặc `{ok:false,error}`.
use serde_json::{json, Value};
use wasm_bindgen::prelude::*;

fn envelope(r: Result<Value, String>) -> String {
    match r {
        Ok(data) => json!({ "ok": true, "data": data }).to_string(),
        Err(error) => json!({ "ok": false, "error": error }).to_string(),
    }
}

#[wasm_bindgen]
pub fn call(route_name: &str, body: &str) -> String {
    envelope(rubik_api::route(route_name, body))
}

/// `rgba` là dữ liệu ImageData (w*h*4 byte) lấy từ canvas.
#[wasm_bindgen]
pub fn scan_frame(w: u32, h: u32, rgba: &[u8], region_scale: f32) -> String {
    envelope(rubik_api::scan_frame(w, h, rgba, region_scale))
}
