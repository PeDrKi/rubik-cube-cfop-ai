# Giai đoạn 3 — Thiết kế khảo sát Validation kiểu BotPrize

Mục tiêu: kiểm định độ hợp lệ cấu trúc (construct validity) của HLI bằng
cách xem điểm HLI tự động có tương quan với đánh giá "giống người" của
CFOP-solver thật hay không — đúng phương pháp Hingston (2009) dùng cho
BotPrize (giám khảo chấm bot/người trên thang điểm, không biết nguồn).

## 1. Câu hỏi nghiên cứu của giai đoạn này

- **RQ-V1**: Điểm HLI tự động có tương quan với đánh giá "mức độ giống
  người" của con người thật không? (kiểm định construct validity)
- **RQ-V2** (phụ): Con người có phân biệt được Nhóm B (Kociemba,
  computer-like) với các nhóm còn lại ở tỷ lệ cao hơn ngẫu nhiên không?
  (kiểm tra xem "computer-like" có thực sự bị nhận ra là máy không)
- **RQ-V3** (phụ, khai phá): Thành phần con nào của HLI (Segmentability /
  Pattern-conformity / Trigger-overlap) tương quan MẠNH NHẤT với đánh giá
  con người? (giúp tinh chỉnh trọng số $w_S, w_P, w_O$ ở Giai đoạn 4)

## 2. Đối tượng tham gia (participants)

**Tiêu chí chọn (bắt buộc cả 2):**
1. Biết giải Rubik's Cube bằng CFOP (không nhận người chỉ biết phương
   pháp beginner/layer-by-layer — họ không có "mô hình kỳ vọng" CFOP để
   so sánh).
2. Có kinh nghiệm đọc/viết công thức Singmaster (`R U R' U'`...) — vì
   stimuli trình bày dạng text, không phải video.

**Cỡ mẫu đề xuất:** 15–20 người (mức tối thiểu hợp lý cho 1 nghiên cứu
pilot HCI/HRI có tính tương quan; BotPrize gốc dùng ban giám khảo tương
tự về quy mô cho mỗi lượt đánh giá).

**Tuyển người:** cộng đồng speedcubing địa phương (CLB Rubik trường/nhóm
Facebook/Discord), snowball sampling qua người quen. Không cần trả thù
lao bắt buộc — khảo sát ~15-20 phút.

**Đạo đức nghiên cứu:**
- Có màn hình "Thông tin & Đồng ý tham gia" trước khi bắt đầu (đã tích
  hợp sẵn trong công cụ, xem mục 5).
- Không thu thập thông tin định danh cá nhân (chỉ hỏi: mức kinh nghiệm
  CFOP, thời gian giải trung bình — dùng để lọc/phân nhóm, KHÔNG dùng để
  định danh).
- Người tham gia có thể dừng bất cứ lúc nào, dữ liệu chưa nộp sẽ không
  được lưu.
- Nếu nộp báo cáo/bài báo dùng dữ liệu người, NÊN hỏi phòng NCKH/khoa của
  bạn xem có cần quy trình duyệt đạo đức (ethics review) chính thức
  không — quy định khác nhau tuỳ trường.

## 3. Bộ dữ liệu stimuli (bài giải cho người chấm xem)

**4 nhóm nguồn, mỗi nhóm 6 bài giải** (tổng 24 stimuli):

| Nhóm | Nguồn | Đặc điểm kỳ vọng |
|---|---|---|
| A | AI-CFOP baseline (λ=0) | Cross/F2L "lộn xộn" (search tự do), OLL/PLL có cấu trúc (tra bảng) |
| A+ | AI-CFOP + Trigger-Biased Search (λ=1.0) | Cross/F2L có nhiều trigger quen thuộc hơn A |
| B | Kociemba (computer-like) | Không có cấu trúc CFOP, số nước ngắn nhất |
| H | **Người thật** (cần thu thập riêng — xem mục 4) | Chuẩn đối chiếu thật |

**Trình bày (QUAN TRỌNG — phải ẩn danh đúng cách):**
- Chỉ hiện **chuỗi nước đi thô** (Singmaster), viết liền, **KHÔNG chia
  giai đoạn Cross/F2L/OLL/PLL bằng nhãn hiển thị** — nếu hiện nhãn
  giai đoạn sẽ lộ ngay đây là Nhóm A/A+ (Nhóm B và người thật không có
  nhãn kiểu này), phá vỡ tính mù (blinding). Người biết CFOP tự đọc ra
  cấu trúc được nếu có, đó chính là điều đang muốn đo.
- Nhãn hiển thị cho người chấm chỉ là "Bài giải #17" (số ngẫu nhiên),
  không tiết lộ nhóm nguồn.
- Thứ tự 24 stimuli được xáo trộn ngẫu nhiên **riêng cho từng người**
  (counterbalancing) để tránh hiệu ứng thứ tự hệ thống.

## 4. Nguồn Nhóm H (người thật) — việc BẠN cần tự thu thập

Code không tự tạo được "người thật" — bạn cần 1 trong 2 cách:

