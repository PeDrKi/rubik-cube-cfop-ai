"""
solver/oll_recognition.py
============================
Nhan dien CASE OLL dua tren PATTERN STICKER (mau sac) cua 8 vi tri lop U,
KHONG can biet cong thuc giai -- day la cach chinh nguoi choi CFOP that
nhan dien case (nhin hinh dang cac o "da len mau U" vs "chua len mau"),
khong phai tra bang tu chuoi nuoc di.

CO SO TOAN HOC (da kiem chung bang code, khong phai suy doan):
  - Tong huong 4 canh U (mod 2) LUON = 0 khi Cross+F2L da xong (dinh luat
    parity toan cuc cua nhom Rubik). Suy ra so canh U da dinh huong dung
    CHI CO THE la 0, 2, hoac 4 -- khong bao gio la 1 hoac 3.
  - Tong huong 4 goc U (mod 3) LUON = 0 khi Cross+F2L da xong (dinh luat
    parity goc toan cuc). Suy ra 4 gia tri huong goc (moi gia tri
    thuoc {0,1,2}) luon cong lai chia het cho 3.
Ca hai dinh luat nay LA HE QUA TOAN HOC BAT BUOC cua nhom Rubik (khong
phai dac thu cua engine nay), nen phan loai duoi day KHONG co rui ro sai
do nho nham cong thuc -- chi la thong ke to hop thuan tuy.

Vi tri (lay tu solver/edge_model.py, solver/corner_model.py):
  eo[0]=UF  eo[1]=UB  eo[2]=UL  eo[3]=UR
  co[0]=UFR co[1]=UFL co[2]=UBR co[3]=UBL

So do khong gian (nhin tu tren xuong, F o duoi -- dung quy uoc chuan cua
tai lieu tham khao OLL):
        UBL --- UB --- UBR
         |               |
        UL      *       UR
         |               |
        UFL --- UF --- UFR
"""

from .full_state import from_facelets, apply_move

EDGE_POS = ['UF', 'UB', 'UL', 'UR']
CORNER_POS = ['UFR', 'UFL', 'UBR', 'UBL']


def _slot_indexed(pos_array, ori_array, n=4):
    """QUAN TRONG: cp/co (va ep/eo) trong engine nay la PIECE-INDEXED
    (cp[j] = vi tri HIEN TAI cua vien j, co[j] = huong cua vien j) --
    KHONG PHAI slot-indexed. Ham nay chuyen doi sang slot-indexed (huong
    dang hien thi TAI moi vi tri vat ly hien tai), can thiet de ve hinh/
    nhan dien dung nhu mat nguoi nhin thay tren cube that.

    (Bug nay tung xuat hien trong ban dau cua file: da phat hien khi test
    voi 1 case OLL that su bi hoan vi (khac cube vua giai xong ap thang 1
    thuat toan, luc do cp con la identity nen 2 quy uoc trung nhau, che
    mat bug) -- xem CHANGELOG_SESSION.md.)
    """
    slot_ori = [None] * n
    for piece_id in range(n):
        slot = pos_array[piece_id]
        if slot < n:
            slot_ori[slot] = ori_array[piece_id]
    return slot_ori


def _edge_shape(eo_slot):
    """Phan loai hinh dang canh (4 nhom chuan, dinh luat parity dam bao
    chi co the la 1 trong 4 truong hop nay -- khong con truong hop nao
    khac). NHAN VAO eo DA CHUYEN DOI slot-indexed (xem _slot_indexed).
    Tra ve (ten_hinh, mo ta)."""
    oriented = [EDGE_POS[i] for i in range(4) if eo_slot[i] == 0]
    n = len(oriented)
    if n == 4:
        return 'AllOriented', 'Ca 4 canh da dung huong (chi con goc can xoay -- day la OCLL, xem solver/ocll_algorithms.py)'
    if n == 0:
        return 'Dot', 'Khong canh nao dung huong (hinh Dot/Cham -- kho nhat, can xoay ca 4 canh)'
    if n == 2:
        pair = set(oriented)
        if pair == {'UF', 'UB'} or pair == {'UL', 'UR'}:
            axis = 'F-B' if pair == {'UF', 'UB'} else 'L-R'
            return 'Line', f'2 canh doi dien dung huong (hinh Line/Vach doc theo truc {axis})'
        else:
            corner_between = {
                frozenset({'UF', 'UR'}): 'UFR', frozenset({'UF', 'UL'}): 'UFL',
                frozenset({'UB', 'UR'}): 'UBR', frozenset({'UB', 'UL'}): 'UBL',
            }[frozenset(pair)]
            return 'Angle', f'2 canh ke nhau dung huong (hinh Angle/Goc L, dinh goc phia {corner_between})'
    raise AssertionError(f'So canh dung huong = {n} -- VI PHAM dinh luat parity, kiem tra lai state dau vao')


