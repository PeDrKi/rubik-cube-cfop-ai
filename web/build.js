// Sinh 2 bản giao diện (một file HTML duy nhất mỗi bản) từ các nguồn chung trong web/src/:
//   node web/build.js          -> ghi file
//   node web/build.js --check  -> chỉ kiểm tra file đã cập nhật chưa (dùng trong CI)
//
// Nguồn:   web/src/index.html   khung HTML (các hộp thoại, thanh công cụ)
//          web/src/style.css    toàn bộ CSS
//          web/src/app.js       toàn bộ JavaScript
//          web/backend.<server|wasm>.txt   phần khác nhau giữa 2 bản (cách gọi server / bộ giải wasm, tài khoản)
//          web/move_table.json  bảng hoán vị nước đi, sinh từ rubik-core
//          web/fonts, web/icons, web/manifest.webmanifest, web/sw.js   tài nguyên chép nguyên xi
// Chỗ khác nhau giữa 2 bản được đánh dấu bằng dòng `<!--@@SLOTn@@-->` (trong HTML) hoặc `//@@SLOTn@@` (trong JS).
// Sửa nguồn xong chạy lại lệnh trên; định dạng mã bằng `npx prettier --write web/src` (cấu hình ở web/.prettierrc.json).
const fs = require('fs'),
  path = require('path');
const root = path.join(__dirname, '..');
const read = (...p) => fs.readFileSync(path.join(__dirname, ...p), 'utf8');
const check = process.argv.includes('--check');

// Bảng hoán vị nước đi (xem rubik-core/examples/gen_move_table.rs) chèn thành một dòng JSON gọn.
const moveTable = JSON.stringify(JSON.parse(read('move_table.json')));
const style = read('src', 'style.css').replace(/\n$/, '');
const script = read('src', 'app.js')
  .replace(/\n$/, '')
  .replace(/\/\*@@MOVE_TABLE@@\*\/\s*\{\}/, () => moveTable);
const shell = read('src', 'index.html')
  .replace(/\n$/, '')
  .replace('<!--@@STYLE@@-->', () => style)
  .replace('<!--@@SCRIPT@@-->', () => script);
const targets = { server: 'rubik-web/static/index.html', wasm: 'rubik-wasm/www/index.html' };

function slots(name) {
  const txt = read(`backend.${name}.txt`).replace(/\n$/, '');
  const m = {};
  let cur = null;
  for (const line of txt.split('\n')) {
    const h = line.match(/^@@SLOT(\d+)@@$/);
    if (h) {
      cur = h[1];
      m[cur] = [];
    } else m[cur].push(line);
  }
  return m;
}

// Tài nguyên tĩnh dùng chung được chép nguyên xi vào cả hai bản; service worker chỉ có ở bản wasm.
const assets = { server: ['fonts', 'icons', 'manifest.webmanifest'], wasm: ['fonts', 'icons', 'manifest.webmanifest', 'sw.js'] };
const outDir = { server: 'rubik-web/static', wasm: 'rubik-wasm/www' };
function filesOf(rel) {
  const full = path.join(__dirname, rel);
  return fs.statSync(full).isDirectory() ? fs.readdirSync(full).sort().flatMap(f => filesOf(path.join(rel, f))) : [rel];
}

let stale = false;
function emit(out, data) {
  const file = path.join(root, out);
  if (check) {
    if (!fs.existsSync(file) || !fs.readFileSync(file).equals(Buffer.from(data))) {
      console.error(`CŨ: ${out} (chạy: node web/build.js)`);
      stale = true;
    }
  } else {
    fs.mkdirSync(path.dirname(file), { recursive: true });
    fs.writeFileSync(file, data);
  }
}

for (const [name, out] of Object.entries(targets)) {
  const s = slots(name);
  const lines = [];
  for (const line of shell.split('\n')) {
    const h = line.match(/^(?:<!--@@SLOT(\d+)@@-->|\/\/@@SLOT(\d+)@@)$/);
    if (!h) {
      lines.push(line);
      continue;
    }
    const id = h[1] || h[2],
      body = s[id];
    if (body === undefined) throw new Error(`backend.${name}.txt thiếu SLOT${id}`);
    // Khe rỗng (một dòng trống duy nhất) nghĩa là bản này không có đoạn đó.
    if (!(body.length === 1 && body[0] === '')) lines.push(...body);
  }
  emit(out, lines.join('\n'));
  if (!check) console.log('đã ghi ' + out);
}
for (const [name, list] of Object.entries(assets)) {
  const files = list.flatMap(filesOf);
  for (const rel of files) emit(path.join(outDir[name], rel), fs.readFileSync(path.join(__dirname, rel)));
  if (!check) console.log(`đã chép ${files.length} file tĩnh vào ${outDir[name]}`);
}
if (stale) process.exit(1);
