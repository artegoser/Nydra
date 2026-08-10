//! Standard chess rules implemented on top of `glorichess-core`.
#![forbid(unsafe_code)]

#[cfg(test)]
mod correctness;
mod error;
mod fen;
mod game;
mod interaction;
mod notation;
mod outcome;
mod perft;
mod piece;
mod pieces;

pub use error::ChessError;
pub use fen::{FenGame, STANDARD_FEN};
pub use game::{
    empty_chess_state, standard_chess_state, ChessRules, BLACK_PLAYER, BLACK_TEAM, WHITE_PLAYER,
    WHITE_TEAM,
};
pub use interaction::ChessInteractionRules;
pub use notation::PgnGame;
pub use outcome::{ChessDrawClaim, ChessOutcome, ChessStatus, PositionKey};
pub use piece::{
    ChessMoveKind, ChessPieceContext, ChessPieceKind, ChessPieceRule, PseudoMove, BISHOP, KING, KNIGHT, PAWN,
    QUEEN, ROOK,
};
pub use pieces::{Bishop, King, Knight, Pawn, Queen, Rook};

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChessSide {
    White,
    Black,}

#[cfg(test)]
mod tests {
    use super::*;
    use glorichess_core::{
        EntityId, EntityRule, EntityState, EntityTypeId, PlayerId, Position, RuleContext,
    };
    use std::collections::BTreeSet;

    fn add_piece(
        state: &mut glorichess_core::GameState,
        id: u32,
        entity_type: EntityTypeId,
        side: ChessSide,
        position: Position,
    ) -> EntityId {
        let id = EntityId::new(id);
        state
            .add_entity(EntityState::new(id, entity_type, side.player(), position))
            .unwrap();
        id
    }

    fn destinations(moves: &[PseudoMove]) -> BTreeSet<Position> {
        moves.iter().map(|movement| movement.to).collect()
    }

    #[test]
    fn standard_setup_contains_all_pieces_and_players() {
        let state = standard_chess_state().unwrap();
        assert_eq!(state.board.width(), 8);
        assert_eq!(state.board.height(), 8);
        assert_eq!(state.entities.len(), 32);
        assert_eq!(state.turn.active_players, vec![WHITE_PLAYER]);
        assert_eq!(
            state
                .entity_at(Position::new(4, 0))
                .unwrap()
                .unwrap()
                .entity_type,
            KING
        );
        assert_eq!(
            state
                .entity_at(Position::new(3, 7))
                .unwrap()
                .unwrap()
                .entity_type,
            QUEEN
        );
        assert_eq!(
            state
                .entity_at(Position::new(0, 1))
                .unwrap()
                .unwrap()
                .entity_type,
            PAWN
        );
        assert_eq!(
            state
                .entity_at(Position::new(7, 6))
                .unwrap()
                .unwrap()
                .entity_type,
            PAWN
        );
        state.validate().unwrap();
    }

    #[test]
    fn standard_registry_contains_all_six_piece_rules() {
        let rules = ChessRules::standard();
        for entity_type in [PAWN, KNIGHT, BISHOP, ROOK, QUEEN, KING] {
            assert!(rules.piece_rule(entity_type).is_ok());
        }
    }

    #[test]
    fn pawn_moves_attacks_captures_and_blocking_are_distinct() {
        let rules = ChessRules::standard();
        let mut state = empty_chess_state().unwrap();
        let pawn = add_piece(&mut state, 1, PAWN, ChessSide::White, Position::new(3, 1));
        let enemy = add_piece(&mut state, 2, KNIGHT, ChessSide::Black, Position::new(4, 2));

        let moves = rules.pseudo_moves(&state, pawn).unwrap();
        assert_eq!(
            destinations(&moves),
            BTreeSet::from([
                Position::new(3, 2),
                Position::new(3, 3),
                Position::new(4, 2),
            ])
        );
        assert_eq!(
            moves
                .iter()
                .find(|movement| movement.to == Position::new(4, 2))
                .unwrap()
                .capture,
            Some(enemy)
        );
        assert_eq!(
            rules
                .attacks(&state, pawn)
                .unwrap()
                .into_iter()
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([Position::new(2, 2), Position::new(4, 2)])
        );

        add_piece(&mut state, 3, BISHOP, ChessSide::White, Position::new(3, 2));
        let blocked = rules.pseudo_moves(&state, pawn).unwrap();
        assert_eq!(
            destinations(&blocked),
            BTreeSet::from([Position::new(4, 2)])
        );
    }

    #[test]
    fn pawn_double_move_requires_start_rank_and_unmoved_state() {
        let rules = ChessRules::standard();
        let mut state = empty_chess_state().unwrap();
        let pawn = add_piece(&mut state, 1, PAWN, ChessSide::White, Position::new(3, 3));
        assert_eq!(
            destinations(&rules.pseudo_moves(&state, pawn).unwrap()),
            BTreeSet::from([Position::new(3, 4)])
        );

        state.remove_entity(pawn).unwrap();
        let pawn = add_piece(&mut state, 2, PAWN, ChessSide::White, Position::new(3, 1));
        state.entity_mut(pawn).unwrap().move_count = 1;
        assert_eq!(
            destinations(&rules.pseudo_moves(&state, pawn).unwrap()),
            BTreeSet::from([Position::new(3, 2)])
        );
    }

