"""
research/hli_metrics.py
=========================
4 chi so con cua Human-Likeness Index (HLI) + ham tong hop.

Tat ca deu la ham THUAN (pure function): nhan vao ket qua da giai (dict tra
ve boi cfop_ai.full_solve() hoac list nuoc di tho tu baseline), khong tu
goi solver -- de de test doc lap va tai su dung trong run_experiment.py.
"""

from .finger_tricks import all_trigger_variants

_TRIGGER_VARIANTS = all_trigger_variants()
# Sap xep trigger dai truoc de match tham lam (greedy longest-match),
# tranh dem trung khi trigger ngan la tap con cua trigger dai hon.
_ALL_TRIGGERS_BY_LEN = sorted(
    ((name, seq) for name, seqs in _TRIGGER_VARIANTS.items() for seq in seqs),
    key=lambda x: -len(x[1])
)


# ── 1. Segmentability ────────────────────────────────────────────────────

def segmentability(cfop_result: dict) -> float:
    """1.0 neu loi giai co du 4 chang CFOP tach bach va giai hoan chinh
    (reached == 'solved'), 0.0 neu khong (that bai o giua chung, hoac la
    loi giai "phang" khong chia chang duoc nhu baseline computer-like).
    """
    return 1.0 if cfop_result.get('reached') == 'solved' else 0.0


# ── 2. Pattern-conformity (OLL/PLL) ──────────────────────────────────────

def pattern_conformity(cfop_result: dict) -> float:
    """Ty le (0..1) cac chang OLL+PLL dung DUNG 1 thuat toan co ten chuan
    (khong phai macro du phong / search vu vet). Doc truc tiep tu field
    co san trong ket qua cfop_ai (can bo sung field nay -- xem ghi chu
    duoi cung file).
    """
    total = cfop_result.get('oll_pll_stage_count', 0)
    named = cfop_result.get('oll_pll_named_count', 0)
    if total == 0:
        return 0.0
    return named / total


# ── 3. Trigger-overlap score (Cross/F2L) ─────────────────────────────────

def trigger_overlap(moves: list[str]) -> float:
    """Ty le (0..1) nuoc di trong chuoi thuoc ve 1 trigger da biet (thay vi
    la nuoc "roi rac" AI tu ghep khong theo pattern nao). Dung greedy
    longest-match tu trai sang phai tren chuoi nuoc.
    """
    if not moves:
        return 0.0
    n = len(moves)
    covered = [False] * n
    i = 0
    while i < n:
        matched_len = 0
        for _name, seq in _ALL_TRIGGERS_BY_LEN:
            L = len(seq)
            if i + L <= n and tuple(moves[i:i + L]) == seq:
                matched_len = L
                break
        if matched_len:
            for k in range(i, i + matched_len):
                covered[k] = True
            i += matched_len
        else:
            i += 1
    return sum(covered) / n


# ── 4. Move-count ratio (bien kiem soat, KHONG cong diem duong vao HLI) ──

GODS_NUMBER_HTM = 20


def move_count_ratio(num_moves: int) -> float:
    """num_moves / God's Number. Dung de BAO CAO / GIAI THICH, khong dua
    vao cong thuc HLI theo huong "cang gan 1 cang tot" -- vi muc tieu
    nghien cuu la human-likeness, KHONG phai toi uu so nuoc."""
    return num_moves / GODS_NUMBER_HTM


# ── Tong hop HLI ──────────────────────────────────────────────────────────

DEFAULT_WEIGHTS = {
    'segmentability': 0.30,
    'pattern_conformity': 0.40,
    'trigger_overlap': 0.30,
}


def compute_hli(cfop_result: dict, cross_f2l_moves: list[str],
                 weights: dict = None) -> dict:
    """Tra ve dict day du: 3 chi so con + HLI tong hop (0..1) + move-count
    ratio (chi de bao cao, khong tinh vao HLI)."""
    w = weights or DEFAULT_WEIGHTS
    seg = segmentability(cfop_result)
    pat = pattern_conformity(cfop_result)
    trg = trigger_overlap(cross_f2l_moves)
    hli = w['segmentability'] * seg + w['pattern_conformity'] * pat + w['trigger_overlap'] * trg
    return {
        'segmentability': seg,
        'pattern_conformity': pat,
        'trigger_overlap': trg,
        'HLI': hli,
        'move_count_ratio': move_count_ratio(len(cfop_result.get('all_moves', []))),
    }


"""
GHI CHU: 'oll_pll_stage_count' / 'oll_pll_named_count' da duoc them vao
cfop_ai.full_solve() (xem solver/oll_solver.py::solve_oll 'corner_source',
solver/pll_solver.py::solve_pll 'source', va solver/cfop_ai.py gop 2 field
nay). Khong can sua gi them de dung pattern_conformity() trong thuc te.
"""
