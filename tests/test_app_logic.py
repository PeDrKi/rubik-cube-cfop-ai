"""
test_app_logic.py
===================
Test cho cac HAM LOGIC THUAN trong formula_panel.py (bang cong thuc CFOP,
tach ra tu main.py -- xem REFACTOR_NOTES.md). Truoc ban sua nay, KHONG co
test nao cho tang app/UI (chi test_cube_engine.py + test_solver.py cho
tang engine/solver). File nay lap khoang trong do cho phan co the tach
rieng khoi pygame that (build_formula_lines, looks_like_moves,
formula_row_data, formula_lines_to_text) -- day la nhung ham RENDER-
INDEPENDENT: nhan/tra ve du lieu thuan (dict/list/str), khong dung
pygame.display/Surface, nen kiem chung duoc ma khong can mo cua so that.

CHAY THU NAO:
  - Neu may co pygame that (vd may dev cua ban):
        python3 -m pytest tests/test_app_logic.py -q
    (hoac python3 tests/test_app_logic.py de chay bang runner noi bo,
    khong can pytest that).
  - Trong moi truong KHONG co pygame (vd sandbox nay, khong co internet de
    pip install pygame): file nay TU DONG them tests/fake_pygame_stub/
    (nam CUNG thu muc voi file nay) vao sys.path neu `import pygame that`
    that bai -- xem bootstrap ngay duoi day. Khong can tu tay set
    PYTHONPATH nua (khac ban truoc chuyen vao tests/), nhung van chay duoc
    neu ban tu set PYTHONPATH=./tests/fake_pygame_stub:. (xem
    docs/README_TEST_APP.md).
    Muc dich cua stub CHI la cho main.py import duoc (main() that su -- vong
    lap game -- KHONG duoc goi va KHONG duoc test o day, no can pygame that
    100% de co y nghia; xem docs/README_TEST_APP.md).

Cac ham can renderer that (_draw_formula_panel_content, _wrap_text voi
font that, _try_open_formula_window) KHONG test o day -- do la ly do phan
"kien truc" (xem docs/REFACTOR_NOTES.md) de nghi tach main() thanh module
rieng de co the mock/test sau nay.
"""

import os
import sys

_THIS_DIR = os.path.dirname(os.path.abspath(__file__))
_ROOT = os.path.dirname(_THIS_DIR)
sys.path.insert(0, _ROOT)   # tim cube_engine/solver/formula_panel... o root

try:
    import pygame   # noqa: F401  (chi de kiem tra co pygame that khong)
except ImportError:
    # Khong co pygame that -> tu dong dung stub CUNG thu muc (tests/fake_pygame_stub/)
    # thay vi bat nguoi dung phai tu set PYTHONPATH thu cong.
    sys.path.insert(0, os.path.join(_THIS_DIR, 'fake_pygame_stub'))

import formula_panel as fp


# ── Fixtures / du lieu mau ─────────────────────────────────────────────────

def _breakdown_all_done():
    row_done = {'status': 'done', 'moves': [], 'case_name': None}
    return {
        'cross': {'status': 'done', 'moves': []},
        'f2l': {s: dict(row_done) for s in ('DFR', 'DFL', 'DBR', 'DBL')},
        'oll': dict(row_done),
        'pll': dict(row_done),
    }


def _breakdown_mixed():
    return {
        'cross': {'status': 'done', 'moves': []},
        'f2l': {
            'DFR': {'status': 'done', 'moves': []},
            'DFL': {'status': 'moves', 'moves': ["R", "U", "R'"]},
            'DBR': {'status': 'not_reached', 'moves': []},
            'DBL': {'status': 'not_reached', 'moves': []},
        },
        'oll': {'status': 'not_reached', 'moves': [], 'case_name': None},
        'pll': {'status': 'not_reached', 'moves': [], 'case_name': None},
    }


def _breakdown_named_case():
    return {
        'cross': {'status': 'done', 'moves': []},
        'f2l': {s: {'status': 'done', 'moves': [], 'case_name': None}
                for s in ('DFR', 'DFL', 'DBR', 'DBL')},
        'oll': {'status': 'moves', 'moves': ['R', 'U2', "R'"], 'case_name': 'Sune'},
        'pll': {'status': 'failed', 'moves': [], 'case_name': None},
    }


# ── build_formula_lines / _stage_row ───────────────────────────────────────