    #[test]
    fn knight_generates_leaps_and_ignores_intervening_pieces() {
        let rules = ChessRules::standard();
        let mut state = empty_chess_state().unwrap();
        let knight = add_piece(&mut state, 1, KNIGHT, ChessSide::White, Position::new(1, 0));
        add_piece(&mut state, 2, PAWN, ChessSide::White, Position::new(1, 1));
        assert_eq!(
            destinations(&rules.pseudo_moves(&state, knight).unwrap()),
            BTreeSet::from([
                Position::new(0, 2),
                Position::new(2, 2),
                Position::new(3, 1)
            ])
        );
    }

    #[test]
    fn sliding_piece_stops_at_first_occupied_square() {
        let rules = ChessRules::standard();
        let mut state = empty_chess_state().unwrap();
        let rook = add_piece(&mut state, 1, ROOK, ChessSide::White, Position::new(3, 3));
        add_piece(&mut state, 2, PAWN, ChessSide::White, Position::new(3, 5));
        let enemy = add_piece(&mut state, 3, PAWN, ChessSide::Black, Position::new(5, 3));

        let moves = rules.pseudo_moves(&state, rook).unwrap();
        assert!(moves
            .iter()
            .any(|movement| movement.to == Position::new(3, 4)));
        assert!(!moves
            .iter()
            .any(|movement| movement.to == Position::new(3, 5)));
        assert!(!moves
            .iter()
            .any(|movement| movement.to == Position::new(3, 6)));
        assert_eq!(
            moves
                .iter()
                .find(|movement| movement.to == Position::new(5, 3))
                .unwrap()
                .capture,
            Some(enemy)
        );
        assert!(!moves
            .iter()
            .any(|movement| movement.to == Position::new(6, 3)));

        let attacks = rules.attacks(&state, rook).unwrap();
        assert!(attacks.contains(&Position::new(3, 5)));
        assert!(attacks.contains(&Position::new(5, 3)));
        assert!(!attacks.contains(&Position::new(3, 6)));
        assert!(!attacks.contains(&Position::new(6, 3)));
    }

    #[test]
    fn bishop_queen_and_king_generate_expected_geometry() {
        let rules = ChessRules::standard();
        let mut state = empty_chess_state().unwrap();
        let bishop = add_piece(&mut state, 1, BISHOP, ChessSide::White, Position::new(3, 3));
        let queen = add_piece(&mut state, 2, QUEEN, ChessSide::White, Position::new(0, 0));
        let king = add_piece(&mut state, 3, KING, ChessSide::Black, Position::new(7, 7));

        let bishop_moves = destinations(&rules.pseudo_moves(&state, bishop).unwrap());
        assert!(bishop_moves.contains(&Position::new(4, 4)));
        assert!(bishop_moves.contains(&Position::new(2, 4)));
        assert!(!bishop_moves.contains(&Position::new(3, 4)));

        let queen_moves = destinations(&rules.pseudo_moves(&state, queen).unwrap());
        assert!(queen_moves.contains(&Position::new(0, 6)));
        assert!(queen_moves.contains(&Position::new(6, 0)));
        assert!(queen_moves.contains(&Position::new(2, 2)));

        assert_eq!(rules.attacks(&state, king).unwrap().len(), 3);
    }

    #[test]
    fn standard_piece_presentation_is_state_context_driven() {
        let state = standard_chess_state().unwrap();
        let pawn = state.entity_at(Position::new(0, 1)).unwrap().unwrap();
        let rule = Pawn;
        let context = RuleContext::from_state(&state, None)
            .entity_context(pawn.id)
            .unwrap();
        let presentation = rule.presentation(context).unwrap();
        assert_eq!(presentation.asset_key, "chess/white/pawn");
    }

    #[test]
    fn side_mapping_is_chess_local() {
        assert_eq!(ChessSide::White.player(), WHITE_PLAYER);
        assert_eq!(ChessSide::Black.player(), BLACK_PLAYER);
        assert_eq!(ChessSide::from_player(WHITE_PLAYER), Some(ChessSide::White));
        assert_eq!(ChessSide::from_player(PlayerId::new(99)), None);
    }

    #[test]
    fn black_pawn_uses_the_opposite_forward_direction() {
        let rules = ChessRules::standard();
        let mut state = empty_chess_state().unwrap();
        let pawn = add_piece(&mut state, 1, PAWN, ChessSide::Black, Position::new(4, 6));
        assert_eq!(
            destinations(&rules.pseudo_moves(&state, pawn).unwrap()),
            BTreeSet::from([Position::new(4, 4), Position::new(4, 5)])
        );
        assert_eq!(
            rules
                .attacks(&state, pawn)
                .unwrap()
                .into_iter()
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([Position::new(3, 5), Position::new(5, 5)])
        );
    }

    fn legal_test_state() -> glorichess_core::GameState {
        empty_chess_state().unwrap()
    }