def _corner_pattern(co_slot):
    """Tra ve tuple huong 4 goc theo thu tu UFR,UFL,UBR,UBL, DA la
    slot-indexed (xem _slot_indexed). Moi gia tri 0 (dung huong) / 1
    (xoay CW) / 2 (xoay CCW xet tu tren xuong)."""
    return tuple(co_slot[0:4])


def _normalize_auf(full):
    """Thu ca 4 phep xoay AUF (U/U2/U'/khong xoay) tren state hien tai,
    tra ve BIEU DIEN CHUAN HOA (tuple nho nhat theo thu tu tu dien) cua
    (edge_shape, corner_pattern) -- de 2 state chi khac nhau boi AUF duoc
    nhan dien la CUNG 1 case (dung nhu nguoi choi CFOP lam: xoay U truoc
    khi nhan dien, khong quan tam huong U hien tai).

    QUAN TRONG (da sua loi 2026-07): so sanh de chon dai dien PHAI dua tren
    CA CAP (eo_slot, co_slot) DAY DU, KHONG duoc rut gon qua shape_name
    truoc khi so sanh. shape_name chi la 1 NHAN CHUNG CHUNG (vd "Angle")
    dung chung cho nhieu sap xep canh khac nhau (VD: {UF,UL} dung huong
    va {UB,UR} dung huong deu la "Angle" nhung la 2 truong hop KHAC NHAU
    ve mat vi tri). Neu chi so sanh (shape_name, pattern) nhu truoc day,
    2 state co tuong quan canh-goc THAT SU KHAC NHAU (khong the quay ve
    nhau bang AUF) van co the trung (shape_name, pattern) MOT CACH TINH
    CO, gay gop nham thanh 1 case -- da kiem chung bang toan hoc: 1 nhom
    AUF bac 4 KHONG THE tao orbit lon hon 4 phan tu, nhung phien ban loi
    cho ra orbit size 8/16 (bat kha thi ve mat ly thuyet nhom) => xac nhan
    day la loi that, da sua bang cach so sanh tren (eo_slot, co_slot) day
    du (giu nguyen vi tri, khong rut gon)."""
    candidates = []
    for auf in ['', 'U', 'U2', "U'"]:
        state = full if auf == '' else apply_move(full, auf)
        ep, eo, cp, co = state
        eo_slot = tuple(_slot_indexed(ep, eo))
        co_slot = tuple(_slot_indexed(cp, co))
        candidates.append((eo_slot, co_slot, auf))
    # chon dai dien chuan hoa (nho nhat theo thu tu tu dien cua CA CAP
    # (eo_slot, co_slot) day du) -- dam bao 2 case AUF-tuong duong luon ra
    # cung 1 ket qua chuan hoa bat ke dang xoay U ban dau la gi, VA khong
    # gop nham 2 case that su khac nhau.
    candidates.sort(key=lambda x: (x[0], x[1]))
    return candidates[0]