def test_build_formula_lines_has_4_headers():
    lines = fp.build_formula_lines(_breakdown_all_done())
    headers = [l for l in lines if l[0] == 'header']
    assert len(headers) == 4
    assert headers[0][1].startswith('1) CROSS')
    assert headers[1][1].startswith('2) F2L')
    assert headers[2][1].startswith('3) OLL')
    assert headers[3][1].startswith('4) PLL')


def test_build_formula_lines_all_done_shows_DONE():
    lines = fp.build_formula_lines(_breakdown_all_done())
    rows = [l for l in lines if l[0] == 'row']
    # Cross + 4 F2L slot + OLL + PLL = 7 dong 'row'
    assert len(rows) == 7
    for row in rows:
        _, label, text, color = row
        assert text == 'DONE!'


def test_build_formula_lines_mixed_status():
    lines = fp.build_formula_lines(_breakdown_mixed())
    rows = {r[1]: r for r in lines if r[0] == 'row'}
    assert rows['Cross'][2] == 'DONE!'
    assert rows['DFR'][2] == 'DONE!'
    assert rows['DFL'][2] == "R U R'"
    # not_reached -> chua co du lieu, hien thong bao "chua toi luot"
    assert 'chưa tới lượt' in rows['DBR'][2]
    assert 'chưa tới lượt' in rows['OLL'][2]


def test_build_formula_lines_named_case_shown_in_label():
    lines = fp.build_formula_lines(_breakdown_named_case())
    rows = {r[1]: r for r in lines if r[0] == 'row'}
    # case_name='Sune' -> nhan phai la "OLL:Sune" (hoac ten dep trong
    # PRETTY_CASE_NAME neu co ánh xạ khác), KHONG con la "OLL" tran
    assert any(k.startswith('OLL:') for k in rows), \
        f"khong thay nhan OLL:<case>, cac nhan hien co: {list(rows.keys())}"


def test_stage_row_failed_message_mentions_retry_hint():
    row = fp._stage_row('PLL', {'status': 'failed', 'moves': [], 'case_name': None})
    assert row[0] == 'row'
    assert 'chưa tìm được' in row[2]


def test_stage_row_not_reached():
    row = fp._stage_row('DBR', {'status': 'not_reached', 'moves': []})
    assert 'chưa tới lượt' in row[2]


# ── _looks_like_moves ───────────────────────────────────────────────────────

def test_looks_like_moves_true_for_real_moves():
    assert fp.looks_like_moves("R U R' U'") is True
    assert fp.looks_like_moves("F2") is True


def test_looks_like_moves_false_for_done_and_notes():
    assert fp.looks_like_moves("DONE!") is False
    assert fp.looks_like_moves("(chưa tìm được trong ngân sách hiện tại)") is False
    assert fp.looks_like_moves("") is False
    assert fp.looks_like_moves(None) is False


# ── _formula_row_data / _formula_lines_to_text ──────────────────────────────

def test_formula_row_data_parallel_lists():
    lines = fp.build_formula_lines(_breakdown_mixed())
    labels, formulas = fp.formula_row_data(lines)
    assert len(labels) == len(formulas) == 7
    idx = labels.index('DFL')
    assert formulas[idx] == "R U R'"


def test_formula_lines_to_text_roundtrip_contains_all_labels():
    lines = fp.build_formula_lines(_breakdown_mixed())
    text = fp.formula_lines_to_text(lines)
    assert 'Cross:' in text
    assert 'DFL:' in text
    assert "R U R'" in text
    # header van con trong text (khong bi loc mat)
    assert '1) CROSS' in text


# ── Cac ham nay dam bao build_formula_lines() la THUAN (khong side-effect,
#    khong mutate breakdown truyen vao) -- quan trong vi main() goi lai ham
#    nay nhieu lan voi cung du lieu tu queue ket qua AI. ─────────────────────

def test_build_formula_lines_does_not_mutate_input():
    bd = _breakdown_mixed()
    import copy
    bd_copy = copy.deepcopy(bd)
    fp.build_formula_lines(bd)
    assert bd == bd_copy, "build_formula_lines() khong duoc sua doi breakdown dau vao"


if __name__ == '__main__':
    import sys
    fns = [v for k, v in list(globals().items()) if k.startswith('test_') and callable(v)]
    ok = fail = 0
    for fn in fns:
        try:
            fn()
            ok += 1
            print(f"  ok  {fn.__name__}")
        except Exception as e:
            fail += 1
            print(f" FAIL {fn.__name__}: {e}")
    print(f"\n{ok} passed, {fail} failed ({len(fns)} tests)")
    sys.exit(1 if fail else 0)
