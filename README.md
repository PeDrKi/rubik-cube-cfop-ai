# RUBIK — CFOP AI Solver + Simulator

Dự án: AI giải Rubik theo phương pháp CFOP (Cross → F2L → OLL → PLL)
với chỉ số Human-Likeness Index (HLI) và thuật toán Trigger-Biased A*
Search

## Đọc gì trước — tuỳ mục đích

| Mục đích | Đọc file |
|---|---|
| **Cài đặt & chạy app lần đầu** | [`docs/HUONG_DAN_CHAY.md`](docs/HUONG_DAN_CHAY.md) |
| **Hiểu kiến trúc thuật toán solver** (CFOP, PDB, IDA*...) | [`docs/CFOP_AI_README.md`](docs/CFOP_AI_README.md) |
| **Tái lập số liệu/hình/bảng trong paper** | [`docs/REPRODUCE.md`](docs/REPRODUCE.md) |
| **Chạy test tầng app** (formula panel, huỷ job AI, logic UI...) | [`docs/README_TEST_APP.md`](docs/README_TEST_APP.md) |
| **Xem đã sửa lỗi/hạn chế gì gần đây và tại sao** | [`docs/REFACTOR_NOTES.md`](docs/REFACTOR_NOTES.md) |
| **Lịch sử chi tiết từng phiên làm việc** | [`docs/CHANGELOG_SESSION.md`](docs/CHANGELOG_SESSION.md) |
| **Đọc paper** | [`paper/main_vi.pdf`](paper/main_vi.pdf) (tiếng Việt) / [`paper/main.pdf`](paper/main.pdf) (English) |

## Cấu trúc thư mục

```
main.py, formula_panel.py, app_logic.py,      # App mô phỏng (pygame) -- CỐ Ý ở
draw_helpers.py, layout.py, renderer_3d.py,   # cùng cấp root với solver/vì cube_engine.py
                                               # được cả solver/*.py lẫn research/*.py
                                               # import trực tiếp giả định nó ở root.

solver/                                        # AI CFOP
  cfop_ai.py, cross_solver.py, f2l_solver.py,
  oll_solver.py, pll_solver.py, search_utils.py,
  pdb_builder.py, *_model.py, *_algorithms.py
  cache/                                       # Pattern DB đã build sẵn
                                                #(tự build lại nếu xoá, chỉ chậm hơn lần đầu)

tests/                                         # 152 test (xem docs/README_TEST_APP.md)
  test_*.py
  fake_pygame_stub/                            # pygame giả để test tầng app khi
                                                # máy không cài pygame thật

scripts/                                       # Script CLI độc lập, KHÔNG phải 1 phần
  benchmark.py, demo_ai_search.py              # của main.py -- chạy tay để xem số
                                                # liệu/PDB, không liên quan demo app.
                                                # Chạy được từ bất kỳ thư mục nào.

research/                                       # Thí nghiệm cho paper (Wilcoxon,
  ml_exploration/                               # bootstrap CI, khảo sát phase3...)
                                                 # ml_exploration/: hướng thử nghiệm
                                                 # "học phần dư heuristic OLL", KHÔNG
                                                 # nằm trong pipeline solver chính.
paper/                                          # main.tex/main_vi.tex + PDF đã build

docs/                                           # Toàn bộ tài liệu phụ (xem bảng trên)
```

## Chạy nhanh

```bash
pip install -r requirements.txt
python3 main.py
```

Chạy test (từ thư mục gốc repo, hoặc bất kỳ đâu — các file test tự tìm
đường dẫn gốc repo, xem `docs/README_TEST_APP.md`):

```bash
python3 tests/test_cube_engine.py
python3 tests/test_solver.py
python3 tests/test_cancellation.py
python3 tests/test_app_logic.py     # tự dùng tests/fake_pygame_stub/ nếu máy không có pygame thật
python3 tests/test_app_logic2.py
```

Phím tắt chính: 
- **Space** scramble · 
- **A** AI tự giải · 
- **H** gợi ý ·
- **T** bảng công thức CFOP · 
- **Esc** huỷ job AI đang chạy / thoát ·
- **F11** fullscreen · 
- **P** overlay hiệu năng (FPS).

## License

MIT — xem [`LICENSE`](LICENSE).
