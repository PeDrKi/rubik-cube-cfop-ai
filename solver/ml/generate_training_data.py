"""
solver/ml/generate_training_data.py
=====================================
Sinh du lieu huan luyen cho mo hinh hoc "phan du" (residual) cua heuristic
OLL pha goc (buoc kho nhat da gap trong qua trinh phat trien).

Y tuong: heuristic hien tai h(s) = max(PDB huong goc, PDB Cross/F2L neu bi
vo) la CAN DUOI DUNG (admissible) nhung KHONG nam duoc tuong tac giua hai
nhom (VD: sua huong goc co the buoc phai tam thoi cham vao quan Cross/F2L
theo cach ma 2 PDB rieng le khong luong truoc). Ta huan luyen 1 model hoi
quy nho (Gradient Boosting, chay CPU, khong can GPU) de HOC phan chenh
lech (residual = khoang_cach_that - h(s)) tu chinh du lieu do cac solver
hien co sinh ra -- khong can nhan cong gan nhan, hoan toan tu dong.

Dau vao (feature) cho moi mau:
  co0..co3   : huong 4 goc U (0,1,2)
  cp0..cp3   : vi tri hien tai cua 4 goc U (0..7, trong 8 khe goc)
  h_base     : gia tri heuristic hien tai (PDB huong goc, khong tinh
               cross/F2L vi luc sinh du lieu cross/F2L luon nguyen)

Nhan (target): khoang_cach_that tim duoc boi A* (bi chan ngan sach -- neu
khong tim duoc trong ngan sach thi BO QUA mau do, khong dua vao du lieu
huan luyen sai/thieu).
"""

import sys
import os
import random
import pickle
import time

sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', '..'))

from cube_engine import make_solved, scramble_cube, do_move
from solver.cross_solver import solve_cross
from solver.f2l_solver import solve_f2l
from solver.oll_solver import solve_oll_edges_only
from solver.oll_solver import _corner_pdb, _corner_key, _goal_B, _heuristic_B
from solver.full_state import from_facelets, apply_move, NO_D_MOVES
from solver.search_utils import a_star
from solver import corner_model as CM


def _features(full):
    ep, eo, cp, co = full
    h_base = _corner_pdb().get(_corner_key(full), 8)
    return [co[0], co[1], co[2], co[3], cp[0], cp[1], cp[2], cp[3], h_base]


def generate(n_samples=4000, search_budget=30_000, search_depth=11, seed=0, verbose=True):
    random.seed(seed)
    X, y = [], []
    attempts = 0
    t0 = time.time()

    while len(X) < n_samples:
        attempts += 1
        st = make_solved()
        scramble_cube(st, n=random.randint(15, 30))

        cmv = solve_cross(st)
        for mv in cmv:
            do_move(st, mv)
        f2l_res = solve_f2l(st)
        for mv in f2l_res['moves']:
            do_move(st, mv)
        if len(f2l_res['solved_slots']) != 4:
            continue
        edge_mvs = solve_oll_edges_only(st)
        if edge_mvs is None:
            continue
        for mv in edge_mvs:
            do_move(st, mv)

        full = from_facelets(st)

        t_search = time.time()
        # dung DUNG heuristic production (_heuristic_B, manh hon, co tinh
        # den viec giu nguyen canh/Cross/F2L) de sinh ground-truth nhat
        # quan voi thuc te su dung, khong phai ban don gian hoa.
        sol = a_star(full, _goal_B, _heuristic_B, search_budget, search_depth, NO_D_MOVES)
        if verbose:
            print(f'[gen] attempt={attempts} search_time={round(time.time()-t_search,2)}s '
                  f'found={sol is not None}', flush=True)
        if sol is None:
            continue   # bo qua mau kho (khong co ground-truth dang tin cay trong ngan sach)

        X.append(_features(full))
        y.append(len(sol))

        if verbose and len(X) % 200 == 0:
            print(f'[gen] {len(X)}/{n_samples} samples '
                  f'(attempts={attempts}, {round(time.time()-t0,1)}s)', flush=True)

    return X, y


if __name__ == '__main__':
    n = int(sys.argv[1]) if len(sys.argv) > 1 else 4000
    out_path = os.path.join(os.path.dirname(__file__), 'oll_corner_training_data.pkl')

    X_all, y_all = [], []
    if os.path.exists(out_path):
        with open(out_path, 'rb') as fh:
            prev = pickle.load(fh)
        X_all, y_all = prev['X'], prev['y']
        print(f'Loaded {len(X_all)} existing samples, generating {n} more...')

    X, y = generate(n_samples=n, search_budget=80_000, search_depth=8,
                     seed=random.randint(0, 10**6))
    X_all.extend(X)
    y_all.extend(y)

    with open(out_path, 'wb') as fh:
        pickle.dump({'X': X_all, 'y': y_all}, fh)
    print('Saved', len(X_all), 'total samples to', out_path)
