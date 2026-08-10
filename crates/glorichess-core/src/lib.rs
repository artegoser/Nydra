//! Generic GloriChess game runtime primitives.
#![forbid(unsafe_code)]

mod board;
mod error;
mod history;
mod ids;
mod state;
mod value;

pub use board::{Board, Position};
pub use error::CoreError;
pub use history::{GameTimeline, History, RecordedAction, StepRecord, TurnRecord, TurnSession};
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

    #[test]
    fn turn_session_records_multiple_steps_from_updated_state() {
        let mut state = sample_state();
        let entity = EntityId::new(7);
        state
            .add_entity(EntityState::new(
                entity,
                EntityTypeId::new(3),
                PlayerId::new(1),
                Position::new(0, 0),
            ))
            .unwrap();

        let mut turn = TurnSession::new(&state, PlayerId::new(1)).unwrap();
        turn.apply_step(RecordedAction::new("move-1"), |state| {
            state.move_entity(entity, Position::new(1, 0))
        })
        .unwrap();
        turn.apply_step(RecordedAction::new("move-2"), |state| {
            assert_eq!(state.entity(entity)?.position, Position::new(1, 0));
            state.move_entity(entity, Position::new(2, 0))
        })
        .unwrap();

        assert_eq!(turn.steps.len(), 2);
        assert_eq!(turn.working.entity(entity).unwrap().position, Position::new(2, 0));
        assert_eq!(turn.before.entity(entity).unwrap().position, Position::new(0, 0));
    }

    #[test]
    fn speculative_state_never_enters_history() {
        let state = sample_state();
        let turn = TurnSession::new(&state, PlayerId::new(1)).unwrap();
        let (candidate, ()) = turn
            .speculate(|state| {
                state.ruleset_state.insert("speculative", true);
                Ok(())
            })
            .unwrap();

        assert_eq!(candidate.ruleset_state.get("speculative").and_then(StateValue::as_bool), Some(true));
        assert!(!turn.working.ruleset_state.contains_key("speculative"));
        assert!(turn.steps.is_empty());
    }

    #[test]
    fn timeline_commits_undoes_and_redoes_complete_turns() {
        let mut state = sample_state();
        let entity = EntityId::new(7);
        state
            .add_entity(EntityState::new(
                entity,
                EntityTypeId::new(3),
                PlayerId::new(1),
                Position::new(0, 0),
            ))
            .unwrap();
        let mut timeline = GameTimeline::new(state).unwrap();

        let mut turn = timeline.begin_turn(PlayerId::new(1)).unwrap();
        turn.apply_step(RecordedAction::new("move"), |state| {
            state.move_entity(entity, Position::new(1, 0))
        })
        .unwrap();
        timeline.commit_turn(turn).unwrap();

        assert_eq!(timeline.current().entity(entity).unwrap().position, Position::new(1, 0));
        assert_eq!(timeline.history().len(), 1);
        assert_eq!(timeline.history().last_step().unwrap().action.kind, "move");
        assert_eq!(timeline.entity_turns_ago(entity, 1).unwrap().position, Position::new(0, 0));

        timeline.undo().unwrap();
        assert_eq!(timeline.current().entity(entity).unwrap().position, Position::new(0, 0));
        assert!(timeline.can_redo());

        timeline.redo().unwrap();
        assert_eq!(timeline.current().entity(entity).unwrap().position, Position::new(1, 0));
        assert_eq!(timeline.history().len(), 1);
    }

    #[test]
    fn rolling_back_a_turn_returns_the_original_snapshot() {
        let state = sample_state();
        let mut turn = TurnSession::new(&state, PlayerId::new(1)).unwrap();
        turn.apply_step(RecordedAction::new("state-change"), |state| {
            state.ruleset_state.insert("changed", true);
            Ok(())
        })
        .unwrap();

        assert_eq!(turn.working.ruleset_state.get("changed").and_then(StateValue::as_bool), Some(true));
        let rolled_back = turn.rollback();
        assert_eq!(rolled_back, state);
    }
}
