//! Standard chess rules implemented on top of `glorichess-core`.
#![forbid(unsafe_code)]

mod error;
mod game;
mod interaction;
mod piece;
mod pieces;

pub use error::ChessError;
pub use game::{
    empty_chess_state, standard_chess_state, ChessRules, BLACK_PLAYER, BLACK_TEAM, WHITE_PLAYER,
    WHITE_TEAM,
};
pub use interaction::ChessInteractionRules;
pub use piece::{
    ChessPieceContext, ChessPieceKind, ChessPieceRule, PseudoMove, BISHOP, KING, KNIGHT, PAWN,
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

        assert!(!driver.interaction().choices.iter().any(|choice| {
            matches!(choice.kind, ChoiceKind::SelectPosition { position } if position == Position::new(3, 1))
        }));
        assert!(driver.interaction().choices.iter().any(|choice| {
            matches!(choice.kind, ChoiceKind::SelectPosition { position } if position == Position::new(4, 2))
        }));
    }

}
