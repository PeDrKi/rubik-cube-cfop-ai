"""
research/phase3_survey/prepare_stimuli.py
=============================================
Sinh 18 stimuli (6 Nhom A, 6 Nhom A+, 6 Nhom B) TU CUNG 6 SCRAMBLE, ghi ra
stimuli.json theo dinh dang rating_tool.html doc duoc. Nhan hien thi cho
nguoi cham CHI LA so thu tu ngau nhien -- khong tiet lo nhom nguon (xem
DESIGN.md muc 3 ve tam quan trong cua viec nay).

Nhom H (nguoi that) KHONG the tu sinh -- ban PHAI tu tay them vao
stimuli.json sau khi chay script nay (xem huong dan in ra cuoi script).

Chay:  python3 -m research.phase3_survey.prepare_stimuli
"""

import json
import random
import signal
import sys

sys.path.insert(0, '.')

from cube_engine import make_solved, do_move
from solver import cfop_ai
from research.baseline_kociemba import solve_baseline
from research.trigger_biased_search import solve_f2l_trigger_biased
from solver.cross_solver import solve_cross
from solver.oll_solver import solve_oll
from solver.pll_solver import solve_pll

N_PER_GROUP = 6
BASE_SEED = 9000
TIMEOUT_S = 25   # bo qua scramble pathological (xem CHANGELOG_SESSION.md)


class _TimeoutError(Exception):
    pass


def _alarm(signum, frame):
    raise _TimeoutError()


def _with_timeout(fn, *args, **kwargs):
    old = signal.signal(signal.SIGALRM, _alarm)
    signal.alarm(TIMEOUT_S)
    try:
        return fn(*args, **kwargs)
    except _TimeoutError:
        return None
    finally:
        signal.alarm(0)
        signal.signal(signal.SIGALRM, old)


def random_scramble(st, n, rng):
    from cube_engine import ALL_MOVES
    prev = None
    moves = []
    for _ in range(n):
        cand = [m for m in ALL_MOVES if m[0] != prev] if prev else ALL_MOVES
        mv = rng.choice(cand)
        do_move(st, mv)
        moves.append(mv)
        prev = mv[0]
    return moves


def solve_group_a(scramble_state, trigger_bias_lambda=0.0):
    """Giai bang pipeline AI-CFOP day du (Cross tra bang -> F2L search,
    co the co trigger-bias -> OLL/PLL tra bang), tra ve CHUOI NUOC DI
    PHANG (khong chia doan) -- dung cho ca Nhom A (lambda=0) va A+
    (lambda=1.0)."""
    st = {f: scramble_state[f].copy() for f in scramble_state}
    cross = solve_cross(st)
    for mv in cross:
        do_move(st, mv)
    f2l_res = solve_f2l_trigger_biased(st, lam=trigger_bias_lambda, nodes_per_depth=40_000)
    if f2l_res['moves'] is None or len(f2l_res['solved_slots']) < 4:
        return None
    for mv in f2l_res['moves']:
        do_move(st, mv)
    oll_res = solve_oll(st)
    if not oll_res['moves']:
        oll_moves = []
    else:
        oll_moves = oll_res['moves']
        for mv in oll_moves:
            do_move(st, mv)
    pll_res = solve_pll(st)
    pll_moves = pll_res['moves'] or []
    for mv in pll_moves:
        do_move(st, mv)
    return cross + f2l_res['moves'] + oll_moves + pll_moves


def main():
    rng_master = random.Random(BASE_SEED)
    stimuli = []
    seed = BASE_SEED
    collected = 0
    max_attempts = N_PER_GROUP * 4   # du du phong cho scramble pathological

    attempt = 0
    while collected < N_PER_GROUP and attempt < max_attempts:
        attempt += 1
        rng = random.Random(seed)
        st = make_solved()
        random_scramble(st, 20, rng)
        snapshot = {f: st[f].copy() for f in st}

        moves_a = _with_timeout(solve_group_a, snapshot, trigger_bias_lambda=0.0)
        moves_ap = _with_timeout(solve_group_a, snapshot, trigger_bias_lambda=1.0)
        st_b = {f: snapshot[f].copy() for f in snapshot}
        moves_b = _with_timeout(solve_baseline, st_b)

        if moves_a and moves_ap and moves_b:
            stimuli.append({'internal_group': 'A', 'seed': seed, 'moves': moves_a})
            stimuli.append({'internal_group': 'A+', 'seed': seed, 'moves': moves_ap})
            stimuli.append({'internal_group': 'B', 'seed': seed, 'moves': moves_b})
            collected += 1
            print(f'[{collected}/{N_PER_GROUP}] seed={seed} OK: '
                  f'A={len(moves_a)}mv A+={len(moves_ap)}mv B={len(moves_b)}mv', flush=True)
        else:
            print(f'[bo qua] seed={seed}: mot nhanh timeout/that bai '
                  f'(A={"OK" if moves_a else "FAIL"}, A+={"OK" if moves_ap else "FAIL"}, '
                  f'B={"OK" if moves_b else "FAIL"}) -- thu seed ke tiep', flush=True)
        seed += 1

    if collected < N_PER_GROUP:
        print(f'\n[canh bao] chi thu duoc {collected}/{N_PER_GROUP} sau {attempt} lan thu.')

    # Xao tron thu tu + gan ID an danh (KHONG lien quan gi toi nhom nguon)
    rng_master.shuffle(stimuli)
    for idx, s in enumerate(stimuli, start=1):
        s['display_id'] = f'Bài giải #{idx}'
        s['moves_str'] = ' '.join(s['moves'])

    out = {
        'instructions': (
            'Với mỗi bài giải dưới đây, cho biết bạn nghĩ khả năng đây là '
            'do một NGƯỜI THẬT tự giải (không phải máy tính/AI) là bao nhiêu.'
        ),
        'stimuli': stimuli,
    }

    out_path = 'research/phase3_survey/stimuli.json'
    with open(out_path, 'w', encoding='utf-8') as f:
        json.dump(out, f, ensure_ascii=False, indent=2)

    print(f'\nDa sinh {len(stimuli)} stimuli (Nhom A/A+/B) -> {out_path}')
    print('\n=== VIEC BAN CAN LAM TIEP ===')
    print('1. Mo file stimuli.json, tu tay THEM 6 stimuli Nhom H (nguoi that)')
    print('   theo dung dinh dang moi phan tu trong list "stimuli":')
    print('   {"internal_group": "H", "seed": <so_bat_ky>,')
    print('    "moves": ["R", "U", "R\'", ...], "display_id": "Bài giải #N",')
    print('    "moves_str": "R U R\' ..."}')
    print('   (display_id phai la so thu tu KE TIEP, khong trung voi cac muc da co,')
    print('    va NEN xao tron vi tri trong list de khong bi doan nhom theo thu tu)')
    print('2. Sau khi them xong, gui ca 2 file stimuli.json + rating_tool.html')
    print('   cho nguoi tham gia khao sat.')


if __name__ == '__main__':
    main()
