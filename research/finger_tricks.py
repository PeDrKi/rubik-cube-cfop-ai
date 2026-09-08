"""
research/finger_tricks.py
===========================
Corpus cac "trigger" (n-gram 2-4 nuoc) ma cong dong speedcubing dung nhu 1
don vi tu duy/thao tac ngon tay duy nhat, KHONG phai ghep tung nuoc rieng
le. Day la nen tang de tinh Trigger-overlap score cho Cross/F2L (phan AI tu
search, chua co "case chuan" nhu OLL/PLL).

Nguon: day la tri thuc pho bien, duoc dat ten thong nhat trong cong dong
speedsolving (khong thuoc ban quyen 1 tac gia cu the) -- tuy nhien khi viet
bao cao/luan van, NEN trich nguon tong quan (vd. J Perm "Finger tricks"
video series, speedsolving.com Wiki) thay vi coi day la dong gop goc cua
ban.

QUAN TRONG ve tinh khoa hoc: day la danh sach KHOI TAO (seed corpus) --
neu dung that trong nghien cuu, NEN mo rong bang cach thu thap solve thuc
te tu nguoi (vd tu file .log cua CSTimer) roi thong ke n-gram xuat hien
nhieu nhat, thay vi chi dua vao danh sach thu cong nay (tranh thien vi xac
nhan -- confirmation bias khi tu chon corpus).
"""

# Moi trigger: (ten, chuoi nuoc dang list). Chi liet ke dang "thuan" (R-based);
# ham normalize_mirror() ben duoi tu sinh cac bien the L-mirror.
TRIGGER_CORPUS = {
    'sexy_move':        ['R', 'U', "R'", "U'"],
    'sexy_move_inv':     ["U", 'R', "U'", "R'"],
    'reverse_sexy':      ["R'", "U'", 'R', 'U'],
    'sledgehammer':      ["R'", 'F', 'R', "F'"],
    'sledgehammer_inv':  ['F', "R'", "F'", 'R'],
    'hedgehog':          ['R', "U'", "R'"],
    'triple_sexy':       ['R', 'U', "R'", "U'", 'R', 'U', "R'", "U'", 'R', 'U', "R'", "U'"],
    'sune_trigger':      ['R', 'U', "R'", 'U', 'R', "U2", "R'"],
    'ubl_insert':        ["U'", 'R', 'U'],
    'urf_insert':        ['U', 'R', "U'"],
    'niklas':            ['R', 'U', "R'", "U'", "R'", 'F', 'R', "F'"],
}


def _mirror_move(mv: str) -> str:
    swap = {'R': 'L', 'L': 'R'}
    face = mv[0]
    if face in swap:
        return swap[face] + mv[1:]
    return mv


def all_trigger_variants():
    """Sinh ca ban goc R-based lan ban mirror L-based cho tung trigger,
    tra ve dict ten -> list-of-tuple (co the co nhieu bien the cung ten)."""
    variants = {}
    for name, seq in TRIGGER_CORPUS.items():
        variants[name] = [tuple(seq)]
        mirrored = tuple(_mirror_move(m) for m in seq)
        if mirrored != tuple(seq):
            variants[name].append(mirrored)
    return variants


if __name__ == '__main__':
    v = all_trigger_variants()
    for name, seqs in v.items():
        for s in seqs:
            print(f"{name}: {' '.join(s)}")
