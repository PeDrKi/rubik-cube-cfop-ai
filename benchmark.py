import sys, time, random, statistics
sys.path.insert(0, '.')
from cube_engine import make_solved, do_move, ALL_MOVES, random_scramble_moves
from solver import cfop_ai

random.seed(7)
N = 10

move_counts = []
times = []
reached_counts = {}

for i in range(N):
    st = make_solved()
    random_scramble_moves(st, n=25)

    t0 = time.time()
    res = cfop_ai.full_solve(st)
    dt = time.time() - t0

    reached_counts[res['reached']] = reached_counts.get(res['reached'], 0) + 1
    if res['reached'] == 'solved':
        move_counts.append(len(res['all_moves']))
        times.append(dt)
    print(f'  [{i+1}/{N}] reached={res["reached"]} moves={len(res["all_moves"])} time={dt:.2f}s', flush=True)

print(f'Chay {N} scramble ngau nhien (25 nuoc/scramble):')
print('Phan bo ket qua "reached":', reached_counts)
print()
if move_counts:
    print(f'Trong {len(move_counts)} lan giai HOAN TOAN thanh cong:')
    print(f'  So nuoc trung binh : {statistics.mean(move_counts):.1f}')
    print(f'  So nuoc min/max    : {min(move_counts)} / {max(move_counts)}')
    print(f'  Do lech chuan      : {statistics.pstdev(move_counts):.1f}')
    print(f'  Thoi gian TB       : {statistics.mean(times)*1000:.1f} ms')
    print(f'  Thoi gian max      : {max(times)*1000:.1f} ms')
    print()
    print("  So sanh voi \"God's Number\" (20 nuoc toi uu tuyet doi, Rokicki 2010):")
    print(f'    -> giai phap nay dai hon toi uu trung binh {statistics.mean(move_counts)-20:.1f} nuoc ({(statistics.mean(move_counts)/20-1)*100:.0f}% so voi toi uu)')
