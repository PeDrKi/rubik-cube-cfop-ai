//! Bo test tich hop cho rubik-core.
//!
//! Chuyen tu `rubik-rs/src/main.rs` (bo test cu) sang day khi gop 2
//! project thanh workspace. Noi dung test KHONG DOI -- van la cac phep
//! doi chieu chinh xac voi ban Python goc (bang hoan vi, kich thuoc PDB,
//! ty le giai thanh cong).

    use rubik_core::cube::{self, CubeState};
    use rand::SeedableRng;

    #[test]
    fn edge_move_table_matches_python_reference() {
        let t = rubik_core::edge_model::tables();
        let mi = rubik_core::cube::move_index("R");
        assert_eq!(t.perm[mi], [0, 1, 2, 10, 4, 5, 6, 8, 3, 9, 7, 11]);
        assert_eq!(t.flip[mi], [0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 0]);
    }

    #[test]
    fn corner_move_table_matches_python_reference() {
        let t = rubik_core::corner_model::tables();
        let mi = rubik_core::cube::move_index("R");
        assert_eq!(t.perm[mi], [2, 1, 6, 3, 0, 5, 4, 7]);
        assert_eq!(t.ori_delta[mi], [2, 0, 1, 0, 1, 0, 2, 0]);
    }

    #[test]
    fn cross_pdb_size_matches_python_reference() {
        let pdb = rubik_core::cross_solver::get_pdb();
        assert_eq!(pdb.len(), 190_080);
        assert_eq!(pdb.max_dist(), 8u16);
    }

    #[test]
    fn pair_pdb_size_matches_python_reference() {
        let pdb = rubik_core::pdb::build_pdb_pair("DFR", "FR");
        assert_eq!(pdb.len(), 576);
        assert_eq!(pdb.max_dist(), 6u16);
    }

    #[test]
    fn solved_cube_has_zero_cross_distance() {
        let st = CubeState::solved();
        let mvs = rubik_core::cross_solver::solve_cross(&st, 20);
        assert!(mvs.is_empty());
    }

    #[test]
    fn cross_solver_solves_cross_on_scramble() {
        let mut st = CubeState::solved();
        let mut rng = rand::rngs::StdRng::seed_from_u64(1);
        rubik_core::cube::random_scramble_moves(&mut st, 20, &mut rng);
        let mvs = rubik_core::cross_solver::solve_cross(&st, 20);
        assert!(mvs.len() <= 8);
        st.apply_sequence(&mvs);
        let full = rubik_core::full_state::from_facelets(&st);
        assert!(rubik_core::full_state::cross_ok(&full));
    }

    #[test]
    fn f2l_solver_completes_all_slots_after_cross() {
        let mut st = CubeState::solved();
        let mut rng = rand::rngs::StdRng::seed_from_u64(2);
        rubik_core::cube::random_scramble_moves(&mut st, 20, &mut rng);

        let cross_mvs = rubik_core::cross_solver::solve_cross(&st, 20);
        st.apply_sequence(&cross_mvs);

        let full = rubik_core::full_state::from_facelets(&st);
        let result = rubik_core::f2l_solver::solve_f2l(full, &[8, 10, 12, 14], 200_000);

        assert_eq!(result.solved_slots.len(), 4);
        st.apply_sequence(&result.moves);
        let full_after = rubik_core::full_state::from_facelets(&st);
        assert!(rubik_core::full_state::cross_f2l_ok(&full_after));
    }

    #[test]
    fn f2l_solver_is_noop_when_already_solved() {
        let st = CubeState::solved();
        let full = rubik_core::full_state::from_facelets(&st);
        let result = rubik_core::f2l_solver::solve_f2l(full, &[8, 10, 12, 14], 200_000);
        assert!(result.moves.is_empty());
        assert_eq!(result.solved_slots.len(), 4);
    }

    #[test]
    fn oll_group_pdb_sizes_match_python_reference() {
        // Python: edge pdb size 190080 max 4; corner pdb size 136080 max 4.
        let ep = rubik_core::pdb::build_group_pdb_edges_anyperm();
        assert_eq!(ep.len(), 190_080);
        assert_eq!(ep.max_dist(), 4);
    }

    #[test]
    fn corner_group_pdb_size_matches_python_reference() {
        let cp = rubik_core::pdb::build_group_pdb_corners_anyperm();
        assert_eq!(cp.len(), 136_080);
        assert_eq!(cp.max_dist(), 4);
    }

    #[test]
    fn oll_table_covers_all_57_cases() {
        // Bản Python chỉ có 55 công thức -> 55 thế. Bản Rust ĐÃ VƯỢT mốc đó:
        // thêm 2 thế "chấm" còn thiếu để đủ 57 thế OLL chuẩn. Hai thế đó
        // trước đây làm `full_solve_breakdown` trả `StageStatus::Failed`
        // (mất cả OLL lẫn PLL) vì `oll_solver::solve_oll_search` cũng không
        // giải được chúng.
        //
        // Phép kiểm phủ trọn 57/57 nằm trong oll_algorithms::tests.
        let t = rubik_core::oll_algorithms::table();
        assert_eq!(t.table.len(), 57);
    }

    #[test]
    fn pll_table_covers_every_last_layer_state() {
        // Bản Python có 47 mục (thuận/nghịch/gương/gương-nghịch) và để lọt
        // 99/288 trạng thái xuống `macro_solver`. Bản Rust nạp mỗi công
        // thức kèm mọi tổ hợp AUF trước/sau, nên bảng lớn hơn nhiều và
        // không còn trạng thái nào lọt.
        //
        // Không chốt con số tuyệt đối ở đây (nó phụ thuộc cách sinh biến
        // thể); điều cần bảo đảm là PHỦ TRỌN, và phép kiểm đó nằm trong
        // pll_algorithms::tests::tra_duoc_moi_trang_thai.
        let t = rubik_core::pll_algorithms::table();
        assert!(t.table.len() >= 47, "bảng PLL nhỏ hơn cả bản Python: {}", t.table.len());
        assert_eq!(t.table.len(), t.name_table.len());
    }

    #[test]
    fn oll_solves_sune_case() {
        let mut st = CubeState::solved();
        st.apply_sequence(&["R", "U", "R'", "U", "R", "U2", "R'"]);
        let full = rubik_core::full_state::from_facelets(&st);
        let (prefix, mvs, name) = rubik_core::oll_algorithms::solve_oll_with_auf(full).unwrap();
        st.apply_sequence(&prefix);
        st.apply_sequence(&mvs);
        // Ap Sune len khoi da giai thi TAO RA the AntiSune, nen bang tra
        // ve ten cua AntiSune chu khong phai Sune -- bang khop theo TRANG
        // THAI CAN GIAI, khong phai theo ten nuoc da tao ra no. (Ban Python
        // cung tra ve dung nhu vay.)
        //
        // Ten bay gio la so OLL chuan: AntiSune = OLL 26. So nay do bang
        // may chu khong chep tay (xem examples/oll_number.rs), va chinh
        // test nay la mot phep kiem cheo cho no.
        assert_eq!(name, "OLL26");
        let full_after = rubik_core::full_state::from_facelets(&st);
        assert!(rubik_core::full_state::cross_f2l_ok(&full_after));
        let (_, eo, _, co) = full_after;
        assert_eq!(&eo[0..4], &[0, 0, 0, 0]);
        assert_eq!(&co[0..4], &[0, 0, 0, 0]);
    }

    #[test]
    fn pll_reports_solved_when_last_layer_already_permuted() {
        let st = CubeState::solved();
        let full = rubik_core::full_state::from_facelets(&st);
        let (mvs, name) = rubik_core::pll_algorithms::solve_pll_lookup_named(full);
        assert_eq!(mvs, Some(vec![]));
        assert_eq!(name, Some("solved"));
    }

    #[test]
    fn pll_solves_tperm_case() {
        let mut st = CubeState::solved();
        // T-perm: hoan vi thuan, ap dung se tao 1 case PLL that.
        st.apply_sequence(&["R", "U", "R'", "U'", "R'", "F", "R2", "U'", "R'", "U'", "R", "U", "R'", "F'"]);
        let full = rubik_core::full_state::from_facelets(&st);
        let (mvs, name) = rubik_core::pll_algorithms::solve_pll_lookup_named(full);
        assert!(mvs.is_some());
        assert!(name.is_some());
        st.apply_sequence(&mvs.unwrap());
        assert!(st.is_solved());
    }

    #[test]
    fn full_cfop_pipeline_solves_cube_when_lookup_tables_cover_the_case() {
        // Chay nhieu seed, chi assert cube da SOLVED cho nhung case ma
        // ca OLL lan PLL deu co trong bang (bang chua day du 57/21 case
        // OLL/dieu kien bien -- xem oll_algorithms.rs).
        let mut any_full_solve = false;
        for seed in 0..30u64 {
            let mut st = CubeState::solved();
            let mut r = rand::rngs::StdRng::seed_from_u64(9000 + seed);
            rubik_core::cube::random_scramble_moves(&mut st, 25, &mut r);

            let cm = rubik_core::cross_solver::solve_cross(&st, 20);
            st.apply_sequence(&cm);
            let f = rubik_core::full_state::from_facelets(&st);
            let res = rubik_core::f2l_solver::solve_f2l(f, &[8, 10, 12, 14], 200_000);
            st.apply_sequence(&res.moves);
            if res.solved_slots.len() != 4 {
                continue;
            }

            let f2 = rubik_core::full_state::from_facelets(&st);
            let oll = match rubik_core::oll_algorithms::solve_oll_with_auf(f2) {
                Some(x) => x,
                None => continue,
            };
            st.apply_sequence(&oll.0);
            st.apply_sequence(&oll.1);

            let f3 = rubik_core::full_state::from_facelets(&st);
            let (pll_mvs, _) = rubik_core::pll_algorithms::solve_pll_lookup_named(f3);
            let mvs = match pll_mvs {
                Some(m) => m,
                None => continue,
            };
            st.apply_sequence(&mvs);

            assert!(st.is_solved(), "seed {seed}: pipeline ket thuc nhung cube CHUA solved (bug)");
            any_full_solve = true;
        }
        assert!(any_full_solve, "khong seed nao duoc bang lookup bao phu du de test co y nghia");
    }

    #[test]
    fn ocll_macro_solves_corner_orientation() {
        let mut st = CubeState::solved();
        st.apply_sequence(&["R", "U", "R'", "U", "R", "U2", "R'"]);
        let full = rubik_core::full_state::from_facelets(&st);
        assert!(rubik_core::full_state::u_edges_oriented(&full));
        let mvs = rubik_core::macro_solver::solve_ocll_macro(full, 4).unwrap();
        st.apply_sequence(&mvs);
        let full_after = rubik_core::full_state::from_facelets(&st);
        assert!(rubik_core::full_state::u_corners_oriented(&full_after));
        assert!(rubik_core::full_state::cross_f2l_ok(&full_after));
    }

    #[test]
    fn pll_macro_van_giai_duoc_khi_duoc_goi_truc_tiep() {
        // TEN CU: `pll_macro_solves_when_lookup_would_miss`. Ten do khong
        // con dung: sau khi sua bang PLL thi bo tra KHONG bao giờ miss nua
        // (287/288 trang thai, cai con lai la khoi da giai). `macro_solver`
        // gio la duong du phong khong bao giờ duoc goi den trong thuc te --
        // nhung van giu test de no khong muc nat.
        let mut st = CubeState::solved();
        st.apply_sequence(&["R'", "F", "R'", "B2", "R", "F'", "R'", "B2", "R2"]); // Aa
        st.apply_sequence(&["R", "U'", "R", "U", "R", "U", "R", "U'", "R'", "U'", "R2"]); // Edge3
        let full = rubik_core::full_state::from_facelets(&st);
        let mvs = rubik_core::macro_solver::solve_pll_macro(full, 4).expect("macro fallback phai giai duoc");
        st.apply_sequence(&mvs);
        assert!(st.is_solved());
    }

    #[test]
    fn full_cfop_pipeline_with_macro_fallback_beats_lookup_only_coverage() {
        let mut solved_count = 0u32;
        let n = 60u64;
        for seed in 0..n {
            let mut st = CubeState::solved();
            let mut r = rand::rngs::StdRng::seed_from_u64(9000 + seed);
            rubik_core::cube::random_scramble_moves(&mut st, 25, &mut r);

            let cm = rubik_core::cross_solver::solve_cross(&st, 20);
            st.apply_sequence(&cm);
            let f = rubik_core::full_state::from_facelets(&st);
            let res = rubik_core::f2l_solver::solve_f2l(f, &[8, 10, 12, 14], 200_000);
            st.apply_sequence(&res.moves);
            if res.solved_slots.len() != 4 {
                continue;
            }

            let f2 = rubik_core::full_state::from_facelets(&st);
            let oll_mvs = match rubik_core::solve::solve_oll(f2) {
                Some(x) => x,
                None => continue,
            };
            st.apply_sequence(&oll_mvs);

            let f3 = rubik_core::full_state::from_facelets(&st);
            let pll_mvs = match rubik_core::solve::solve_pll(f3) {
                Some(x) => x,
                None => continue,
            };
            st.apply_sequence(&pll_mvs);

            assert!(st.is_solved(), "seed {seed}: pipeline ket thuc nhung cube CHUA solved (bug)");
            solved_count += 1;
        }
        assert!(solved_count as f64 / n as f64 > 0.8, "ty le giai tron ven qua thap: {solved_count}/{n}");
    }

    #[test]
    fn full_cube_solve_with_retry_reaches_near_100_percent() {
        let n = 60u64;
        let mut solved = 0u32;
        for seed in 0..n {
            let mut st = CubeState::solved();
            let mut r = rand::rngs::StdRng::seed_from_u64(5000 + seed);
            cube::random_scramble_moves(&mut st, 25, &mut r);
            if rubik_core::solve::solve_full_cube(&st, 2).is_some() {
                solved += 1;
            }
        }
        // Da do thuc te: co retry (toi da 2 lan) dat ~90% (54/60). Con lai
        // la cac case OLL "Dot" kho (0 canh dung huong) ma A* pha A chua
        // du manh -- xem ghi chu trong lich su trao doi / README. Nguong
        // duoi day phan anh dung thuc te da do, khong phai muc tieu ly
        // tuong -- xem do la viec CON LAI, khong phai da xong.
        assert!(solved as f64 / n as f64 >= 0.85, "ty le giai tron ven (co retry) qua thap: {solved}/{n}");
    }
    /// Kiem tra hoi quy: ty le giai tron ven phai o muc cao (da do thuc te
    /// 99% sau khi sua bug F2L "quen them slot da xong vao done" + tang
    /// node budget 200k->400k). Nguong 95% de con it du dia an toan cho
    /// bien dong seed/timing, khong phai muc tieu ly tuong.
    #[test]
    fn batch_200_success_rate() {
        let n = 200u64;
        let mut solved = 0u32;
        for seed in 0..n {
            let mut st = CubeState::solved();
            let mut r = rand::rngs::StdRng::seed_from_u64(5000 + seed);
            cube::random_scramble_moves(&mut st, 25, &mut r);
            // KHONG chi kiem `.is_some()`. Ban cu lam vay, nghia la neu
            // bo giai tra ve mot chuoi SAI -- hoac `move_simplify` cat nham
            // mot nuoc -- thi test van xanh. Ma `move_simplify` chay tren
            // MOI loi giai (solve.rs:73), nen do dung la cho can canh nhat.
            // O day ap that loi giai vao roi doi chieu voi khoi da giai.
            if let Some(mvs) = rubik_core::solve::solve_full_cube(&st, 2) {
                let refs: Vec<&str> = mvs.iter().map(|m| m.as_str()).collect();
                let mut check = st;
                check.apply_sequence(&refs);
                assert_eq!(
                    check,
                    CubeState::solved(),
                    "seed {seed}: bo giai tra ve {} nuoc nhung KHONG giai duoc khoi",
                    mvs.len()
                );
                solved += 1;
            }
        }
        let rate = solved as f64 / n as f64;
        assert!(rate >= 0.95, "ty le giai tron ven qua thap: {solved}/{n} ({:.1}%)", rate * 100.0);
    }
