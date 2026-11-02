/* Service worker của bản GitHub Pages: mở được khi không có mạng và cài được lên màn hình điện thoại (PWA).
   - Cài đặt: nạp sẵn vỏ ứng dụng (trang, worker, bộ giải WebAssembly, font, biểu tượng).
   - Mã chương trình (trang, config.js, worker.js, pkg/*.js và pkg/*.wasm): THỬ MẠNG TRƯỚC, hỏng/mất mạng mới dùng bản đã lưu.
     Không dùng "trả bản cũ trước" cho nhóm này vì sau một lần cập nhật có thể ghép nhầm rubik_wasm.js mới với file .wasm cũ
     (hai file phải cùng một bản), khiến bộ giải lỗi. Có mạng thì luôn nhận đúng bộ mới; chỉ khi ngoại tuyến mới dùng bộ đã lưu
     (và bộ đã lưu luôn đủ cặp vì cả hai được ghi lại sau mỗi lần tải thành công).
   - Font, biểu tượng và các file tĩnh khác: trả bản đã lưu ngay, tải bản mới ở nền (stale-while-revalidate).
   - Không đụng tới yêu cầu khác nguồn (Supabase, CDN): tài khoản/xếp hạng cần mạng thật.
   Đổi VERSION khi muốn buộc mọi máy bỏ bản đã lưu. */
const VERSION = 'rubik-v2';
const SHELL = ['./', 'index.html', 'config.js', 'worker.js', 'manifest.webmanifest',
  'pkg/rubik_wasm.js', 'pkg/rubik_wasm_bg.wasm',
  'icons/icon-192.png', 'icons/icon-512.png', 'icons/apple-touch-icon.png',
  'fonts/be-vietnam-pro-latin-400-normal.woff2', 'fonts/be-vietnam-pro-latin-500-normal.woff2', 'fonts/be-vietnam-pro-latin-700-normal.woff2',
  'fonts/be-vietnam-pro-vietnamese-400-normal.woff2', 'fonts/be-vietnam-pro-vietnamese-500-normal.woff2', 'fonts/be-vietnam-pro-vietnamese-700-normal.woff2'];

self.addEventListener('install', (e) => {
  // Từng file riêng: thiếu một file (ví dụ chạy thử chưa build pkg/) không làm hỏng cả lần cài.
  e.waitUntil(caches.open(VERSION).then((c) => Promise.all(SHELL.map((u) => c.add(u).catch(() => {})))).then(() => self.skipWaiting()));
});

self.addEventListener('activate', (e) => {
  e.waitUntil(caches.keys().then((ks) => Promise.all(ks.filter((k) => k !== VERSION).map((k) => caches.delete(k)))).then(() => self.clients.claim()));
});

const isCode = (url) => /(^|\/)(config\.js|worker\.js|pkg\/.*\.(js|wasm))$/.test(url.pathname);
const offline = () => new Response('Ngoại tuyến', { status: 503, statusText: 'Offline' });

self.addEventListener('fetch', (e) => {
  const req = e.request, url = new URL(req.url);
  if (req.method !== 'GET' || url.origin !== location.origin) return;
  if (req.mode === 'navigate') {
    e.respondWith(fetch(req).then((r) => { if (r.ok) caches.open(VERSION).then((c) => c.put('index.html', r.clone())); return r; })
      .catch(() => caches.match('index.html').then((r) => r || caches.match('./'))));
    return;
  }
  if (isCode(url)) {
    // `cache: 'no-cache'` buộc hỏi lại server (không dùng bản trong bộ nhớ đệm HTTP của trình duyệt) để chắc chắn lấy đúng bản mới.
    e.respondWith(fetch(req, { cache: 'no-cache' })
      .then((r) => { if (r.ok) { const copy = r.clone(); caches.open(VERSION).then((c) => c.put(req, copy)); } return r; })
      .catch(() => caches.match(req).then((r) => r || offline())));
    return;
  }
  e.respondWith(caches.open(VERSION).then(async (c) => {
    const hit = await c.match(req);
    const net = fetch(req).then((r) => { if (r.ok) c.put(req, r.clone()); return r; }).catch(() => null);
    return hit || (await net) || offline();
  }));
});
