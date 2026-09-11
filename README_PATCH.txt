================================================================
patch_v4 — tich hop model v4 (do ca khoi cube) vao RUBIK_rust_app
================================================================

CACH AP DUNG
  1. Giai nen RUBIK_rust_app.zip
  2. Chep de thu muc nay len RUBIK_rust_app/ (giu nguyen cau truc):
        rubik-vision-ml/src/cube.rs          (MOI)
        rubik-vision-ml/src/lib.rs           (SUA: them 2 dong khai bao module)
        rubik-vision-ml/examples/try_cube.rs (MOI)
        rubik-vision-ml/models/v4_cube.onnx  (MOI)
     KHONG dung toi models/face_detector.onnx cu — MlDetector cu giu
     nguyen, rubik-app/src/scan_ui.rs khong phai sua gi de van build duoc.
  3. cd rubik-vision-ml && cargo build

----------------------------------------------------------------
DA KIEM CHUNG THAT (khac voi ban rubik-vision-ml goc)
----------------------------------------------------------------
README goc cua rubik-vision-ml ghi "CHUA build-test duoc". Lan nay da
build va CHAY THAT:

  - cargo 1.95.0, ort 2.0.0-rc.13, ONNX Runtime 1.28.0
  - `cargo build -p rubik-vision-ml`  -> THANH CONG, 0 loi
  - `cargo run --example try_cube -- <anh>` -> chay xong, xuat anh ket qua

  - Doi chieu voi ban tham chieu Python tren cung 1 anh:
    7 diem goc lech nhau 0.08 - 0.30 PIXEL. Tuc logic giai ma
    (v = out*2-0.5), viec don den khi crop tho ra ngoai anh, va vong
    tinh chinh 2 luot ben Rust khop dung ban Python da do dac.

  - `cargo run --release --example merge_check` (KHONG can model):
    chay DUNG doan code gop nhieu khung cua scan_ui.rs tren du lieu tong
    hop -> nhieu +-6 giam con lech 1, 3 mat KHONG bi lan chi so.

  - `cargo run --release --example sample_check` (KHONG can model):
    ve 9 mau biet truoc vao 1 tu giac MEO PHOI CANH roi cho dung duong
    lay mau cua sample_faces doc lai -> LECH 0 tren ca 27 kenh mau, dung
    thu tu row-major. Duong warp_to_square + sample_grid_with_confidence
    va thu tu 4 dinh la CHINH XAC, khong phai "chac la dung".

  - Thu tu 4 dinh trong face_quads() da kiem chung: lay mau 3 mat tu
    NHAN THAT cho ra mau DONG NHAT o ca 3 mat (luoi nam tron trong dung
    mat can lay). Rieng HUONG XOAY cua luoi (mat Top xoay 1 buoc) da
    duoc hieu chuan thong ke tu truoc tren 58 setup co nhan mau that:
    dung -> 93%, sai -> 83%.

LUU Y VE `ort`: build script cua ort-sys tu tai ONNX Runtime tu
cdn.pyke.io. Neu mang cua ban chan CDN do, dat bien moi truong tro toi
mot ban ONNX Runtime co san:
    ORT_PREFER_DYNAMIC_LINK=1
    ORT_LIB_LOCATION=<thu muc chua libonnxruntime.so / onnxruntime.dll>
Phien ban PHAI khop API ma ort rc.13 yeu cau (ONNX Runtime 1.28.x).
Dung 1.25 se build duoc nhung CHAY se panic "requested API version 27".

----------------------------------------------------------------
API MOI
----------------------------------------------------------------
  use rubik_vision_ml::CubeDetector;

  let mut det = CubeDetector::load()?;          // nap 1 LAN luc khoi dong

  let d = det.detect(&img)?;                    // 1 luot  (~1ms)
  let d = det.detect_refined(&img)?;            // 2 luot  (~2ms) <- NEN DUNG

  d.points        // [(f32,f32); 7] = top, tr, br, bot, bl, tl, center
  d.hexagon()     // 6 dinh ngoai, de ve overlay
  d.face_quads()  // [[(f32,f32);4];3] = [Top, Right, Left], moi tu giac
                  // theo thu tu TL,TR,BR,BL — khop warp_to_square
  d.largest_face()      // mat huong ve camera ro nhat (khi chi can 1 mat)
  d.bounding_square(w,h) // chi de ve overlay, DUNG dung de lay mau

  det.sample_faces(&img)?   // ca luong: 2 luot -> 3 tu giac -> hieu chinh
                            // phoi canh -> lay mau luoi 3x3.
                            // Tra [(9 mau, 9 do tin cay); 3]

----------------------------------------------------------------
CON SO — VI SAO DOI SANG v4
----------------------------------------------------------------
Do dau-cuoi (hinh hoc -> luoi 3x3 -> doc mau -> doi chieu nhan that):

  4 dinh THAT (tran)                     97.28%
  hop vuong HOAN HAO (khong qua ML)      38.94%   <<<
  hop vuong do MlDetector cu do           32.94%
  v4: 7 diem cube -> 3 tu giac            72.56%
  v4 + 2 luot                             79.11%
  v4 + 2 luot + bo phieu 6 khung hinh     88.71%