**Cách A (khuyến khích, dễ nhất):** xin 6 file log CSTimer từ chính bạn
hoặc bạn bè biết CFOP, export dạng "reconstruction" (CSTimer có tính
năng ghi lại chuỗi nước đi qua Bluetooth cube hoặc nhập tay). Định dạng
cần: 1 chuỗi Singmaster hoàn chỉnh mỗi bài, giải đúng 1 scramble ngẫu
nhiên tương tự các scramble dùng cho Nhóm A/A+/B (nên dùng CÙNG 6
scramble cho cả 4 nhóm để so sánh công bằng theo cặp).

**Cách B (thay thế):** trích từ kho "reconstruction" công khai trên
speedsolving.com hoặc cubing.net (nhiều speedcuber tự đăng lại chuỗi
nước đi các lần giải PB của họ) — nếu dùng, PHẢI ghi rõ nguồn/người
đóng góp trong báo cáo (không phải dữ liệu của bạn thu thập, cần trích
dẫn đúng).

## 5. Công cụ thu thập dữ liệu

Xem `rating_tool.html` (mở trực tiếp bằng trình duyệt, không cần cài gì)
và `prepare_stimuli.py` (sinh file `stimuli.json` làm input cho tool).

Quy trình dùng:
1. Chạy `prepare_stimuli.py` để tự động sinh 18 stimuli (Nhóm A/A+/B,
   6 mỗi nhóm) từ chính solver hiện có.
2. Tự tay bổ sung 6 stimuli Nhóm H vào `stimuli.json` (theo đúng định
   dạng, xem hướng dẫn trong file).
3. Gửi `rating_tool.html` + `stimuli.json` (2 file, để cùng thư mục) cho
   từng người tham gia — họ mở bằng trình duyệt, làm khảo sát, bấm
   "Tải kết quả" cuối cùng để xuất file CSV riêng của họ.
4. Thu thập lại toàn bộ CSV (1 file/người) vào `results/`, chạy
   `analyze_results.py` để tính tương quan.

## 6. Thang đo & câu hỏi cho mỗi stimulus

**Câu hỏi chính (bắt buộc):**
> "Bạn nghĩ khả năng bài giải này là do MỘT NGƯỜI THẬT tự giải (không
> phải máy tính/AI) là bao nhiêu?"
> Thang Likert 1–5: 1 = Chắc chắn là máy, 5 = Chắc chắn là người thật

**Câu hỏi phụ (tuỳ chọn, giúp phân tích RQ-V3):**
> "Các nước đi trong bài giải này có cảm giác 'tự nhiên'/quen tay
> (finger-trick) hay không?"
> Thang Likert 1–5: 1 = Hoàn toàn không, 5 = Rất tự nhiên

**Câu hỏi mở (tuỳ chọn, dữ liệu định tính):**
> "Điều gì khiến bạn nghĩ vậy?" (text tự do, có thể bỏ trống)

## 7. Kế hoạch phân tích (khi có dữ liệu)

1. **Độ tin cậy liên giám khảo (inter-rater reliability)**: tính ICC
   (Intraclass Correlation) hoặc Krippendorff's alpha trên điểm câu hỏi
   chính — nếu quá thấp (<0.4), cần xem lại thiết kế/tiêu chí tuyển
   người trước khi diễn giải tương quan.
2. **RQ-V1**: tương quan Spearman giữa HLI (tự động) và điểm trung
   bình/trung vị người chấm cho mỗi stimulus (N=24 điểm dữ liệu).
3. **RQ-V2**: so điểm trung bình câu hỏi chính giữa Nhóm B và
   (A ∪ A+ ∪ H) bằng Mann-Whitney U test; và tính "tỷ lệ bị nhận đúng
   là máy" kiểu BotPrize (% giám khảo chấm ≤2 điểm) cho riêng Nhóm B.
4. **RQ-V3**: tương quan Spearman riêng giữa từng thành phần con
   (Segmentability, Pattern-conformity, Trigger-overlap) và điểm người
   chấm — thành phần nào tương quan mạnh nhất gợi ý trọng số nên tăng.

## 8. Giới hạn đã biết trước (ghi vào Threats to Validity)

- Trình bày dạng text (không video/hoạt hình) — có thể không phản ánh
  đúng "cảm giác" khi xem cube xoay thật; nhưng đây cũng chính là hình
  thức "reconstruction" tiêu chuẩn mà cộng đồng speedcubing tự dùng để
  đánh giá lẫn nhau, nên có tính hợp lệ sinh thái (ecological validity)
  hợp lý.
- Cỡ mẫu 15-20 người là pilot, không đủ mạnh cho publication-grade CI
  hẹp — phù hợp để "chứng minh khái niệm" (proof of concept) cho
  construct validity, nên nêu rõ giới hạn này khi viết Discussion.
- Nhóm H phụ thuộc nguồn bạn tự thu thập — chất lượng/tính đại diện của
  6 bài giải người thật ảnh hưởng trực tiếp đến kết luận.
