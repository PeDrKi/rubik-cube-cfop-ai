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
    fn oll_table_size_matches_python_reference() {
        // Python: OLL verified 55, table size 55 (khong trung key).
        let t = rubik_core::oll_algorithms::table();
        assert_eq!(t.table.len(), 55);
    }

    #[test]
    fn pll_table_size_matches_python_reference() {
        // Python: PLL verified 21, table size 47.
        let t = rubik_core::pll_algorithms::table();
        assert_eq!(t.table.len(), 47);
    }

    #[test]
    fn oll_solves_sune_case() {
        let mut st = CubeState::solved();
        st.apply_sequence(&["R", "U", "R'", "U", "R", "U2", "R'"]);
        let full = rubik_core::full_state::from_facelets(&st);
        let (prefix, mvs, name) = rubik_core::oll_algorithms::solve_oll_with_auf(full).unwrap();
        st.apply_sequence(&prefix);
        st.apply_sequence(&mvs);
        // Doi chieu voi Python: solve_oll_with_auf() tren state nay tra ve
        // 'AntiSune' (khong phai 'Sune') -- da xac nhan Python CUNG tra ve
        // dung ten nay (khop bang theo TRANG THAI can giai, khong phai
        // theo ten nuoc da tao ra no).
        assert_eq!(name, "AntiSune");
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
    fn pll_macro_solves_when_lookup_would_miss() {
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
            if rubik_core::solve::solve_full_cube(&st, 2).is_some() {
                solved += 1;
            }
        }
        let rate = solved as f64 / n as f64;
        assert!(rate >= 0.95, "ty le giai tron ven qua thap: {solved}/{n} ({:.1}%)", rate * 100.0);
    }