    #[test]
    fn attack_maps_detect_check_and_pinned_piece_moves_are_filtered() {
        let rules = ChessRules::standard();
        let mut state = legal_test_state();
        let white_king = add_piece(&mut state, 1, KING, ChessSide::White, Position::new(4, 0));
        let pinned_rook = add_piece(&mut state, 2, ROOK, ChessSide::White, Position::new(4, 1));
        add_piece(&mut state, 3, ROOK, ChessSide::Black, Position::new(4, 7));
        add_piece(&mut state, 4, KING, ChessSide::Black, Position::new(7, 7));

        assert!(!rules.in_check(&state, ChessSide::White).unwrap());
        assert!(rules.is_square_attacked(&state, ChessSide::Black, Position::new(4, 1)).unwrap());

        let pseudo = destinations(&rules.pseudo_moves(&state, pinned_rook).unwrap());
        assert!(pseudo.contains(&Position::new(3, 1)));
        let legal = destinations(&rules.legal_moves(&state, pinned_rook).unwrap());
        assert!(!legal.contains(&Position::new(3, 1)));
        assert!(!legal.contains(&Position::new(5, 1)));
        assert!(legal.contains(&Position::new(4, 2)));
        assert_eq!(rules.king(&state, ChessSide::White).unwrap(), white_king);
    }

    #[test]
    fn king_cannot_move_or_capture_into_resulting_attack() {
        let rules = ChessRules::standard();
        let mut state = legal_test_state();
        let king = add_piece(&mut state, 1, KING, ChessSide::White, Position::new(4, 0));
        let victim = add_piece(&mut state, 2, ROOK, ChessSide::Black, Position::new(4, 1));
        add_piece(&mut state, 3, ROOK, ChessSide::Black, Position::new(7, 1));
        add_piece(&mut state, 4, KING, ChessSide::Black, Position::new(7, 7));

        let pseudo = rules.pseudo_moves(&state, king).unwrap();
        assert!(pseudo.iter().any(|movement| movement.capture == Some(victim)));
        let legal = rules.legal_moves(&state, king).unwrap();
        assert!(!legal.iter().any(|movement| movement.to == Position::new(4, 1)));
    }

    #[test]
    fn double_check_leaves_only_king_actions() {
        let rules = ChessRules::standard();
        let mut state = legal_test_state();
        let king = add_piece(&mut state, 1, KING, ChessSide::White, Position::new(4, 0));
        add_piece(&mut state, 2, ROOK, ChessSide::White, Position::new(0, 0));
        add_piece(&mut state, 3, ROOK, ChessSide::Black, Position::new(4, 7));
        add_piece(&mut state, 4, BISHOP, ChessSide::Black, Position::new(1, 3));
        add_piece(&mut state, 5, KING, ChessSide::Black, Position::new(7, 7));

        assert!(rules.in_check(&state, ChessSide::White).unwrap());
        let legal = rules.legal_moves_for_side(&state, ChessSide::White).unwrap();
        assert!(!legal.is_empty());
        assert!(legal.iter().all(|movement| movement.actor == king));
    }

    #[test]
    fn chess_interaction_exposes_only_legal_destinations() {
        use glorichess_core::{ChoiceKind, InteractionDriver, TurnSession};

        let rules = ChessRules::standard();
        let mut state = legal_test_state();
        add_piece(&mut state, 1, KING, ChessSide::White, Position::new(4, 0));
        let rook = add_piece(&mut state, 2, ROOK, ChessSide::White, Position::new(4, 1));
        add_piece(&mut state, 3, ROOK, ChessSide::Black, Position::new(4, 7));
        add_piece(&mut state, 4, KING, ChessSide::Black, Position::new(7, 7));

        let turn = TurnSession::new(&state, WHITE_PLAYER).unwrap();
        let mut driver = InteractionDriver::new(ChessInteractionRules::new(&rules), turn).unwrap();
        let rook_choice = driver
            .interaction()
            .choices
            .iter()
            .find(|choice| matches!(choice.kind, ChoiceKind::SelectEntity { entity } if entity == rook))
            .unwrap()
            .id;
        driver.choose(rook_choice).unwrap();

        let is_rook_destination = |choice: &glorichess_core::Choice, target: Position| {
            matches!(choice.kind, ChoiceKind::SelectPosition { position } if position == target)
                && choice
                    .data
                    .get("actor")
                    .and_then(glorichess_core::StateValue::as_u64)
                    == Some(u64::from(rook.get()))
        };
        assert!(!driver
            .interaction()
            .choices
            .iter()
            .any(|choice| is_rook_destination(choice, Position::new(3, 1))));
        assert!(driver
            .interaction()
            .choices
            .iter()
            .any(|choice| is_rook_destination(choice, Position::new(4, 2))));
    }

    #[test]
    fn chess_destinations_are_actor_bound_and_can_execute_without_click_selection() {
        use glorichess_core::{ChoiceKind, InteractionDriver, InteractionUpdate, TurnSession};

        let rules = ChessRules::standard();
        let state = standard_chess_state().unwrap();
        let pawn = state.entity_at(Position::new(4, 1)).unwrap().unwrap().id;
        let turn = TurnSession::new(&state, WHITE_PLAYER).unwrap();
        let mut driver = InteractionDriver::new(ChessInteractionRules::new(&rules), turn).unwrap();

        let destination = driver
            .interaction()
            .choices
            .iter()
            .find(|choice| {
                matches!(choice.kind, ChoiceKind::SelectPosition { position } if position == Position::new(4, 3))
                    && choice
                        .data
                        .get("actor")
                        .and_then(glorichess_core::StateValue::as_u64)
                        == Some(u64::from(pawn.get()))
            })
            .unwrap()
            .id;

        assert_eq!(driver.choose(destination).unwrap(), InteractionUpdate::Finished);
        assert_eq!(
            driver.turn().working.entity(pawn).unwrap().position,
            Position::new(4, 3)
        );
    }

