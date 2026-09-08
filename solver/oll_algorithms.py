"""
solver/oll_algorithms.py
============================
Co so du lieu cong thuc OLL DAY DU (khong chi rieng OCLL) -- moi cong
thuc dinh huong CA canh LAN goc lop U trong 1 buoc, thay cho kien truc
2-look (dinh huong canh bang search truoc, roi goc bang Sune/Anti-Sune
sau) khi co san cong thuc phu hop trong bang.

QUAN TRONG -- 2 BAI HOC TU QUA TRINH XAY DUNG (xem CHANGELOG_SESSION.md
de biet chi tiet qua trinh phat hien):

1. OLL KHONG can giu nguyen hoan vi (permutation) -- chi can giu dung
   HUONG (orientation). Ban dau tuong nham la phai giu cp/ep khong doi
   (giong PLL), nhung dieu do SAI: cac thuat toan OLL that (vd Sune) VAN
   hoan vi ca goc lan canh, chi la PLL sau do se sua lai hoan vi. Kiem
   chung o day CHI kiem tra HUONG, khong kiem tra hoan vi.

2. cp/co (va ep/eo) trong engine nay la PIECE-INDEXED (theo doi tung
   VIEN cu the qua cac nuoc di), KHONG PHAI slot-indexed (theo VI TRI
   hien tai). Vi cac thuat toan OLL bat buoc phai hoan vi (theo bai hoc
   1), khi ap dung 1 thuat toan len cube da giai, co[j] sau do la "huong
   ma VIEN j (bat dau o vi tri j) da tich luy", KHONG PHAI "huong TAI
   VI TRI j hien tai". Vi cach cac nuoc di cong don huong theo VI TRI
   HIEN TAI cua tung buoc (xem corner_model.py::apply_move_to_corners),
   co the CHUNG MINH: huong tich luy cua "vien bat dau o vi tri s" CHINH
   LA delta ma BAT KY vien nao khac cung se tich luy NEU no cung bat dau
   o vi tri s (khong phu thuoc no la vien nao) -- vi cac quy tac cong
   don huong chi phu thuoc VI TRI, khong phu thuoc DANH TINH vien. Suy
   ra: pattern THAT SU can co o vi tri s (SLOT-indexed) de thuat toan
   giai duoc la NGHICH DAO (am, mod 3 cho goc / mod 2 cho canh) cua
   delta doc duoc tu ket qua ap dung len cube da giai.

Tieu chuan kiem chung cho moi thuat toan X (ap dung tu trang thai da giai):
  1. KHONG lam vo Cross + F2L (cross_f2l_ok).
  2. (Khong yeu cau gi ve hoan vi -- xem bai hoc 1 o tren.)
Bang tra cuu duoc xay theo key = (-delta) mod (2 hoac 3) cho tung vi tri,
DA LA SLOT-INDEXED DUNG NGHIA (xem bai hoc 2), dung truc tiep duoc voi
oll_recognition.identify_oll_case() ma khong can chuyen doi them.
"""

from cube_engine import make_solved, do_move
from .full_state import from_facelets, cross_f2l_ok
from .oll_recognition import EDGE_POS, CORNER_POS

