//! breakdown.rs — port của cfop_ai.full_solve_breakdown() (Python).
//!
//! Khác `solve::solve_full_cube` (chỉ trả về 1 chuỗi nước gộp), hàm này
//! trả về lời giải TÁCH THEO TỪNG CHẶNG CFOP kèm trạng thái, để hiển thị
//! trong bảng công thức (phím T).

use crate::cross_solver;
use crate::cube::CubeState;
use crate::f2l_solver;
use crate::full_state::{self, F2L_ORDER};
use crate::macro_solver;
use crate::move_simplify;
use crate::oll_algorithms;
use crate::oll_solver;
use crate::pll_algorithms;

#[derive(Clone, Debug, PartialEq)]
pub enum StageStatus {
    /// Chặng này vốn đã xong sẵn, không cần nước nào.
    Done,
    /// Giải được — kèm chuỗi nước và tên thế (nếu có).
    Moves { moves: Vec<String>, case_name: Option<String> },
    /// Không tìm được trong ngân sách hiện tại.
    Failed,
    /// Chưa tới lượt (chặng trước chưa xong nên không tính được).
    Pending,
}

pub struct Breakdown {
    pub cross: StageStatus,
    /// Theo đúng thứ tự F2L_ORDER: DFR, DFL, DBR, DBL.
    pub f2l: [(&'static str, StageStatus); 4],
    pub oll: StageStatus,
    pub pll: StageStatus,
}

impl Breakdown {
    fn all_pending() -> Self {
        Breakdown {
            cross: StageStatus::Pending,
            f2l: [
                ("DFR", StageStatus::Pending),
                ("DFL", StageStatus::Pending),
                ("DBR", StageStatus::Pending),
                ("DBL", StageStatus::Pending),
            ],
            oll: StageStatus::Pending,
            pll: StageStatus::Pending,
        }
    }

    /// Tổng số nước của toàn bộ lời giải (bỏ qua chặng chưa giải được).
    pub fn total_moves(&self) -> usize {
        let n = |s: &StageStatus| match s {
            StageStatus::Moves { moves, .. } => moves.len(),
            _ => 0,
        };
        n(&self.cross)
            + self.f2l.iter().map(|(_, s)| n(s)).sum::<usize>()
            + n(&self.oll)
            + n(&self.pll)
    }
}

fn to_strings(v: &[&'static str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

/// Tính lời giải đầy đủ, tách theo chặng. Không thay đổi `state`.
pub fn full_solve_breakdown(state: &CubeState) -> Breakdown {
    let mut out = Breakdown::all_pending();
    let mut s = *state;

    // ── 1) Cross ────────────────────────────────────────────────────────
    let full0 = full_state::from_facelets(&s);
    if full_state::cross_ok(&full0) {
        out.cross = StageStatus::Done;
    } else {
        let mvs = cross_solver::solve_cross(&s, 20);
        s.apply_sequence(&mvs);
        out.cross = StageStatus::Moves {
            moves: move_simplify::simplify(&mvs),
            case_name: None,
        };
    }

    // ── 2) F2L (4 cặp) ──────────────────────────────────────────────────
    let full_f2l = full_state::from_facelets(&s);
    let res = f2l_solver::solve_f2l_seeded(full_f2l, &[8, 10, 12, 14], 400_000, None);
    for (i, slot) in F2L_ORDER.iter().enumerate() {
        let st = match res.per_slot.get(slot) {
            Some(Some(mvs)) if mvs.is_empty() => StageStatus::Done,
            Some(Some(mvs)) => StageStatus::Moves {
                moves: move_simplify::simplify(mvs),
                case_name: None,
            },
            Some(None) => StageStatus::Failed,
            None => StageStatus::Pending,
        };
        out.f2l[i] = (slot, st);
    }
    if res.solved_slots.len() != 4 {
        // F2L chưa xong thì OLL/PLL không tính được -> để Pending.
        return out;
    }
    s.apply_sequence(&res.moves);

    // ── 3) OLL ──────────────────────────────────────────────────────────
    let full_oll = full_state::from_facelets(&s);
    if full_state::u_edges_oriented(&full_oll) && full_state::u_corners_oriented(&full_oll) {
        out.oll = StageStatus::Done;
    } else if let Some((prefix, mvs, name)) = oll_algorithms::solve_oll_with_auf(full_oll) {
        let mut all = prefix;
        all.extend(mvs);
        s.apply_sequence(&all);
        out.oll = StageStatus::Moves {
            moves: move_simplify::simplify(&all),
            case_name: Some(name.to_string()),
        };
    } else {
        let fallback = if full_state::u_edges_oriented(&full_oll) {
            macro_solver::solve_ocll_macro(full_oll, 4)
        } else {
            oll_solver::solve_oll_search(full_oll, None)
        };
        match fallback {
            Some(mvs) => {
                s.apply_sequence(&mvs);
                out.oll = StageStatus::Moves {
                    moves: move_simplify::simplify(&mvs),
                    case_name: Some("2-look".to_string()),
                };
            }
            None => {
                out.oll = StageStatus::Failed;
                return out;
            }
        }
    }

    // ── 4) PLL ──────────────────────────────────────────────────────────
    if s.is_solved() {
        out.pll = StageStatus::Done;
        return out;
    }
    let full_pll = full_state::from_facelets(&s);
    let (mvs, name) = pll_algorithms::solve_pll_lookup_named(full_pll);
    out.pll = match mvs {
        Some(m) if m.is_empty() => StageStatus::Done,
        Some(m) => StageStatus::Moves {
            moves: move_simplify::simplify(&m),
            case_name: name.map(|n| n.to_string()),
        },
        None => match macro_solver::solve_pll_macro(full_pll, 4) {
            Some(m) => StageStatus::Moves {
                moves: move_simplify::simplify(&m),
                case_name: Some("macro".to_string()),
            },
            None => StageStatus::Failed,
        },
    };

    out
}
