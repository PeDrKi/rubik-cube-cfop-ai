-- =====================================================================
-- Rubik CFOP: tài khoản, ván xếp hạng và bảng xếp hạng trên Supabase.
--
-- Chạy MỘT LẦN trong Supabase Dashboard → SQL Editor (dán toàn bộ file
-- rồi bấm Run). Chạy lại nhiều lần cũng không sao: các lệnh đều kiểm tra
-- "đã có chưa" và bảng hoán vị được ghi đè.
--
-- Vì sao thành tích đáng tin: trình duyệt KHÔNG gửi thời gian. Hàm
-- start_game tự sinh đề xáo và ghi giờ bắt đầu bằng đồng hồ của database;
-- finish_game áp lại đề xáo + các nước người chơi gửi lên bằng bảng hoán
-- vị cube_moves (sinh ra từ chính engine Rust của dự án), chỉ ghi nhận nếu
-- khối giải xong thật, và tự tính thời gian. Bảng games không mở cho
-- client đọc/ghi trực tiếp (RLS bật, không có policy), mọi thứ đi qua hàm.
--
-- Giới hạn còn lại (cần biết): hệ thống chặn được gian lận kiểu nộp tức thì, nhưng không chặn được người
-- dùng chính bộ giải của dự án (hoặc phần mềm khác) để giải thay mình rồi nộp lời giải. Hãy coi bảng xếp
-- hạng là vui, không dùng cho giải thưởng thật.
-- =====================================================================

-- ---------- hồ sơ người chơi ----------
create table if not exists public.profiles (
  id         uuid primary key references auth.users(id) on delete cascade,
  username   text not null check (username ~ '^[A-Za-z0-9_]{3,20}$'),
  created_at timestamptz not null default now()
);
create unique index if not exists profiles_username_key on public.profiles (lower(username));
alter table public.profiles enable row level security;
drop policy if exists "Ai cũng xem được tên người chơi" on public.profiles;
create policy "Ai cũng xem được tên người chơi" on public.profiles for select using (true);

-- Tạo hồ sơ ngay khi đăng ký; tên lấy từ options.data.username lúc signUp.
create or replace function public.handle_new_user() returns trigger
language plpgsql security definer set search_path = public as $$
begin
  if new.raw_user_meta_data->>'username' is null
     or new.raw_user_meta_data->>'username' !~ '^[A-Za-z0-9_]{3,20}$' then
    raise exception 'Tên tài khoản dài 3–20 ký tự, chỉ gồm chữ không dấu, số và dấu _';
  end if;
  insert into public.profiles (id, username) values (new.id, new.raw_user_meta_data->>'username');
  return new;
end $$;
drop trigger if exists on_auth_user_created on auth.users;
create trigger on_auth_user_created after insert on auth.users
  for each row execute function public.handle_new_user();