    #[test]
    fn selecting_the_same_piece_twice_clears_chess_selection() {
        use glorichess_core::{ChoiceKind, InteractionDriver, TurnSession};

        let rules = ChessRules::standard();
        let state = standard_chess_state().unwrap();
        let pawn = state.entity_at(Position::new(4, 1)).unwrap().unwrap().id;
        let turn = TurnSession::new(&state, WHITE_PLAYER).unwrap();
        let mut driver = InteractionDriver::new(ChessInteractionRules::new(&rules), turn).unwrap();

        for expected_selected in [Some(pawn), None] {
            let choice = driver
                .interaction()
                .choices
                .iter()
                .find(|choice| matches!(choice.kind, ChoiceKind::SelectEntity { entity } if entity == pawn))
                .unwrap()
                .id;
            driver.choose(choice).unwrap();
            assert_eq!(ChessInteractionRules::selected_entity(driver.draft()), expected_selected);
        }
    }

    #[test]
    fn en_passant_is_derived_from_the_previous_committed_turn() {
        use glorichess_core::GameTimeline;

        let rules = ChessRules::standard();
        let mut state = empty_chess_state().unwrap();
        add_piece(&mut state, 1, KING, ChessSide::White, Position::new(4, 0));
        let white_pawn = add_piece(&mut state, 2, PAWN, ChessSide::White, Position::new(4, 4));
        add_piece(&mut state, 3, KING, ChessSide::Black, Position::new(4, 7));
        let black_pawn = add_piece(&mut state, 4, PAWN, ChessSide::Black, Position::new(3, 6));
        state.set_active_players(vec![BLACK_PLAYER]).unwrap();

        let mut timeline = GameTimeline::new(state).unwrap();
        let mut black_turn = timeline.begin_turn(BLACK_PLAYER).unwrap();
        let double = rules
            .legal_moves(&black_turn.working, black_pawn)
            .unwrap()
            .into_iter()
            .find(|movement| movement.to == Position::new(3, 4))
            .unwrap();
        rules.execute_move(&mut black_turn, None, double, None).unwrap();
        timeline.commit_turn(black_turn).unwrap();

        let ep = rules
            .legal_moves_with_history(timeline.current(), Some(timeline.history()), white_pawn)
            .unwrap()
            .into_iter()
            .find(|movement| movement.to == Position::new(3, 5))
            .unwrap();
        assert!(matches!(ep.kind, ChessMoveKind::EnPassant { victim } if victim == black_pawn));

        let mut white_turn = timeline.begin_turn(WHITE_PLAYER).unwrap();
        rules
            .execute_move(&mut white_turn, Some(timeline.history()), ep, None)
            .unwrap();
        assert!(!white_turn.working.entities.contains_key(&black_pawn));
        assert_eq!(
            white_turn.working.entity(white_pawn).unwrap().position,
            Position::new(3, 5)
        );
    }

    #[test]
    fn black_can_en_passant_after_white_double_move() {
        use glorichess_core::GameTimeline;

        let rules = ChessRules::standard();
        let mut state = empty_chess_state().unwrap();
        add_piece(&mut state, 1, KING, ChessSide::White, Position::new(4, 0));
        let white_pawn = add_piece(&mut state, 2, PAWN, ChessSide::White, Position::new(3, 1));
        add_piece(&mut state, 3, KING, ChessSide::Black, Position::new(4, 7));
        let black_pawn = add_piece(&mut state, 4, PAWN, ChessSide::Black, Position::new(4, 3));

        let mut timeline = GameTimeline::new(state).unwrap();
        let mut white_turn = timeline.begin_turn(WHITE_PLAYER).unwrap();
        let double = rules
            .legal_moves(&white_turn.working, white_pawn)
            .unwrap()
            .into_iter()
            .find(|movement| movement.to == Position::new(3, 3))
            .unwrap();
        rules.execute_move(&mut white_turn, None, double, None).unwrap();
        timeline.commit_turn(white_turn).unwrap();

        let moves = rules
            .legal_moves_with_history(timeline.current(), Some(timeline.history()), black_pawn)
            .unwrap();
        assert!(moves.iter().any(|movement| {
            movement.to == Position::new(3, 2)
                && matches!(movement.kind, ChessMoveKind::EnPassant { victim } if victim == white_pawn)
        }));
    }