Dong thu hai la mau chot: mat cube nhin nghieng la HINH THOI, nen luoi
3x3 song song truc dat vao hop bao cua no lay mau sai — NGAY CA khi hop
vuong chinh xac tuyet doi. Khong co muc IoU nao cuu duoc cach bieu dien
hop vuong. Do la ly do v4 doi hoan toan muc tieu sang "do ca khoi cube".

----------------------------------------------------------------
DA LAM: PHUONG AN (B) — LUONG QUET 2 LAN CHUP x 3 MAT
----------------------------------------------------------------
File sua them: rubik-app/src/scan_ui.rs  (49.171 -> 60.629 ky tu)

Vi sao (B) chu khong phai (A): v4 duoc huan luyen tren anh THAY 3 MAT
cung luc. Neu giu luong 6 anh (moi anh 1 mat vuong goc voi camera) thi
2 mat kia gan nhu khuat — day la phan bo v4 CHUA TUNG HOC. (B) dua model
ve dung phan bo no da hoc, va nguoi dung chi phai chup 2 lan thay vi 6.

CACH DUNG TRONG APP
  1. Build:  cargo run --features ml_detect
  2. Trong man hinh quet, tich o "Chup 3 mat/lan (model v4)".
  3. LAN 1 — cam cube nhin vao GOC co 3 mat U (tren), F (truoc-TRAI),
     R (phai). Nghieng nhe de thay ro ca 3 mat. Chup.
  4. LAN 2 — xoay sang GOC DOI DIEN: D (nay o tren), B (truoc-TRAI),
     L (phai). Chup.
  5. Bam "Nhan dien trang thai" nhu cu.

  GOP NHIEU KHUNG HINH (tu dong): khi da bat che do 3 mat, app lien tuc
  chay model tren tung khung hinh moi va gom toi da 6 khung. Thanh tien
  do "Dang gom N/6 khung hinh" hien ngay tren man hinh — GIU YEN cube cho
  du 6 khung roi hay chup. Luc chup, 3 mat duoc gop qua cac khung bang
  `aggregate_frames` (gop CO TRONG SO theo do tin cay, dung ham ma luong
  1-mat dang dung).
  Do duoc tren bo anh Bielefeld: 1 khung 85,3% -> 6 khung 88,7% moi o;
  ty le anh doc dung CA 27 O tu 4,8% len 19,0%. Neu cube xe dich giua
  chung, lich su tu dong bi xoa va gom lai tu dau.

GOC XOAY TUNG MAT KHONG CAN DUNG — resolve_scan da tu thu 4^6 = 4096 to
hop xoay. Thu duy nhat tu the cam quyet dinh la DANH TINH mat (mat nao la
U, la F...). Neu nguoi dung xoay theo chieu nguoc lai thi Trai/Phai bi
doi cho -> UI co san nut "⇄ hoan doi lan 1 / lan 2" anh xa lai NGAY,
khong phai chup lai. Va nut "↺ chup lai lan N" neu can.

THIET KE CO Y: KHONG dung vao rubik-vision/src/scan.rs. Toan bo thay doi
nam o tang app. Luong 6 anh cu VAN GIU NGUYEN (bo tich o la ve nhu cu),
nen phan da test ky khong bi anh huong.

DA KIEM CHUNG BUILD (3 cau hinh, deu 0 loi):
  cargo check -p rubik-app                                  -> OK
  cargo check -p rubik-app --features ml_detect             -> OK
  cargo check --workspace --features rubik-app/ml_detect    -> OK
  cargo test -p rubik-core --release   -> 21/21 test PASS
Chi con 1 warning dead_code o theme.rs, co san tu truoc, khong lien quan.

CHUA KIEM CHUNG DUOC (can ban lam tren may that):
  - Chua chay thu voi WEBCAM THAT. Toi khong co camera trong moi truong
    nay. Build sach va logic da doi chieu voi ban Python, nhung hanh vi
    thuc te khi cam cube truoc camera thi ban phai tu thu.
  - Chua do do chinh xac cua luong 2 lan chup tren cube THAT cua ban.
    Con so 79.11% do tren anh Bielefeld, khong phai webcam cua ban.
  - Che do gop nhieu khung moi chi kiem chung bang bien dich va bang
    phep do OFFLINE tren bo anh Bielefeld. Chua chay voi webcam that,
    nen chua biet 6 khung lien tiep tu webcam co du khac nhau de viec
    gop mang lai loi ich nhu tren anh Bielefeld hay khong.

  - Neu lan 2 hay bi doi Trai/Phai, hay bao toi doi mac dinh trong
    TRIPLE_SLOTS thay vi bam nut moi lan.

GIAY PHEP: v4_cube.onnx huan luyen tu bo du lieu Bielefeld
(Wienke et al. 2018, DOI 10.4119/unibi/2930668, ODC-By v1.0). Model la
TAC PHAM PHAI SINH -> bat buoc ghi nhan nguon khi phat hanh.