-- ---------- bảng hoán vị nước đi ----------
-- perm[i] = vị trí ô (1..54) mà ô i lấy màu sau khi đi nước đó.
-- Thứ tự ô: 6 mặt U D F B L R, mỗi mặt 9 ô theo hàng — giống chuỗi trạng thái 54 ký tự của rubik-api.
create table if not exists public.cube_moves (
  token text primary key,
  perm  smallint[] not null check (array_length(perm, 1) = 54)
);
alter table public.cube_moves enable row level security;
truncate public.cube_moves;
insert into public.cube_moves (token, perm) values
('U','{7,4,1,8,5,2,9,6,3,10,11,12,13,14,15,16,17,18,46,47,48,22,23,24,25,26,27,37,38,39,31,32,33,34,35,36,19,20,21,40,41,42,43,44,45,28,29,30,49,50,51,52,53,54}'),
('U''','{3,6,9,2,5,8,1,4,7,10,11,12,13,14,15,16,17,18,37,38,39,22,23,24,25,26,27,46,47,48,31,32,33,34,35,36,28,29,30,40,41,42,43,44,45,19,20,21,49,50,51,52,53,54}'),
('U2','{9,8,7,6,5,4,3,2,1,10,11,12,13,14,15,16,17,18,28,29,30,22,23,24,25,26,27,19,20,21,31,32,33,34,35,36,46,47,48,40,41,42,43,44,45,37,38,39,49,50,51,52,53,54}'),
('U2''','{9,8,7,6,5,4,3,2,1,10,11,12,13,14,15,16,17,18,28,29,30,22,23,24,25,26,27,19,20,21,31,32,33,34,35,36,46,47,48,40,41,42,43,44,45,37,38,39,49,50,51,52,53,54}'),
('D','{1,2,3,4,5,6,7,8,9,16,13,10,17,14,11,18,15,12,19,20,21,22,23,24,43,44,45,28,29,30,31,32,33,52,53,54,37,38,39,40,41,42,34,35,36,46,47,48,49,50,51,25,26,27}'),
('D''','{1,2,3,4,5,6,7,8,9,12,15,18,11,14,17,10,13,16,19,20,21,22,23,24,52,53,54,28,29,30,31,32,33,43,44,45,37,38,39,40,41,42,25,26,27,46,47,48,49,50,51,34,35,36}'),
('D2','{1,2,3,4,5,6,7,8,9,18,17,16,15,14,13,12,11,10,19,20,21,22,23,24,34,35,36,28,29,30,31,32,33,25,26,27,37,38,39,40,41,42,52,53,54,46,47,48,49,50,51,43,44,45}'),
('D2''','{1,2,3,4,5,6,7,8,9,18,17,16,15,14,13,12,11,10,19,20,21,22,23,24,34,35,36,28,29,30,31,32,33,25,26,27,37,38,39,40,41,42,52,53,54,46,47,48,49,50,51,43,44,45}'),
('F','{1,2,3,4,5,6,45,42,39,52,49,46,13,14,15,16,17,18,25,22,19,26,23,20,27,24,21,28,29,30,31,32,33,34,35,36,37,38,10,40,41,11,43,44,12,7,47,48,8,50,51,9,53,54}'),
('F''','{1,2,3,4,5,6,46,49,52,39,42,45,13,14,15,16,17,18,21,24,27,20,23,26,19,22,25,28,29,30,31,32,33,34,35,36,37,38,9,40,41,8,43,44,7,12,47,48,11,50,51,10,53,54}'),
('F2','{1,2,3,4,5,6,12,11,10,9,8,7,13,14,15,16,17,18,27,26,25,24,23,22,21,20,19,28,29,30,31,32,33,34,35,36,37,38,52,40,41,49,43,44,46,45,47,48,42,50,51,39,53,54}'),
('F2''','{1,2,3,4,5,6,12,11,10,9,8,7,13,14,15,16,17,18,27,26,25,24,23,22,21,20,19,28,29,30,31,32,33,34,35,36,37,38,52,40,41,49,43,44,46,45,47,48,42,50,51,39,53,54}'),
('B','{48,51,54,4,5,6,7,8,9,10,11,12,13,14,15,37,40,43,19,20,21,22,23,24,25,26,27,34,31,28,35,32,29,36,33,30,3,38,39,2,41,42,1,44,45,46,47,18,49,50,17,52,53,16}'),
('B''','{43,40,37,4,5,6,7,8,9,10,11,12,13,14,15,54,51,48,19,20,21,22,23,24,25,26,27,30,33,36,29,32,35,28,31,34,16,38,39,17,41,42,18,44,45,46,47,1,49,50,2,52,53,3}'),
('B2','{18,17,16,4,5,6,7,8,9,10,11,12,13,14,15,3,2,1,19,20,21,22,23,24,25,26,27,36,35,34,33,32,31,30,29,28,54,38,39,51,41,42,48,44,45,46,47,43,49,50,40,52,53,37}'),
('B2''','{18,17,16,4,5,6,7,8,9,10,11,12,13,14,15,3,2,1,19,20,21,22,23,24,25,26,27,36,35,34,33,32,31,30,29,28,54,38,39,51,41,42,48,44,45,46,47,43,49,50,40,52,53,37}'),
('L','{36,2,3,33,5,6,30,8,9,19,11,12,22,14,15,25,17,18,1,20,21,4,23,24,7,26,27,28,29,16,31,32,13,34,35,10,43,40,37,44,41,38,45,42,39,46,47,48,49,50,51,52,53,54}'),
('L''','{19,2,3,22,5,6,25,8,9,36,11,12,33,14,15,30,17,18,10,20,21,13,23,24,16,26,27,28,29,7,31,32,4,34,35,1,39,42,45,38,41,44,37,40,43,46,47,48,49,50,51,52,53,54}'),
('L2','{10,2,3,13,5,6,16,8,9,1,11,12,4,14,15,7,17,18,36,20,21,33,23,24,30,26,27,28,29,25,31,32,22,34,35,19,45,44,43,42,41,40,39,38,37,46,47,48,49,50,51,52,53,54}'),
('L2''','{10,2,3,13,5,6,16,8,9,1,11,12,4,14,15,7,17,18,36,20,21,33,23,24,30,26,27,28,29,25,31,32,22,34,35,19,45,44,43,42,41,40,39,38,37,46,47,48,49,50,51,52,53,54}'),
('R','{1,2,21,4,5,24,7,8,27,10,11,34,13,14,31,16,17,28,19,20,12,22,23,15,25,26,18,9,29,30,6,32,33,3,35,36,37,38,39,40,41,42,43,44,45,52,49,46,53,50,47,54,51,48}'),
('R''','{1,2,34,4,5,31,7,8,28,10,11,21,13,14,24,16,17,27,19,20,3,22,23,6,25,26,9,18,29,30,15,32,33,12,35,36,37,38,39,40,41,42,43,44,45,48,51,54,47,50,53,46,49,52}'),
('R2','{1,2,12,4,5,15,7,8,18,10,11,3,13,14,6,16,17,9,19,20,34,22,23,31,25,26,28,27,29,30,24,32,33,21,35,36,37,38,39,40,41,42,43,44,45,54,53,52,51,50,49,48,47,46}'),
('R2''','{1,2,12,4,5,15,7,8,18,10,11,3,13,14,6,16,17,9,19,20,34,22,23,31,25,26,28,27,29,30,24,32,33,21,35,36,37,38,39,40,41,42,43,44,45,54,53,52,51,50,49,48,47,46}'),
('u','{7,4,1,8,5,2,9,6,3,10,11,12,13,14,15,16,17,18,46,47,48,49,50,51,25,26,27,37,38,39,40,41,42,34,35,36,19,20,21,22,23,24,43,44,45,28,29,30,31,32,33,52,53,54}'),
('u''','{3,6,9,2,5,8,1,4,7,10,11,12,13,14,15,16,17,18,37,38,39,40,41,42,25,26,27,46,47,48,49,50,51,34,35,36,28,29,30,31,32,33,43,44,45,19,20,21,22,23,24,52,53,54}'),
('u2','{9,8,7,6,5,4,3,2,1,10,11,12,13,14,15,16,17,18,28,29,30,31,32,33,25,26,27,19,20,21,22,23,24,34,35,36,46,47,48,49,50,51,43,44,45,37,38,39,40,41,42,52,53,54}'),
('u2''','{9,8,7,6,5,4,3,2,1,10,11,12,13,14,15,16,17,18,28,29,30,31,32,33,25,26,27,19,20,21,22,23,24,34,35,36,46,47,48,49,50,51,43,44,45,37,38,39,40,41,42,52,53,54}'),
('d','{1,2,3,4,5,6,7,8,9,16,13,10,17,14,11,18,15,12,19,20,21,40,41,42,43,44,45,28,29,30,49,50,51,52,53,54,37,38,39,31,32,33,34,35,36,46,47,48,22,23,24,25,26,27}'),
('d''','{1,2,3,4,5,6,7,8,9,12,15,18,11,14,17,10,13,16,19,20,21,49,50,51,52,53,54,28,29,30,40,41,42,43,44,45,37,38,39,22,23,24,25,26,27,46,47,48,31,32,33,34,35,36}'),
('d2','{1,2,3,4,5,6,7,8,9,18,17,16,15,14,13,12,11,10,19,20,21,31,32,33,34,35,36,28,29,30,22,23,24,25,26,27,37,38,39,49,50,51,52,53,54,46,47,48,40,41,42,43,44,45}'),
('d2''','{1,2,3,4,5,6,7,8,9,18,17,16,15,14,13,12,11,10,19,20,21,31,32,33,34,35,36,28,29,30,22,23,24,25,26,27,37,38,39,49,50,51,52,53,54,46,47,48,40,41,42,43,44,45}'),
('f','{1,2,3,44,41,38,45,42,39,52,49,46,53,50,47,16,17,18,25,22,19,26,23,20,27,24,21,28,29,30,31,32,33,34,35,36,37,13,10,40,14,11,43,15,12,7,4,48,8,5,51,9,6,54}'),
('f''','{1,2,3,47,50,53,46,49,52,39,42,45,38,41,44,16,17,18,21,24,27,20,23,26,19,22,25,28,29,30,31,32,33,34,35,36,37,6,9,40,5,8,43,4,7,12,15,48,11,14,51,10,13,54}'),
('f2','{1,2,3,15,14,13,12,11,10,9,8,7,6,5,4,16,17,18,27,26,25,24,23,22,21,20,19,28,29,30,31,32,33,34,35,36,37,53,52,40,50,49,43,47,46,45,44,48,42,41,51,39,38,54}'),
('f2''','{1,2,3,15,14,13,12,11,10,9,8,7,6,5,4,16,17,18,27,26,25,24,23,22,21,20,19,28,29,30,31,32,33,34,35,36,37,53,52,40,50,49,43,47,46,45,44,48,42,41,51,39,38,54}'),
('b','{48,51,54,47,50,53,7,8,9,10,11,12,38,41,44,37,40,43,19,20,21,22,23,24,25,26,27,34,31,28,35,32,29,36,33,30,3,6,39,2,5,42,1,4,45,46,15,18,49,14,17,52,13,16}'),
('b''','{43,40,37,44,41,38,7,8,9,10,11,12,53,50,47,54,51,48,19,20,21,22,23,24,25,26,27,30,33,36,29,32,35,28,31,34,16,13,39,17,14,42,18,15,45,46,4,1,49,5,2,52,6,3}'),
('b2','{18,17,16,15,14,13,7,8,9,10,11,12,6,5,4,3,2,1,19,20,21,22,23,24,25,26,27,36,35,34,33,32,31,30,29,28,54,53,39,51,50,42,48,47,45,46,44,43,49,41,40,52,38,37}'),
('b2''','{18,17,16,15,14,13,7,8,9,10,11,12,6,5,4,3,2,1,19,20,21,22,23,24,25,26,27,36,35,34,33,32,31,30,29,28,54,53,39,51,50,42,48,47,45,46,44,43,49,41,40,52,38,37}'),
('l','{36,35,3,33,32,6,30,29,9,19,20,12,22,23,15,25,26,18,1,2,21,4,5,24,7,8,27,28,17,16,31,14,13,34,11,10,43,40,37,44,41,38,45,42,39,46,47,48,49,50,51,52,53,54}'),
('l''','{19,20,3,22,23,6,25,26,9,36,35,12,33,32,15,30,29,18,10,11,21,13,14,24,16,17,27,28,8,7,31,5,4,34,2,1,39,42,45,38,41,44,37,40,43,46,47,48,49,50,51,52,53,54}'),
('l2','{10,11,3,13,14,6,16,17,9,1,2,12,4,5,15,7,8,18,36,35,21,33,32,24,30,29,27,28,26,25,31,23,22,34,20,19,45,44,43,42,41,40,39,38,37,46,47,48,49,50,51,52,53,54}'),
('l2''','{10,11,3,13,14,6,16,17,9,1,2,12,4,5,15,7,8,18,36,35,21,33,32,24,30,29,27,28,26,25,31,23,22,34,20,19,45,44,43,42,41,40,39,38,37,46,47,48,49,50,51,52,53,54}'),
('r','{1,20,21,4,23,24,7,26,27,10,35,34,13,32,31,16,29,28,19,11,12,22,14,15,25,17,18,9,8,30,6,5,33,3,2,36,37,38,39,40,41,42,43,44,45,52,49,46,53,50,47,54,51,48}'),
('r''','{1,35,34,4,32,31,7,29,28,10,20,21,13,23,24,16,26,27,19,2,3,22,5,6,25,8,9,18,17,30,15,14,33,12,11,36,37,38,39,40,41,42,43,44,45,48,51,54,47,50,53,46,49,52}'),
('r2','{1,11,12,4,14,15,7,17,18,10,2,3,13,5,6,16,8,9,19,35,34,22,32,31,25,29,28,27,26,30,24,23,33,21,20,36,37,38,39,40,41,42,43,44,45,54,53,52,51,50,49,48,47,46}'),
('r2''','{1,11,12,4,14,15,7,17,18,10,2,3,13,5,6,16,8,9,19,35,34,22,32,31,25,29,28,27,26,30,24,23,33,21,20,36,37,38,39,40,41,42,43,44,45,54,53,52,51,50,49,48,47,46}'),
('M','{1,35,3,4,32,6,7,29,9,10,20,12,13,23,15,16,26,18,19,2,21,22,5,24,25,8,27,28,17,30,31,14,33,34,11,36,37,38,39,40,41,42,43,44,45,46,47,48,49,50,51,52,53,54}'),
('M''','{1,20,3,4,23,6,7,26,9,10,35,12,13,32,15,16,29,18,19,11,21,22,14,24,25,17,27,28,8,30,31,5,33,34,2,36,37,38,39,40,41,42,43,44,45,46,47,48,49,50,51,52,53,54}'),
('M2','{1,11,3,4,14,6,7,17,9,10,2,12,13,5,15,16,8,18,19,35,21,22,32,24,25,29,27,28,26,30,31,23,33,34,20,36,37,38,39,40,41,42,43,44,45,46,47,48,49,50,51,52,53,54}'),
('M2''','{1,11,3,4,14,6,7,17,9,10,2,12,13,5,15,16,8,18,19,35,21,22,32,24,25,29,27,28,26,30,31,23,33,34,20,36,37,38,39,40,41,42,43,44,45,46,47,48,49,50,51,52,53,54}'),
('E','{1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,40,41,42,25,26,27,28,29,30,49,50,51,34,35,36,37,38,39,31,32,33,43,44,45,46,47,48,22,23,24,52,53,54}'),
('E''','{1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,49,50,51,25,26,27,28,29,30,40,41,42,34,35,36,37,38,39,22,23,24,43,44,45,46,47,48,31,32,33,52,53,54}'),
('E2','{1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,31,32,33,25,26,27,28,29,30,22,23,24,34,35,36,37,38,39,49,50,51,43,44,45,46,47,48,40,41,42,52,53,54}'),
('E2''','{1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,31,32,33,25,26,27,28,29,30,22,23,24,34,35,36,37,38,39,49,50,51,43,44,45,46,47,48,40,41,42,52,53,54}'),
('S','{1,2,3,44,41,38,7,8,9,10,11,12,53,50,47,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32,33,34,35,36,37,13,39,40,14,42,43,15,45,46,4,48,49,5,51,52,6,54}'),
('S''','{1,2,3,47,50,53,7,8,9,10,11,12,38,41,44,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32,33,34,35,36,37,6,39,40,5,42,43,4,45,46,15,48,49,14,51,52,13,54}'),
('S2','{1,2,3,15,14,13,7,8,9,10,11,12,6,5,4,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32,33,34,35,36,37,53,39,40,50,42,43,47,45,46,44,48,49,41,51,52,38,54}'),
('S2''','{1,2,3,15,14,13,7,8,9,10,11,12,6,5,4,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32,33,34,35,36,37,53,39,40,50,42,43,47,45,46,44,48,49,41,51,52,38,54}'),
('x','{19,20,21,22,23,24,25,26,27,36,35,34,33,32,31,30,29,28,10,11,12,13,14,15,16,17,18,9,8,7,6,5,4,3,2,1,39,42,45,38,41,44,37,40,43,52,49,46,53,50,47,54,51,48}'),
('x''','{36,35,34,33,32,31,30,29,28,19,20,21,22,23,24,25,26,27,1,2,3,4,5,6,7,8,9,18,17,16,15,14,13,12,11,10,43,40,37,44,41,38,45,42,39,48,51,54,47,50,53,46,49,52}'),
('x2','{10,11,12,13,14,15,16,17,18,1,2,3,4,5,6,7,8,9,36,35,34,33,32,31,30,29,28,27,26,25,24,23,22,21,20,19,45,44,43,42,41,40,39,38,37,54,53,52,51,50,49,48,47,46}'),
('x2''','{10,11,12,13,14,15,16,17,18,1,2,3,4,5,6,7,8,9,36,35,34,33,32,31,30,29,28,27,26,25,24,23,22,21,20,19,45,44,43,42,41,40,39,38,37,54,53,52,51,50,49,48,47,46}'),
('y','{7,4,1,8,5,2,9,6,3,12,15,18,11,14,17,10,13,16,46,47,48,49,50,51,52,53,54,37,38,39,40,41,42,43,44,45,19,20,21,22,23,24,25,26,27,28,29,30,31,32,33,34,35,36}'),
('y''','{3,6,9,2,5,8,1,4,7,16,13,10,17,14,11,18,15,12,37,38,39,40,41,42,43,44,45,46,47,48,49,50,51,52,53,54,28,29,30,31,32,33,34,35,36,19,20,21,22,23,24,25,26,27}'),
('y2','{9,8,7,6,5,4,3,2,1,18,17,16,15,14,13,12,11,10,28,29,30,31,32,33,34,35,36,19,20,21,22,23,24,25,26,27,46,47,48,49,50,51,52,53,54,37,38,39,40,41,42,43,44,45}'),
('y2''','{9,8,7,6,5,4,3,2,1,18,17,16,15,14,13,12,11,10,28,29,30,31,32,33,34,35,36,19,20,21,22,23,24,25,26,27,46,47,48,49,50,51,52,53,54,37,38,39,40,41,42,43,44,45}'),
('z','{43,40,37,44,41,38,45,42,39,52,49,46,53,50,47,54,51,48,25,22,19,26,23,20,27,24,21,30,33,36,29,32,35,28,31,34,16,13,10,17,14,11,18,15,12,7,4,1,8,5,2,9,6,3}'),
('z''','{48,51,54,47,50,53,46,49,52,39,42,45,38,41,44,37,40,43,21,24,27,20,23,26,19,22,25,34,31,28,35,32,29,36,33,30,3,6,9,2,5,8,1,4,7,12,15,18,11,14,17,10,13,16}'),
('z2','{18,17,16,15,14,13,12,11,10,9,8,7,6,5,4,3,2,1,27,26,25,24,23,22,21,20,19,36,35,34,33,32,31,30,29,28,54,53,52,51,50,49,48,47,46,45,44,43,42,41,40,39,38,37}'),
('z2''','{18,17,16,15,14,13,12,11,10,9,8,7,6,5,4,3,2,1,27,26,25,24,23,22,21,20,19,36,35,34,33,32,31,30,29,28,54,53,52,51,50,49,48,47,46,45,44,43,42,41,40,39,38,37}');

