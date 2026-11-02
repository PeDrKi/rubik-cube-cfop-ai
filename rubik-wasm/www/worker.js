import init, { call, scan_frame } from './pkg/rubik_wasm.js';
// Toàn bộ phép tính nặng chạy ở worker để giao diện không bị đứng.
// Lỗi tải WebAssembly -> báo `fatal`; lỗi khi đang tính (panic Rust = RuntimeError "unreachable") -> trả lỗi cho
// đúng lời gọi đó và báo `crashed` để trang khởi động lại worker (bộ nhớ wasm sau panic không còn tin được).
const ready = init()
  .then(() => { call('warmup', '{}'); postMessage({ ready: true }); })
  .catch((e) => { postMessage({ fatal: 'Không tải được bộ giải WebAssembly: ' + ((e && e.message) || e) + '.' }); throw e; });
const fail = (msg) => JSON.stringify({ ok: false, error: msg });
onmessage = async (e) => {
  const { id, route, body, frame, buf } = e.data;
  try { await ready; } catch (_) { postMessage({ id, out: fail('Bộ giải chưa tải được, hãy tải lại trang.') }); return; }
  try {
    const out = frame ? scan_frame(frame.w, frame.h, new Uint8Array(buf), frame.s) : call(route, JSON.stringify(body || {}));
    postMessage({ id, out });
  } catch (err) {
    postMessage({ id, out: fail('Bộ giải gặp lỗi nội bộ: ' + ((err && err.message) || err)), crashed: true });
  }
};