    #[test]
    fn en_passant_expires_after_the_immediate_reply() {
        use glorichess_core::GameTimeline;

        let rules = ChessRules::standard();
        let mut state = empty_chess_state().unwrap();
        let white_king = add_piece(&mut state, 1, KING, ChessSide::White, Position::new(4, 0));
        let white_pawn = add_piece(&mut state, 2, PAWN, ChessSide::White, Position::new(4, 4));
        let black_king = add_piece(&mut state, 3, KING, ChessSide::Black, Position::new(4, 7));
        let black_pawn = add_piece(&mut state, 4, PAWN, ChessSide::Black, Position::new(3, 6));
        state.set_active_players(vec![BLACK_PLAYER]).unwrap();

        let mut timeline = GameTimeline::new(state).unwrap();
        let mut black_turn = timeline.begin_turn(BLACK_PLAYER).unwrap();
        let double = rules
            .legal_moves(&black_turn.working, black_pawn)
            .unwrap()
            .into_iter()
            .find(|movement| movement.to == Position::new(3, 4))
            .unwrap();
        rules.execute_move(&mut black_turn, None, double, None).unwrap();
        timeline.commit_turn(black_turn).unwrap();

        let mut white_turn = timeline.begin_turn(WHITE_PLAYER).unwrap();
        let king_move = rules
            .legal_moves_with_history(&white_turn.working, Some(timeline.history()), white_king)
            .unwrap()
            .into_iter()
            .find(|movement| movement.to == Position::new(5, 0))
            .unwrap();
        rules
            .execute_move(&mut white_turn, Some(timeline.history()), king_move, None)
            .unwrap();
        timeline.commit_turn(white_turn).unwrap();

        let mut black_turn = timeline.begin_turn(BLACK_PLAYER).unwrap();
        let king_move = rules
            .legal_moves_with_history(&black_turn.working, Some(timeline.history()), black_king)
            .unwrap()
            .into_iter()
            .find(|movement| movement.to == Position::new(5, 7))
            .unwrap();
        rules
            .execute_move(&mut black_turn, Some(timeline.history()), king_move, None)
            .unwrap();
        timeline.commit_turn(black_turn).unwrap();

        let legal = rules
            .legal_moves_with_history(timeline.current(), Some(timeline.history()), white_pawn)
            .unwrap();
        assert!(!legal.iter().any(|movement| matches!(movement.kind, ChessMoveKind::EnPassant { .. })));
    }

    #[test]
    fn en_passant_is_rejected_when_it_exposes_the_king() {
        use glorichess_core::GameTimeline;

        let rules = ChessRules::standard();
        let mut state = empty_chess_state().unwrap();
        add_piece(&mut state, 1, KING, ChessSide::White, Position::new(7, 4));
        let white_pawn = add_piece(&mut state, 2, PAWN, ChessSide::White, Position::new(6, 4));
        add_piece(&mut state, 3, KING, ChessSide::Black, Position::new(7, 7));
        add_piece(&mut state, 4, ROOK, ChessSide::Black, Position::new(0, 4));
        let black_pawn = add_piece(&mut state, 5, PAWN, ChessSide::Black, Position::new(5, 6));
        state.set_active_players(vec![BLACK_PLAYER]).unwrap();

        let mut timeline = GameTimeline::new(state).unwrap();
        let mut black_turn = timeline.begin_turn(BLACK_PLAYER).unwrap();
        let double = rules
            .legal_moves(&black_turn.working, black_pawn)
            .unwrap()
            .into_iter()
            .find(|movement| movement.to == Position::new(5, 4))
            .unwrap();
        rules.execute_move(&mut black_turn, None, double, None).unwrap();
        timeline.commit_turn(black_turn).unwrap();

        let legal = rules
            .legal_moves_with_history(timeline.current(), Some(timeline.history()), white_pawn)
            .unwrap();
        assert!(!legal.iter().any(|movement| movement.to == Position::new(5, 5)));
    }

    #[test]
    fn castling_is_derived_from_king_and_rook_move_counts() {
        use glorichess_core::TurnSession;

        let rules = ChessRules::standard();
        let mut state = empty_chess_state().unwrap();
        let king = add_piece(&mut state, 1, KING, ChessSide::White, Position::new(4, 0));
        let queen_rook = add_piece(&mut state, 2, ROOK, ChessSide::White, Position::new(0, 0));
        let king_rook = add_piece(&mut state, 3, ROOK, ChessSide::White, Position::new(7, 0));
        add_piece(&mut state, 4, KING, ChessSide::Black, Position::new(4, 7));

        let legal = rules.legal_moves(&state, king).unwrap();
        assert!(legal.iter().any(|movement| {
            movement.to == Position::new(6, 0)
                && matches!(movement.kind, ChessMoveKind::Castle { rook, .. } if rook == king_rook)
        }));
        assert!(legal.iter().any(|movement| {
            movement.to == Position::new(2, 0)
                && matches!(movement.kind, ChessMoveKind::Castle { rook, .. } if rook == queen_rook)
        }));

        let castle = legal
            .into_iter()
            .find(|movement| movement.to == Position::new(6, 0))
            .unwrap();
        let mut turn = TurnSession::new(&state, WHITE_PLAYER).unwrap();
        rules.execute_move(&mut turn, None, castle, None).unwrap();
        assert_eq!(turn.working.entity(king).unwrap().position, Position::new(6, 0));
        assert_eq!(turn.working.entity(king_rook).unwrap().position, Position::new(5, 0));
        assert_eq!(turn.working.entity(king).unwrap().move_count, 1);
        assert_eq!(turn.working.entity(king_rook).unwrap().move_count, 1);
    }

    #[test]
    fn castling_rejects_attacked_transit_and_previously_moved_pieces() {
        let rules = ChessRules::standard();
        let mut state = empty_chess_state().unwrap();
        let king = add_piece(&mut state, 1, KING, ChessSide::White, Position::new(4, 0));
        let rook = add_piece(&mut state, 2, ROOK, ChessSide::White, Position::new(7, 0));
        add_piece(&mut state, 3, KING, ChessSide::Black, Position::new(4, 7));
        add_piece(&mut state, 4, ROOK, ChessSide::Black, Position::new(5, 7));
        assert!(!rules
            .legal_moves(&state, king)
            .unwrap()
            .iter()
            .any(|movement| movement.to == Position::new(6, 0)));

        state.remove_entity(EntityId::new(4)).unwrap();
        state.move_entity(rook, Position::new(7, 1)).unwrap();
        state.move_entity(rook, Position::new(7, 0)).unwrap();
        assert!(!rules
            .legal_moves(&state, king)
            .unwrap()
            .iter()
            .any(|movement| movement.to == Position::new(6, 0)));

        state.entity_mut(rook).unwrap().move_count = 0;
        state.move_entity(king, Position::new(4, 1)).unwrap();
        state.move_entity(king, Position::new(4, 0)).unwrap();
        assert!(!rules
            .legal_moves(&state, king)
            .unwrap()
            .iter()
            .any(|movement| movement.to == Position::new(6, 0)));
    }