-- ---------- ván chơi ----------
create table if not exists public.games (
  id          bigint generated always as identity primary key,
  user_id     uuid not null references auth.users(id) on delete cascade,
  scramble    text not null,
  status      text not null default 'playing' check (status in ('playing', 'solved', 'abandoned')),
  started_at  timestamptz not null default now(),
  finished_at timestamptz,
  time_ms     integer,
  move_count  integer
);
create index if not exists games_user_status on public.games (user_id, status);
alter table public.games enable row level security;

-- ---------- hàm nội bộ ----------
create or replace function public.cube_apply(state text[], moves text[]) returns text[]
language plpgsql stable set search_path = public as $$
declare
  m text; p smallint[]; nxt text[]; i int;
begin
  foreach m in array coalesce(moves, '{}') loop
    select perm into p from cube_moves where token = m;
    if p is null then raise exception 'Nước đi không hợp lệ: %', m; end if;
    nxt := state;
    for i in 1..54 loop nxt[i] := state[p[i]]; end loop;
    state := nxt;
  end loop;
  return state;
end $$;

create or replace function public.cube_solved(state text[]) returns boolean
language sql immutable as $$
  select bool_and(state[f * 9 + k] = state[f * 9 + 1])
  from generate_series(0, 5) f, generate_series(1, 9) k
