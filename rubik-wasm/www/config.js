// Cấu hình tài khoản & bảng xếp hạng (Supabase). Để trống url thì các nút tài khoản/xếp hạng tự ẩn.
// Lấy 2 giá trị dưới trong Supabase Dashboard → Project Settings → API (hoặc nút "Connect").
// anonKey (còn gọi là "publishable key") được phép công khai: quyền thật do RLS và các hàm trong supabase/schema.sql quyết định.
window.RUBIK_SUPABASE = {
  url: 'https://eemwrxpgqckqrpvapfoo.supabase.co',          // ví dụ 'https://abcdxyz.supabase.co'
  anonKey: 'sb_publishable_G26kDWFwcOcFMosDqiVjog_cAHI529I',      // chuỗi dài bắt đầu bằng 'eyJ...' hoặc 'sb_publishable_...'
  emailDomain: 'rubik.local', // đuôi email nội bộ ghép sau tên tài khoản; đổi nếu Supabase báo email không hợp lệ
};
