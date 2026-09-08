import sys, time, random
sys.path.insert(0, '.')
from cube_engine import make_solved, do_move, ALL_MOVES, random_scramble_moves
from solver.full_state import from_facelets
from solver.cross_solver import solve_cross, get_pdb
from solver.f2l_solver import solve_f2l

t0 = time.time()
pdb = get_pdb(verbose=False)
print(f'PDB Cross: {len(pdb)} trang thai duoc BFS het toan bo (build lan dau {time.time()-t0:.2f}s)')
print('  Khong gian day du 4 canh Cross: 12*11*10*9 * 2^4 =', 12*11*10*9*16, 'trang thai')
print('  -> BFS quet HET toan bo -> co khoang cach CHINH XAC tu MOI trang thai')
print('  -> chi can greedy (luon chon nuoc giam khoang cach 1) la ra loi giai TOI UU')

st = make_solved()
random.seed(42)
scramble_moves = random_scramble_moves(st, n=20)

print()
print('Scramble ngau nhien (20 nuoc):', ' '.join(scramble_moves))

t0 = time.time()
sol = solve_cross(st)
dt = time.time() - t0
print(f'AI tra PDB -> giai Cross trong {len(sol)} nuoc: {" ".join(sol)}   (tra cuu {dt*1000:.3f} ms)')

for mv in sol:
    do_move(st, mv)

t0 = time.time()
res = solve_f2l(st)
dt = time.time() - t0
print()
print('Ket qua solve_f2l:', type(res), list(res.keys()) if isinstance(res, dict) else res)
print(f'(thoi gian search F2L: {dt:.3f}s)')

print()
print('Chi tiet tung cap F2L (giai lan luot DFR -> DFL -> DBR -> DBL):')
for slot, mv in res['per_slot'].items():
    print(f'  {slot}: {len(mv) if mv else 0} nuoc -> {" ".join(mv) if mv else "(khong tim thay trong ngan sach)"}')
print()
print('Tong nuoc F2L:', len(res['moves']), '->', ' '.join(res['moves']))
print('Cac slot da xong:', res['solved_slots'])