$$;

create or replace function public.cube_tokens(moves text) returns text[]
language sql immutable as $$
  select coalesce(array_agg(t), '{}') from regexp_split_to_table(trim(coalesce(moves, '')), '\s+') t where t <> ''
$$;

revoke all on function public.cube_apply(text[], text[]) from public, anon, authenticated;
revoke all on function public.cube_solved(text[]) from public, anon, authenticated;
revoke all on function public.cube_tokens(text) from public, anon, authenticated;

-- ---------- hàm cho trình duyệt gọi (supabase.rpc) ----------

-- Tên đã có người dùng chưa (gọi trước khi đăng ký để báo lỗi dễ hiểu).
create or replace function public.username_available(p_username text) returns boolean
language sql stable security definer set search_path = public as $$
  select not exists (select 1 from profiles where lower(username) = lower(trim(p_username)))
$$;

-- Bắt đầu ván xếp hạng: huỷ ván đang dở, sinh đề xáo 25 nước (không hai nước liền cùng mặt), bấm giờ.
create or replace function public.start_game() returns json
language plpgsql volatile security definer set search_path = public as $$
declare
  uid uuid := auth.uid();
  faces text := 'UDFBLR';
  suf text[] := array['', '''', '2'];
  prev text := ''; f text; seq text[] := '{}'; i int;
  solved text[]; st text[]; gid bigint;
begin
  if uid is null then raise exception 'Bạn cần đăng nhập'; end if;
  for i in 1..25 loop
    loop
      f := substr(faces, 1 + floor(random() * 6)::int, 1);
      exit when f <> prev;
    end loop;
    prev := f;
    seq := seq || (f || suf[1 + floor(random() * 3)::int]);
  end loop;
  select array_agg(substr(faces, 1 + (n - 1) / 9, 1) order by n) into solved from generate_series(1, 54) n;
  st := cube_apply(solved, seq);
  update games set status = 'abandoned' where user_id = uid and status = 'playing';
  insert into games (user_id, scramble) values (uid, array_to_string(seq, ' ')) returning id into gid;
  return json_build_object('game_id', gid, 'scramble', array_to_string(seq, ' '), 'state', array_to_string(st, ''));
end $$;

-- Nộp lời giải: chỉ ghi nhận khi các nước gửi lên thật sự giải xong đề xáo; thời gian do database tự đo.
create or replace function public.finish_game(p_game_id bigint, p_moves text) returns json
language plpgsql volatile security definer set search_path = public as $$
declare
  uid uuid := auth.uid();
  g games%rowtype; sol text[]; solved text[]; st text[]; ms int; best int;
begin
  if uid is null then raise exception 'Bạn cần đăng nhập'; end if;
  select * into g from games where id = p_game_id and user_id = uid for update;
  if not found then raise exception 'Không tìm thấy ván này'; end if;
  if g.status <> 'playing' then raise exception 'Ván này đã kết thúc'; end if;
  if now() - g.started_at > interval '1 hour' then
    update games set status = 'abandoned' where id = g.id;
    return json_build_object('error', 'Ván đã quá 1 giờ nên không được tính');
  end if;
  sol := cube_tokens(p_moves);
  if coalesce(array_length(sol, 1), 0) > 2000 then raise exception 'Lời giải quá dài'; end if;
  select array_agg(substr('UDFBLR', 1 + (n - 1) / 9, 1) order by n) into solved from generate_series(1, 54) n;
  st := cube_apply(cube_apply(solved, cube_tokens(g.scramble)), sol);
  if not cube_solved(st) then raise exception 'Các nước gửi lên chưa giải xong khối'; end if;
  ms := (extract(epoch from now() - g.started_at) * 1000)::int;
  -- Chống gian lận: nộp ngay chuỗi đảo ngược của đề xáo sẽ cho thành tích ~0 giây. Tối thiểu 5 giây
  -- và 100 ms/nước (10 nước/giây). Trả JSON lỗi (không raise) để lệnh huỷ ván bên dưới không bị rollback.
  if ms < greatest(5000, coalesce(array_length(sol, 1), 0) * 100) then
    update games set status = 'abandoned' where id = g.id;
    return json_build_object('error', 'Thời gian giải nhanh bất thường nên không được tính');
  end if;
  update games set status = 'solved', finished_at = now(), time_ms = ms,
                   move_count = coalesce(array_length(sol, 1), 0)
   where id = g.id;
  select min(time_ms) into best from games where user_id = uid and status = 'solved';
  return json_build_object('time_ms', ms, 'move_count', coalesce(array_length(sol, 1), 0), 'personal_best', best = ms);
end $$;

create or replace function public.abandon_game(p_game_id bigint) returns void
language sql volatile security definer set search_path = public as $$
  update games set status = 'abandoned' where id = p_game_id and user_id = auth.uid() and status = 'playing'
$$;

-- Bảng xếp hạng: p_by = 'time' (nhanh nhất) | 'moves' (ít nước nhất) | 'solves' (giải nhiều nhất).
create or replace function public.leaderboard(p_by text default 'time', p_limit int default 50)
returns table (rank bigint, username text, solves bigint, best_time_ms int, best_moves int, avg_time_ms int)
language sql stable security definer set search_path = public as $$
  with s as (
    select g.user_id, count(*) as solves, min(g.time_ms) as best_time_ms,
           min(g.move_count) as best_moves, avg(g.time_ms)::int as avg_time_ms
    from games g where g.status = 'solved' group by g.user_id
  )
  select row_number() over (order by
           case when p_by = 'solves' then -s.solves end,
           case when p_by = 'moves' then s.best_moves end,
           s.best_time_ms, s.best_moves, p.username),
         p.username, s.solves, s.best_time_ms, s.best_moves, s.avg_time_ms
  from s join profiles p on p.id = s.user_id
  order by 1
  limit least(greatest(coalesce(p_limit, 50), 1), 100)
$$;

-- Thành tích của người đang đăng nhập + 10 ván gần nhất.
create or replace function public.my_profile() returns json
language sql stable security definer set search_path = public as $$
  select json_build_object(
    'username', (select username from profiles where id = auth.uid()),
    'games', count(*),
    'solves', count(*) filter (where status = 'solved'),
    'best_time_ms', min(time_ms),
    'best_moves', min(move_count),
    'avg_time_ms', avg(time_ms)::int,
    'recent', coalesce((select json_agg(r) from (
        select (extract(epoch from finished_at) * 1000)::bigint as finished_at, time_ms, move_count
        from games where user_id = auth.uid() and status = 'solved'
        order by finished_at desc limit 10) r), '[]'::json))
  from games where user_id = auth.uid()
$$;

revoke all on function public.start_game() from public, anon;
revoke all on function public.finish_game(bigint, text) from public, anon;
revoke all on function public.abandon_game(bigint) from public, anon;
revoke all on function public.my_profile() from public, anon;
grant execute on function public.start_game() to authenticated;
grant execute on function public.finish_game(bigint, text) to authenticated;
grant execute on function public.abandon_game(bigint) to authenticated;
grant execute on function public.my_profile() to authenticated;
grant execute on function public.leaderboard(text, int) to anon, authenticated;
grant execute on function public.username_available(text) to anon, authenticated;

-- ---------- dọn dữ liệu cũ ----------
-- Ván "đang chơi" quá 1 giờ (người chơi đóng tab) → "bỏ dở"; ván bỏ dở quá 7 ngày → xoá. Ván đã giải xong giữ mãi.
-- Hàm KHÔNG mở cho trình duyệt gọi. Chạy tay trong SQL Editor (select public.cleanup_old_games();) hoặc đặt lịch hằng
-- ngày bằng pg_cron (bật ở Database → Extensions):
--   select cron.schedule('rubik-cleanup', '17 3 * * *', 'select public.cleanup_old_games()');
create or replace function public.cleanup_old_games() returns json
language plpgsql volatile security definer set search_path = public as $$
declare stale int; old int;
begin
  update games set status = 'abandoned' where status = 'playing' and started_at < now() - interval '1 hour';
  get diagnostics stale = row_count;
  delete from games where status = 'abandoned' and started_at < now() - interval '7 days';
  get diagnostics old = row_count;
  return json_build_object('stale_games', stale, 'old_games_deleted', old);
end $$;
revoke all on function public.cleanup_old_games() from public, anon, authenticated;
create index if not exists games_status_started on public.games (status, started_at);

-- ---------- xoá tài khoản ----------
-- Người dùng tự xoá tài khoản của mình (trang yêu cầu nhập lại mật khẩu trước khi gọi). Xoá auth.users kéo theo
-- profiles và games (on delete cascade), nên người đó cũng biến mất khỏi bảng xếp hạng. Không hoàn tác được.
create or replace function public.delete_my_account() returns void
language plpgsql volatile security definer set search_path = public, auth as $$
begin
  if auth.uid() is null then raise exception 'Bạn cần đăng nhập'; end if;
  delete from auth.users where id = auth.uid();
end $$;
revoke all on function public.delete_my_account() from public, anon;
grant execute on function public.delete_my_account() to authenticated;