    #[test]
    fn promotion_waits_for_an_explicit_interaction_choice_and_supports_underpromotion() {
        use glorichess_core::{ChoiceKind, InteractionDriver, TurnSession};

        let rules = ChessRules::standard();
        let mut state = empty_chess_state().unwrap();
        add_piece(&mut state, 1, KING, ChessSide::White, Position::new(4, 0));
        let pawn = add_piece(&mut state, 2, PAWN, ChessSide::White, Position::new(0, 6));
        add_piece(&mut state, 3, KING, ChessSide::Black, Position::new(4, 7));

        let turn = TurnSession::new(&state, WHITE_PLAYER).unwrap();
        let mut driver = InteractionDriver::new(ChessInteractionRules::new(&rules), turn).unwrap();
        let pawn_choice = driver
            .interaction()
            .choices
            .iter()
            .find(|choice| matches!(choice.kind, ChoiceKind::SelectEntity { entity } if entity == pawn))
            .unwrap()
            .id;
        driver.choose(pawn_choice).unwrap();
        let destination = driver
            .interaction()
            .choices
            .iter()
            .find(|choice| matches!(choice.kind, ChoiceKind::SelectPosition { position } if position == Position::new(0, 7)))
            .unwrap()
            .id;
        driver.choose(destination).unwrap();

        let option_keys = driver
            .interaction()
            .choices
            .iter()
            .filter_map(|choice| match &choice.kind {
                ChoiceKind::SelectOption { key } => Some(key.as_str()),
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(option_keys, BTreeSet::from(["bishop", "knight", "queen", "rook"]));
        assert_eq!(driver.turn().steps.len(), 0);

        let knight = driver
            .interaction()
            .choices
            .iter()
            .find(|choice| matches!(&choice.kind, ChoiceKind::SelectOption { key } if key == "knight"))
            .unwrap()
            .id;
        driver.choose(knight).unwrap();
        assert!(driver.is_finished());
        assert_eq!(driver.turn().steps.len(), 1);
        assert_eq!(driver.turn().working.entity(pawn).unwrap().entity_type, KNIGHT);
    }

    #[test]
    fn capture_promotion_removes_the_target_and_changes_type() {
        use glorichess_core::TurnSession;

        let rules = ChessRules::standard();
        let mut state = empty_chess_state().unwrap();
        add_piece(&mut state, 1, KING, ChessSide::White, Position::new(4, 0));
        let pawn = add_piece(&mut state, 2, PAWN, ChessSide::White, Position::new(1, 6));
        add_piece(&mut state, 3, KING, ChessSide::Black, Position::new(4, 7));
        let victim = add_piece(&mut state, 4, ROOK, ChessSide::Black, Position::new(2, 7));
        let movement = rules
            .legal_moves(&state, pawn)
            .unwrap()
            .into_iter()
            .find(|movement| movement.to == Position::new(2, 7))
            .unwrap();
        let mut turn = TurnSession::new(&state, WHITE_PLAYER).unwrap();
        rules.execute_move(&mut turn, None, movement, Some(BISHOP)).unwrap();
        assert!(!turn.working.entities.contains_key(&victim));
        assert_eq!(turn.working.entity(pawn).unwrap().entity_type, BISHOP);
    }

    fn commit_to(
        timeline: &mut glorichess_core::GameTimeline,
        rules: &ChessRules,
        entity: EntityId,
        to: Position,
    ) {
        let actor = timeline.current().turn.active_players[0];
        let mut turn = timeline.begin_turn(actor).unwrap();
        let movement = rules
            .legal_moves_with_history(&turn.working, Some(timeline.history()), entity)
            .unwrap()
            .into_iter()
            .find(|movement| movement.to == to)
            .unwrap();
        rules
            .execute_move(&mut turn, Some(timeline.history()), movement, None)
            .unwrap();
        timeline.commit_turn(turn).unwrap();
    }

    #[test]
    fn status_detects_checkmate_and_stalemate() {
        use glorichess_core::History;

        let rules = ChessRules::standard();
        let history = History::default();

        let mut mate = empty_chess_state().unwrap();
        add_piece(&mut mate, 1, KING, ChessSide::White, Position::new(5, 5));
        add_piece(&mut mate, 2, QUEEN, ChessSide::White, Position::new(6, 6));
        add_piece(&mut mate, 3, KING, ChessSide::Black, Position::new(7, 7));
        mate.set_active_players(vec![BLACK_PLAYER]).unwrap();
        let status = rules.status(&mate, &history).unwrap();
        assert!(status.in_check);
        assert_eq!(
            status.outcome,
            Some(ChessOutcome::Checkmate {
                winner: WHITE_PLAYER,
                loser: BLACK_PLAYER,
            })
        );

        let mut stalemate = empty_chess_state().unwrap();
        add_piece(&mut stalemate, 1, KING, ChessSide::White, Position::new(5, 6));
        add_piece(&mut stalemate, 2, QUEEN, ChessSide::White, Position::new(6, 5));
        add_piece(&mut stalemate, 3, KING, ChessSide::Black, Position::new(7, 7));
        stalemate.set_active_players(vec![BLACK_PLAYER]).unwrap();
        let status = rules.status(&stalemate, &history).unwrap();
        assert!(!status.in_check);
        assert_eq!(status.outcome, Some(ChessOutcome::Stalemate));
    }

    #[test]
    fn resignation_and_draw_agreement_are_persistent_terminal_outcomes() {
        use glorichess_core::History;

        let rules = ChessRules::standard();
        let history = History::default();
        let mut resignation = standard_chess_state().unwrap();
        assert_eq!(
            rules.resign(&mut resignation, WHITE_PLAYER).unwrap(),
            ChessOutcome::Resignation {
                winner: BLACK_PLAYER,
                loser: WHITE_PLAYER,
            }
        );
        assert_eq!(
            rules.status(&resignation, &history).unwrap().outcome,
            Some(ChessOutcome::Resignation {
                winner: BLACK_PLAYER,
                loser: WHITE_PLAYER,
            })
        );

        let mut agreement = standard_chess_state().unwrap();
        assert_eq!(rules.agree_draw(&mut agreement), ChessOutcome::DrawAgreement);
        assert_eq!(
            rules.status(&agreement, &history).unwrap().outcome,
            Some(ChessOutcome::DrawAgreement)
        );
    }

    #[test]
    fn repetition_key_tracks_side_castling_and_effective_en_passant() {
        use glorichess_core::{GameTimeline, History};

        let rules = ChessRules::standard();
        let mut with_rights = empty_chess_state().unwrap();
        add_piece(&mut with_rights, 1, KING, ChessSide::White, Position::new(4, 0));
        let rook = add_piece(&mut with_rights, 2, ROOK, ChessSide::White, Position::new(7, 0));
        add_piece(&mut with_rights, 3, KING, ChessSide::Black, Position::new(4, 7));
        let no_history = History::default();
        let key_with_rights = rules.position_key(&with_rights, &no_history).unwrap();
        let mut without_rights = with_rights.clone();
        without_rights.entity_mut(rook).unwrap().move_count = 1;
        let key_without_rights = rules.position_key(&without_rights, &no_history).unwrap();
        assert_ne!(key_with_rights, key_without_rights);

        let mut black_to_move = with_rights.clone();
        black_to_move.set_active_players(vec![BLACK_PLAYER]).unwrap();
        let black_key = rules.position_key(&black_to_move, &no_history).unwrap();
        assert_ne!(key_with_rights, black_key);

        let mut ep_state = empty_chess_state().unwrap();
        add_piece(&mut ep_state, 10, KING, ChessSide::White, Position::new(4, 0));
        add_piece(&mut ep_state, 11, PAWN, ChessSide::White, Position::new(4, 4));
        add_piece(&mut ep_state, 12, KING, ChessSide::Black, Position::new(4, 7));
        let black_pawn = add_piece(&mut ep_state, 13, PAWN, ChessSide::Black, Position::new(3, 6));
        ep_state.set_active_players(vec![BLACK_PLAYER]).unwrap();
        let mut timeline = GameTimeline::new(ep_state).unwrap();
        commit_to(&mut timeline, &rules, black_pawn, Position::new(3, 4));
        let with_ep = rules.position_key(timeline.current(), timeline.history()).unwrap();
        let without_ep = rules.position_key(timeline.current(), &History::default()).unwrap();
        assert_ne!(with_ep, without_ep);
    }

    #[test]
    fn threefold_is_claimable_and_fivefold_is_automatic() {
        use glorichess_core::GameTimeline;

        let rules = ChessRules::standard();
        let mut state = empty_chess_state().unwrap();
        add_piece(&mut state, 1, KING, ChessSide::White, Position::new(4, 0));
        let white_knight = add_piece(&mut state, 2, KNIGHT, ChessSide::White, Position::new(1, 0));
        add_piece(&mut state, 3, KING, ChessSide::Black, Position::new(4, 7));
        let black_knight = add_piece(&mut state, 4, KNIGHT, ChessSide::Black, Position::new(1, 7));
        let mut timeline = GameTimeline::new(state).unwrap();

        for _ in 0..2 {
            commit_to(&mut timeline, &rules, white_knight, Position::new(2, 2));
            commit_to(&mut timeline, &rules, black_knight, Position::new(2, 5));
            commit_to(&mut timeline, &rules, white_knight, Position::new(1, 0));
            commit_to(&mut timeline, &rules, black_knight, Position::new(1, 7));
        }
        let status = rules.status(timeline.current(), timeline.history()).unwrap();
        assert_eq!(status.repetition_count, 3);
        assert!(status.can_claim_threefold_repetition);
        assert_eq!(status.outcome, None);

        let mut claimed = timeline.current().clone();
        assert_eq!(
            rules
                .claim_draw(
                    &mut claimed,
                    timeline.history(),
                    ChessDrawClaim::ThreefoldRepetition,
                )
                .unwrap(),
            ChessOutcome::ThreefoldRepetition
        );

        for _ in 0..2 {
            commit_to(&mut timeline, &rules, white_knight, Position::new(2, 2));
            commit_to(&mut timeline, &rules, black_knight, Position::new(2, 5));
            commit_to(&mut timeline, &rules, white_knight, Position::new(1, 0));
            commit_to(&mut timeline, &rules, black_knight, Position::new(1, 7));
        }
        let status = rules.status(timeline.current(), timeline.history()).unwrap();
        assert_eq!(status.repetition_count, 5);
        assert_eq!(status.outcome, Some(ChessOutcome::FivefoldRepetition));
    }

    #[test]
    fn halfmove_clock_resets_and_fifty_seventy_five_rules_are_exposed() {
        use glorichess_core::{History, TurnSession};

        let rules = ChessRules::standard();
        let history = History::default();
        let mut state = empty_chess_state().unwrap();
        add_piece(&mut state, 1, KING, ChessSide::White, Position::new(4, 0));
        let white_knight = add_piece(&mut state, 2, KNIGHT, ChessSide::White, Position::new(1, 0));
        let white_pawn = add_piece(&mut state, 3, PAWN, ChessSide::White, Position::new(0, 1));
        add_piece(&mut state, 4, KING, ChessSide::Black, Position::new(4, 7));
        add_piece(&mut state, 5, PAWN, ChessSide::Black, Position::new(0, 6));

        let mut turn = TurnSession::new(&state, WHITE_PLAYER).unwrap();
        let movement = rules
            .legal_moves(&turn.working, white_knight)
            .unwrap()
            .into_iter()
            .find(|movement| movement.to == Position::new(2, 2))
            .unwrap();
        rules.execute_move(&mut turn, None, movement, None).unwrap();
        assert_eq!(rules.halfmove_clock(&turn.working), 1);

        let mut pawn_state = state.clone();
        rules.set_halfmove_clock(&mut pawn_state, 99);
        let mut turn = TurnSession::new(&pawn_state, WHITE_PLAYER).unwrap();
        let movement = rules
            .legal_moves(&turn.working, white_pawn)
            .unwrap()
            .into_iter()
            .find(|movement| movement.to == Position::new(0, 2))
            .unwrap();
        rules.execute_move(&mut turn, None, movement, None).unwrap();
        assert_eq!(rules.halfmove_clock(&turn.working), 0);

        let mut threshold = state.clone();
        rules.set_halfmove_clock(&mut threshold, 100);
        let status = rules.status(&threshold, &history).unwrap();
        assert!(status.can_claim_fifty_move_rule);
        assert_eq!(status.outcome, None);
        let mut claimed = threshold.clone();
        assert_eq!(
            rules
                .claim_draw(&mut claimed, &history, ChessDrawClaim::FiftyMoveRule)
                .unwrap(),
            ChessOutcome::FiftyMoveRule
        );
        rules.set_halfmove_clock(&mut threshold, 150);
        assert_eq!(
            rules.status(&threshold, &history).unwrap().outcome,
            Some(ChessOutcome::SeventyFiveMoveRule)
        );
    }

    #[test]
    fn capture_resets_halfmove_clock() {
        use glorichess_core::TurnSession;

        let rules = ChessRules::standard();
        let mut state = empty_chess_state().unwrap();
        add_piece(&mut state, 1, KING, ChessSide::White, Position::new(4, 0));
        let rook = add_piece(&mut state, 2, ROOK, ChessSide::White, Position::new(0, 0));
        add_piece(&mut state, 3, KING, ChessSide::Black, Position::new(4, 7));
        add_piece(&mut state, 4, KNIGHT, ChessSide::Black, Position::new(0, 7));
        rules.set_halfmove_clock(&mut state, 99);
        let mut turn = TurnSession::new(&state, WHITE_PLAYER).unwrap();
        let capture = rules
            .legal_moves(&turn.working, rook)
            .unwrap()
            .into_iter()
            .find(|movement| movement.to == Position::new(0, 7))
            .unwrap();
        rules.execute_move(&mut turn, None, capture, None).unwrap();
        assert_eq!(rules.halfmove_clock(&turn.working), 0);
    }

    #[test]
    fn dead_positions_are_automatic_and_checkmate_precedes_75_move_rule() {
        use glorichess_core::History;

        let rules = ChessRules::standard();
        let history = History::default();
        let mut dead = empty_chess_state().unwrap();
        add_piece(&mut dead, 1, KING, ChessSide::White, Position::new(0, 0));
        add_piece(&mut dead, 2, KING, ChessSide::Black, Position::new(7, 7));
        assert_eq!(
            rules.status(&dead, &history).unwrap().outcome,
            Some(ChessOutcome::DeadPosition)
        );

        let mut mate = empty_chess_state().unwrap();
        add_piece(&mut mate, 1, KING, ChessSide::White, Position::new(5, 5));
        add_piece(&mut mate, 2, QUEEN, ChessSide::White, Position::new(6, 6));
        add_piece(&mut mate, 3, KING, ChessSide::Black, Position::new(7, 7));
        mate.set_active_players(vec![BLACK_PLAYER]).unwrap();
        rules.set_halfmove_clock(&mut mate, 150);
        assert!(matches!(
            rules.status(&mate, &history).unwrap().outcome,
            Some(ChessOutcome::Checkmate { .. })
        ));
    }

}
