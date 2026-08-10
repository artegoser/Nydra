use crate::{ChessError, ChessRules, ChessSide, PAWN, BISHOP, KNIGHT, QUEEN, ROOK};
use glorichess_core::GameTimeline;

impl ChessRules {
    /// Counts legal leaf positions using the production move generator and
    /// make-move path. This intentionally performs no search/evaluation.
    pub fn perft(&self, timeline: &GameTimeline, depth: u8) -> Result<u64, ChessError> {
        if depth == 0 {
            return Ok(1);
        }

        let state = timeline.current();
        let history = timeline.history();
        let [active] = state.turn.active_players.as_slice() else {
            return Err(ChessError::InvalidTurnState);
        };
        let side = ChessSide::from_player(*active).ok_or(ChessError::UnknownSide(*active))?;
        let moves = self.legal_moves_for_side_with_history(state, Some(history), side)?;
        let mut nodes = 0_u64;

        for movement in moves {
            let actor = state.entity(movement.actor)?;
            let promotions: &[Option<glorichess_core::EntityTypeId>] =
                if actor.entity_type == PAWN && movement.to.y == side.promotion_rank() {
                    &[
                        Some(QUEEN),
                        Some(ROOK),
                        Some(BISHOP),
                        Some(KNIGHT),
                    ]
                } else {
                    &[None]
                };

            for promotion in promotions.iter().copied() {
                let mut child = timeline.clone();
                let mut turn = child.begin_turn(side.player())?;
                self.execute_move(&mut turn, Some(history), movement, promotion)?;
                child.commit_turn(turn)?;
                nodes = nodes.saturating_add(self.perft(&child, depth - 1)?);
            }
        }

        Ok(nodes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::standard_chess_state;

    #[test]
    fn initial_position_matches_reference_perft_through_depth_four() {
        let rules = ChessRules::standard();
        let timeline = GameTimeline::new(standard_chess_state().unwrap()).unwrap();
        for (depth, expected) in [(1, 20), (2, 400), (3, 8_902), (4, 197_281)] {
            assert_eq!(rules.perft(&timeline, depth).unwrap(), expected, "depth {depth}");
        }
    }

    #[test]
    fn kiwipete_matches_castling_heavy_reference_perft() {
        let rules = ChessRules::standard();
        let timeline = rules
            .from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1")
            .unwrap()
            .timeline;
        for (depth, expected) in [(1, 48), (2, 2_039), (3, 97_862)] {
            assert_eq!(rules.perft(&timeline, depth).unwrap(), expected, "depth {depth}");
        }
    }

    #[test]
    fn en_passant_and_discovered_check_reference_position_matches_perft() {
        let rules = ChessRules::standard();
        let timeline = rules
            .from_fen("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1")
            .unwrap()
            .timeline;
        for (depth, expected) in [(1, 14), (2, 191), (3, 2_812), (4, 43_238)] {
            assert_eq!(rules.perft(&timeline, depth).unwrap(), expected, "depth {depth}");
        }
    }

    #[test]
    fn promotion_and_castling_reference_position_matches_perft() {
        let rules = ChessRules::standard();
        let timeline = rules
            .from_fen("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1")
            .unwrap()
            .timeline;
        for (depth, expected) in [(1, 6), (2, 264), (3, 9_467)] {
            assert_eq!(rules.perft(&timeline, depth).unwrap(), expected, "depth {depth}");
        }
    }
}