_RAW_ALGS = {
    # --- OCLL (canh da dinh huong san, chi goc can xoay) ---
    'Sune':     "R U R' U R U2 R'",
    'AntiSune': "R U2 R' U' R U' R'",

    # --- Full OLL (ca canh lan goc can dinh huong) ---
    # Dot case (0 canh dinh huong) -- do nguoi dung cung cap va kiem
    # chung thuc te dung cho 1 case Dot that (xem hoi thoai).
    'Dot_variant1': "F R' F' R U R U' R' U F R U R' U' F'",
    # --- Cac case con lai (49 non-dot + 5 dot), sinh/kiem chung tu dong ---
    # 49 case non-dot: dung chinh 2-look solver noi bo (_solve_phase_A_ladder +
    # _solve_phase_B_ladder) de tu giai tung trang thai mau roi ghep chuoi nuoc
    # di lai -- KHONG chep tay, tu kiem chung 100% bang cross_f2l_ok + huong=0.
    'case_auto_01': "R U2 R' U' R U' R' U R U R' U R U2 R'",
    'case_auto_02': "U R U2 R' U' R U' R' U' R U R' U R U2 R'",
    'case_auto_03': "U R U R' U R U2 R'",
    'case_auto_04': "U2 R U R' U R U2 R' U2 R U2 R' U' R U' R'",
    'case_auto_05': "U' R U2 R' U' R U' R'",
    'case_auto_06': "R U R' U R U2 R' U' R U R' U R U2 R'",
    'case_auto_07': "R U2 R' U' R U' R' R U2 R' U' R U' R'",
    'case_auto_08': "L' B' U' B U L U R U2 R' U' R U' R' U' R U R' U R U2 R'",
    'case_auto_09': "L' B' U' B U L R U2 R' U' R U' R' R U2 R' U' R U' R'",
    'case_auto_10': "L' B' U' B U L R U R' U R U2 R' U R U R' U R U2 R'",
    'case_auto_11': "L' B' U' B U L U' R U2 R' U' R U' R'",
    'case_auto_12': "L' B' U' B U L R U R' U R U2 R' U' R U R' U R U2 R'",
    'case_auto_13': "L' B' U' B U L U2 R U2 R' U' R U' R'",
    'case_auto_14': "L' B' U' B U L U2 R U R' U R U2 R' U2 R U2 R' U' R U' R'",
    'case_auto_15': "L' B' U' B U L U R U2 R' U' R U' R'",
    'case_auto_16': "L' B' U' B U L R U R' U R U2 R' U' R U2 R' U' R U' R'",
    'case_auto_17': "L' B' U' B U L U2 R U R' U R U2 R'",
    'case_auto_18': "L' B' U' B U L R U2 R' U' R U' R'",
    'case_auto_19': "L' B' U' B U L R U R' U R U2 R' U R U2 R' U' R U' R'",
    'case_auto_20': "L' B' U' B U L U R U R' U R U2 R' U' R U2 R' U' R U' R'",
    'case_auto_21': "L' B' U' B U L U2 R U R' U R U2 R' U' R U R' U R U2 R'",
    'case_auto_22': "L' B' U' B U L U R U R' U R U2 R' U' R U R' U R U2 R'",
    'case_auto_23': "B' U' R' U R B U2 R U2 R' U' R U' R' U' R U R' U R U2 R'",
    'case_auto_24': "B' U' R' U R B U2 R U R' U R U2 R' U2 R U2 R' U' R U' R'",
    'case_auto_25': "B' U' R' U R B U R U R' U R U2 R'",
    'case_auto_26': "B' U' R' U R B R U R' U R U2 R' U' R U R' U R U2 R'",
    'case_auto_27': "B' U' R' U R B U' R U R' U R U2 R'",
    'case_auto_28': "B' U' R' U R B U2 R U R' U R U2 R'",
    'case_auto_29': "B' U' R' U R B R U R' U R U2 R' R U R' U R U2 R'",
    'case_auto_30': "B' U' R' U R B U R U R' U R U2 R' U' R U2 R' U' R U' R'",
    'case_auto_31': "B' U' R' U R B R U R' U R U2 R' U R U R' U R U2 R'",
    'case_auto_32': "B' U' R' U R B U' R U2 R' U' R U' R'",
    'case_auto_33': "B' U' R' U R B R U R' U R U2 R' U2 R U2 R' U' R U' R'",
    'case_auto_34': "B' U' R' U R B R U R' U R U2 R' U' R U2 R' U' R U' R'",
    'case_auto_35': "B' U' R' U R B U R U R' U R U2 R' U' R U R' U R U2 R'",
    'case_auto_36': "B' U' R' U R B R U R' U R U2 R' U R U2 R' U' R U' R'",
    'case_auto_37': "B' U' R' U R B R U2 R' U' R U' R' R U2 R' U' R U' R'",
    'case_auto_38': "B' U' R' U R B R U2 R' U' R U' R'",
    'case_auto_39': "B' U' R' U R B U R U2 R' U' R U' R'",
    'case_auto_40': "B' U' R' U R B U2 R U R' U R U2 R' U' R U R' U R U2 R'",
    'case_auto_41': "B' U' R' U R B R U2 R' U' R U' R' U R U R' U R U2 R'",
    'case_auto_42': "B' U' R' U R B",
    'case_auto_43': "B' U' R' U R B U R U2 R' U' R U' R' U' R U R' U R U2 R'",
    'case_auto_44': "B' U' R' U R B U' R U R' U R U2 R' U2 R U2 R' U' R U' R'",
    'case_auto_45': "B' U' R' U R B U2 R U R' U R U2 R' U' R U2 R' U' R U' R'",
    'case_auto_46': "B' U' R' U R B R U R' U R U2 R'",
    'case_auto_47': "B' U' R' U R B U2 R U2 R' U' R U' R'",
    'case_auto_48': "B' U' R' U R B R U2 R' U' R U' R' U' R U R' U R U2 R'",
    'case_auto_49': "B' U' R' U R B U R U R' U R U2 R' U2 R U2 R' U' R U' R'",

    # --- 5 case Dot con lai (OLL 1, 3, 4, 17, 19 theo danh so cong dong) ---
    # Tra tu SpeedCubeReview.com / SpeedCubeDB.com (chi dung nuoc don lop
    # U/D/F/B/L/R, khong M/S/E/wide/rotation -- dung yeu cau engine), TU KIEM
    # CHUNG lai bang chinh co che _table_key_for cua file nay (khong tin suong
    # nguon web). OLL 2 va OLL 20 (2/8 case Dot con lai) CHUA tim duoc cong
    # thuc thuan (chi dung M/wide trong moi nguon tra duoc) -- se roi ve 2-look
    # search (van dung, chi cham hon).
    'Dot_OLL1': "R U2 R2 F R F' U2 R' F R F'",
    'Dot_OLL3': "F U R U' R' F' U F R U R' U' F'",
    'Dot_OLL4': "F' U' L' U L F U F R U R' U' F'",
    'Dot_OLL17': "R U R' U R' F R F' U2 R' F R F'",
    'Dot_OLL19': "F R' F' R U R U' R' U' F R U R' U' F'",
}


