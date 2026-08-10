//! Generic GloriChess game runtime primitives.
#![forbid(unsafe_code)]

mod board;
mod error;
mod ids;
mod state;
mod value;

pub use board::{Board, Position};
pub use error::CoreError;
pub use ids::{AbilityId, ChoiceId, EntityId, EntityTypeId, PlayerId, TeamId};
pub use state::{
    EntityData, EntityState, EntityStore, GameState, PlayerData, PlayerState, PlayerStore,
    RulesetState, TeamData, TeamState, TeamStore, TurnData, TurnState,
};
pub use value::{StateMap, StateValue};

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_state() -> GameState {
        let mut state = GameState::new(4, 4).unwrap();
        let team = TeamId::new(10);
        let player_a = PlayerId::new(1);
        let player_b = PlayerId::new(2);
        state.add_team(TeamState::new(team)).unwrap();
        state
            .add_player(PlayerState::new(player_a).with_team(team))
            .unwrap();
        state
            .add_player(PlayerState::new(player_b).with_team(team))
            .unwrap();
        state.set_active_players(vec![player_a]).unwrap();
        state
    }

    #[test]
    fn board_rejects_invalid_positions() {
        let board = Board::new(8, 8).unwrap();
        assert!(board.contains(Position::new(7, 7)));
        assert!(!board.contains(Position::new(8, 7)));
        assert_eq!(
            board.entity_at(Position::new(8, 0)),
            Err(CoreError::PositionOutOfBounds(Position::new(8, 0)))
        );
    }

    #[test]
    fn entity_lifecycle_keeps_board_and_store_in_sync() {
        let mut state = sample_state();
        let entity_id = EntityId::new(7);
        let owner = PlayerId::new(1);
        state
            .add_entity(EntityState::new(
                entity_id,
                EntityTypeId::new(3),
                owner,
                Position::new(1, 1),
            ))
            .unwrap();

        assert_eq!(state.entity_at(Position::new(1, 1)).unwrap().unwrap().id, entity_id);
        state.move_entity(entity_id, Position::new(2, 2)).unwrap();
        assert!(state.entity_at(Position::new(1, 1)).unwrap().is_none());
        assert_eq!(state.entity(entity_id).unwrap().move_count, 1);
        assert_eq!(state.entity(entity_id).unwrap().position, Position::new(2, 2));

        let removed = state.remove_entity(entity_id).unwrap();
        assert_eq!(removed.id, entity_id);
        assert!(state.entity_at(Position::new(2, 2)).unwrap().is_none());
        state.validate().unwrap();
    }

    #[test]
    fn owner_and_controller_are_independent() {
        let mut state = sample_state();
        let entity_id = EntityId::new(7);
        state
            .add_entity(EntityState::new(
                entity_id,
                EntityTypeId::new(3),
                PlayerId::new(1),
                Position::new(0, 0),
            ))
            .unwrap();

        state.set_controller(entity_id, PlayerId::new(2)).unwrap();
        let entity = state.entity(entity_id).unwrap();
        assert_eq!(entity.owner, PlayerId::new(1));
        assert_eq!(entity.controller, PlayerId::new(2));
        state.validate().unwrap();
    }

    #[test]
    fn entity_and_ruleset_state_are_extensible() {
        let mut state = sample_state();
        let mut entity = EntityState::new(
            EntityId::new(7),
            EntityTypeId::new(3),
            PlayerId::new(1),
            Position::new(0, 0),
        );
        entity.state.insert("health", 100_u32);
        state.add_entity(entity).unwrap();
        state.ruleset_state.insert("round", 4_u32);

        assert_eq!(
            state.entity(EntityId::new(7)).unwrap().state.get("health").and_then(StateValue::as_u64),
            Some(100)
        );
        assert_eq!(
            state.ruleset_state.get("round").and_then(StateValue::as_u64),
            Some(4)
        );
    }

    #[test]
    fn validate_detects_manual_entity_position_corruption() {
        let mut state = sample_state();
        let entity_id = EntityId::new(7);
        state
            .add_entity(EntityState::new(
                entity_id,
                EntityTypeId::new(3),
                PlayerId::new(1),
                Position::new(0, 0),
            ))
            .unwrap();

        state.entity_mut(entity_id).unwrap().position = Position::new(1, 0);
        assert!(matches!(
            state.validate(),
            Err(CoreError::EntityPlacementMismatch { entity, .. }) if entity == entity_id
        ));
    }

    #[test]
    fn duplicate_active_players_are_rejected() {
        let mut state = sample_state();
        let player = PlayerId::new(1);
        assert_eq!(
            state.set_active_players(vec![player, player]),
            Err(CoreError::DuplicateActivePlayer(player))
        );
    }
}