def identify_oll_case(full):
    """Nhan dien case OLL hien tai tu trang thai full (yeu cau Cross+F2L
    da xong -- khong kiem tra lai o day, goi noi tu cfop_ai/oll_solver
    noi da dam bao dieu kien nay).

    Tra ve dict:
      shape: ten hinh dang canh (AllOriented/Dot/Line/Angle)
      shape_desc: mo ta tieng Viet
      corner_pattern: tuple 4 gia tri huong goc (UFR,UFL,UBR,UBL) O TRANG
        THAI HIEN TAI (chua chuan hoa AUF)
      normalized_key: (shape, norm_eo_slot, norm_co_slot) DA CHUAN HOA AUF
        -- DUNG CAI NAY de so sanh 2 case co "giong nhau" khong (bat ke
        dang xoay U). Gom day du eo/co slot (khong chi shape+pattern rut
        gon) de tranh gop nham 2 case khac nhau (xem _normalize_auf).
      auf_needed: so nuoc U can xoay truoc de dua ve dang chuan hoa
      is_solved: da dinh huong xong ca canh lan goc chua
    """
    ep, eo, cp, co = full
    eo_slot = _slot_indexed(ep, eo)
    co_slot = _slot_indexed(cp, co)
    shape_name, shape_desc = _edge_shape(eo_slot)
    corner_pattern = _corner_pattern(co_slot)
    norm_eo, norm_co, auf = _normalize_auf(full)
    norm_shape, _ = _edge_shape(list(norm_eo))
    is_solved = (shape_name == 'AllOriented' and corner_pattern == (0, 0, 0, 0))
    return {
        'shape': shape_name,
        'shape_desc': shape_desc,
        'corner_pattern': corner_pattern,
        'normalized_key': (norm_shape, norm_eo, norm_co),
        'auf_needed': auf,
        'is_solved': is_solved,
    }


def sticker_grid(full):
    """Tra ve luoi 3x3 (list-of-list) bieu dien 8 sticker lop U + tam,
    moi o la 'oriented' (mau U dung huong) hoac 'twisted_cw'/'twisted_ccw'
    (goc, chua dung huong) hoac 'off' (canh chua dung huong) hoac 'center'.
    Dung de VE HINH truc quan (xem generate_oll_svg())."""
    ep, eo, cp, co = full
    eo_slot = _slot_indexed(ep, eo)
    co_slot = _slot_indexed(cp, co)
    grid = [[None]*3 for _ in range(3)]
    grid[1][1] = 'center'
    # canh: UB=(0,1) UL=(1,0) UR=(1,2) UF=(2,1)
    edge_pos_grid = {'UB': (0, 1), 'UL': (1, 0), 'UR': (1, 2), 'UF': (2, 1)}
    for i, name in enumerate(EDGE_POS):
        r, c = edge_pos_grid[name]
        grid[r][c] = 'oriented' if eo_slot[i] == 0 else 'off'
    # goc: UBL=(0,0) UBR=(0,2) UFL=(2,0) UFR=(2,2)
    corner_pos_grid = {'UBL': (0, 0), 'UBR': (0, 2), 'UFL': (2, 0), 'UFR': (2, 2)}
    for i, name in enumerate(CORNER_POS):
        r, c = corner_pos_grid[name]
        if co_slot[i] == 0:
            grid[r][c] = 'oriented'
        elif co_slot[i] == 1:
            grid[r][c] = 'twisted_cw'
        else:
            grid[r][c] = 'twisted_ccw'
    return grid


if __name__ == '__main__':
    import sys
    sys.path.insert(0, '.')
    from cube_engine import make_solved, do_move

    print('=== Test 1: cube da giai (khong scramble gi) ===')
    st = make_solved()
    full = from_facelets(st)
    r = identify_oll_case(full)
    print(r)
    assert r['is_solved'], 'LOI: cube da giai phai duoc nhan la is_solved=True'

    print()
    print('=== Test 2: sau khi ap Sune (tao 1 case OLL that) ===')
    st = make_solved()
    for mv in "R U R' U R U2 R'".split():
        do_move(st, mv)
    full = from_facelets(st)
    r = identify_oll_case(full)
    print(r)
    assert r['shape'] == 'AllOriented', 'LOI: Sune la OCLL, canh phai da dung huong het'

    print()
    print('=== Test 3: kiem tra AUF-normalization dung ===')
    st = make_solved()
    for mv in "R U R' U R U2 R' U".split():   # Sune + 1 nuoc U them
        do_move(st, mv)
    full2 = from_facelets(st)
    r2 = identify_oll_case(full2)
    print(r2)
    assert r['normalized_key'] == r2['normalized_key'], \
        'LOI: cung 1 case (chi lech AUF) phai cho ra normalized_key GIONG NHAU'
    print()
    print('TAT CA TEST PASS')
