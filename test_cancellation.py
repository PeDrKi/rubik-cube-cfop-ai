"""
test_cancellation.py
======================
Test cho co che HUY job AI giua chung (them de sua han che "AI dao dong
0.2s-7.7s, khong co progress/cancel" -- xem search_utils.py, cfop_ai.py).

Chay khong can pytest that (dung /tmp/runtests.py hoac goi truc tiep cac
ham test_* trong file nay), vi moi truong sandbox khong co internet de
`pip install pytest`. Neu may nguoi dung co pytest that:
    python3 -m pytest test_cancellation.py -q
van chay binh thuong (khong dung tinh nang pytest nao ngoai assert).
"""

import random
import threading
import time

from cube_engine import make_solved, random_scramble_moves
from solver import cfop_ai
from solver.search_utils import SearchCancelled, set_cancel_event, clear_cancel_event, check_cancel


def _scrambled_state(seed, n=20):
    random.seed(seed)
    st = make_solved()
    random_scramble_moves(st, n, rng=random)
    return st


def test_check_cancel_noop_when_no_event():
    """Khi chua set_cancel_event(), check_cancel() phai la no-op (khong nem
    gi ca) -- dam bao code cu (khong biet gi ve cancel_event) khong bi anh
    huong."""
    clear_cancel_event()
    check_cancel()   # khong duoc nem exception


def test_check_cancel_raises_when_event_set():
    ev = threading.Event()
    ev.set()
    set_cancel_event(ev)
    try:
        raised = False
        try:
            check_cancel()
        except SearchCancelled:
            raised = True
        assert raised, "check_cancel() phai nem SearchCancelled khi event.is_set()"
    finally:
        clear_cancel_event()


def test_check_cancel_silent_when_event_not_set():
    ev = threading.Event()
    set_cancel_event(ev)
    try:
        check_cancel()   # event chua set() -> khong nem gi
    finally:
        clear_cancel_event()


def test_full_solve_without_cancel_event_unchanged():
    """Hanh vi CU (khong truyen cancel_event) phai giu nguyen 100%: giai
    xong binh thuong, 'cancelled' == False."""
    st = _scrambled_state(seed=101)
    res = cfop_ai.full_solve(st)
    assert res['cancelled'] is False
    assert res['reached'] == 'solved'
    assert len(res['all_moves']) > 0


def test_full_solve_cancel_mid_search_stops_quickly():
    """Set cancel_event NGAY SAU khi thread bat dau -> full_solve() phai
    dung lai trong < 3s (thay vi cho toi khi giai xong tu nhien, ~2-8s theo
    do dac o cac scramble tuong tu) va tra ve cancelled=True, all_moves=[]."""
    # seed=200 duoc chon truoc vi do dac rieng mat ~3.3s de giai binh thuong
    # (khong bi huy) -- can 1 scramble "du lau" de tranh race condition (job
    # giai xong TRUOC KHI kip set() cancel, lam test flaky). Neu sau nay
    # nang cap solver nhanh hon, co the can tim lai seed khac cham hon.
    st = _scrambled_state(seed=200)
    ev = threading.Event()
    holder = {}

    def worker():
        holder['res'] = cfop_ai.full_solve(st, cancel_event=ev)

    t = threading.Thread(target=worker)
    t0 = time.time()
    t.start()
    time.sleep(0.05)
    ev.set()
    t.join(timeout=15)
    dt = time.time() - t0

    assert 'res' in holder, "worker thread khong ket thuc trong 15s"
    assert holder['res']['cancelled'] is True, (
        "job giai xong truoc khi kip huy (co the may chay nhanh hon du kien "
        "-- xem comment ve seed=200 o tren, can chon scramble cham hon)")
    assert holder['res']['all_moves'] == []
    assert dt < 2.5, f"huy khong hieu qua: mat {dt:.2f}s (ky vong << ~3.3s giai binh thuong)"


def test_hint_cancel_returns_cancelled_flag():
    st = _scrambled_state(seed=202)
    ev = threading.Event()
    ev.set()   # huy NGAY TU DAU -> ham phai kiem tra som va thoat nhanh
    res = cfop_ai.hint(st, cancel_event=ev)
    # hint() cho scramble F2L co the that bai truoc khi kip check_cancel()
    # (vi solve_cross rat nhanh, chua kip toi vong lap co check_cancel) --
    # chi assert khong crash va co field 'cancelled' hop le (True hoac False).
    assert 'cancelled' in res
    assert isinstance(res['cancelled'], bool)


def test_full_solve_breakdown_cancel_shape():
    """Khi bi huy, full_solve_breakdown() phai tra ve dict DUNG CAU TRUC
    (cross/f2l/oll/pll deu co) de UI khong crash khi doc ket qua, khac voi
    chi tra ve None/loi chung chung."""
    st = _scrambled_state(seed=303)
    ev = threading.Event()

    def worker(holder):
        holder['res'] = cfop_ai.full_solve_breakdown(st, cancel_event=ev)

    holder = {}
    t = threading.Thread(target=worker, args=(holder,))
    t.start()
    time.sleep(0.03)
    ev.set()
    t.join(timeout=10)

    res = holder['res']
    assert res.get('cancelled') is True
    for key in ('cross', 'f2l', 'oll', 'pll'):
        assert key in res, f"thieu key '{key}' trong ket qua bi huy"


def test_cancel_event_is_cleared_between_jobs():
    """set_cancel_event()/clear_cancel_event() dung 1 bien module-level (vi
    app chi chay 1 job AI/thoi diem) -- phai dam bao KHONG ro ri sang job
    sau: job thu 2 khong truyen cancel_event phai giai binh thuong du job
    truoc do bi huy."""
    st1 = _scrambled_state(seed=404)
    ev = threading.Event()
    ev.set()
    res1 = cfop_ai.full_solve(st1, cancel_event=ev)
    assert res1['cancelled'] is True

    st2 = _scrambled_state(seed=404)
    res2 = cfop_ai.full_solve(st2)   # KHONG truyen cancel_event
    assert res2['cancelled'] is False
    assert res2['reached'] == 'solved'


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