def _inv(seq):
    out = []
    for m in reversed(seq):
        if m.endswith("'"):
            out.append(m[:-1])
        elif m.endswith('2'):
            out.append(m)
        else:
            out.append(m + "'")
    return out


def _delta_of(seq):
    """Ap seq len cube da giai, tra ve (eo_delta[:4], co_delta[:4], hop
    le khong). Hop le = khong lam vo Cross+F2L. Vi bat dau tu identity
    (cp=ep=(0,1,2,3)), co/eo doc duoc CHINH LA delta theo vi tri bat dau
    (xem docstring dau file)."""
    st = make_solved()
    for mv in seq:
        do_move(st, mv)
    full = from_facelets(st)
    ep, eo, cp, co = full
    valid = cross_f2l_ok(full)
    return eo[0:4], co[0:4], valid


def _table_key_for(seq):
    """Tra ve (eo_key, co_key, hop le) -- pattern SLOT-INDEXED THAT SU ma
    seq giai duoc (xem bai hoc 2 trong docstring dau file: key = -delta)."""
    eo_delta, co_delta, valid = _delta_of(seq)
    eo_key = tuple((-x) % 2 for x in eo_delta)
    co_key = tuple((-x) % 3 for x in co_delta)
    return eo_key, co_key, valid


def verify_and_build_table():
    """Kiem chung TAT CA thuat toan trong _RAW_ALGS bang cach tinh key
    that su (xem _table_key_for), chi giu lai cai HOP LE. Chay 1 lan khi
    import module."""
    table = {}
    name_table = {}
    verified_names = []
    rejected_names = []
    for name, seqstr in _RAW_ALGS.items():
        seq = seqstr.split()
        eo_key, co_key, valid = _table_key_for(seq)
        if not valid:
            rejected_names.append(name)
            continue
        table[(eo_key, co_key)] = seq
        name_table[(eo_key, co_key)] = name
        verified_names.append(name)
    return table, name_table, verified_names, rejected_names


OLL_TABLE, OLL_TABLE_NAME, VERIFIED_ALG_NAMES, REJECTED_ALG_NAMES = verify_and_build_table()

# Bang cong thuc OLL "sach" (ten -> chuoi Singmaster goc, khong doi guong/
# nghich dao) chi gom cac thuat toan DA KIEM CHUNG, dung de hien thi trong
# UI (bang tra cuu cong thuc). Luu y: hien tai CHUA phai 57/57 case day du,
# xem docstring dau file.
OLL_ALGS_FOR_DISPLAY = {name: _RAW_ALGS[name] for name in VERIFIED_ALG_NAMES}


def solve_oll_lookup(full):
    """Tra bang truc tiep (KHONG AUF -- goi ham nay 4 lan voi 4 ban xoay
    U/U2/U'/khong xoay cua state, hoac dung solve_oll_with_auf() ben
    duoi) cho trang thai full hien tai. Tra ve list nuoc di hoac None."""
    ep, eo, cp, co = full
    from .oll_recognition import _slot_indexed
    eo_slot = tuple(_slot_indexed(ep, eo))
    co_slot = tuple(_slot_indexed(cp, co))
    return OLL_TABLE.get((eo_slot, co_slot))


def solve_oll_with_auf(full):
    """Thu ca 4 AUF (khong xoay/U/U2/U'), tra ve (auf_prefix, solve_moves,
    case_name) cho lan dau tien tim thay trong bang, hoac None neu khong
    case nao khop. auf_prefix can duoc AP DUNG TRUOC solve_moves khi thuc
    thi that. case_name (vd 'Sune', 'AntiSune'...) dung de HIEN THI trong
    UI -- KHONG anh huong logic giai."""
    ep, eo, cp, co = full
    from .full_state import apply_move
    from .oll_recognition import _slot_indexed
    for auf in ['', 'U', 'U2', "U'"]:
        state = full if auf == '' else apply_move(full, auf)
        sep, seo, scp, sco = state
        eo_slot = tuple(_slot_indexed(sep, seo))
        co_slot = tuple(_slot_indexed(scp, sco))
        key = (eo_slot, co_slot)
        mvs = OLL_TABLE.get(key)
        if mvs is not None:
            prefix = [] if auf == '' else [auf]
            return prefix, mvs, OLL_TABLE_NAME.get(key)
    return None


if __name__ == '__main__':
    print(f'Da kiem chung: {len(VERIFIED_ALG_NAMES)}/{len(_RAW_ALGS)} cong thuc OLL hop le')
    print('  Hop le:', VERIFIED_ALG_NAMES)
    if REJECTED_ALG_NAMES:
        print('  BI LOAI (kiem chung sai):', REJECTED_ALG_NAMES)
    print(f'Kich thuoc bang tra cuu: {len(OLL_TABLE)} case')
