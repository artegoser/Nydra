use nydra_core::{
    Choice, ChoiceKind, ChoiceSpec, EntityId, EntityPresentation, EntityRule, EntityRuleContext,
    EntityState, EntityTypeId, GameOutcome, GameState, History, InteractionError, InteractionFlow,
    InteractionRules, OutcomeRule, PlayerId, PlayerState, Position, PresentationCue, RecordedAction,
    RuleContext, RuleError, RuleRegistry, StateMap, StateValue, TurnSession,
};
use std::collections::{BTreeSet, VecDeque};

pub const STONE: EntityTypeId = EntityTypeId::new(1);
pub const BLACK: PlayerId = PlayerId::new(1);
pub const WHITE: PlayerId = PlayerId::new(2);
const CONSECUTIVE_PASSES: &str = "go.consecutive_passes";

pub struct StoneRule;

impl EntityRule for StoneRule {
    fn presentation(
        &self,
        context: EntityRuleContext<'_>,
    ) -> Result<EntityPresentation, RuleError> {
        let side = if context.entity().owner == BLACK {
            "black"
        } else {
            "white"
        };
        Ok(EntityPresentation::new(format!("go/{side}/stone")).with_label(format!("{side} stone")))
    }
}

pub fn registry() -> RuleRegistry {
    let mut registry = RuleRegistry::new();
    registry
        .register_entity(STONE, StoneRule)
        .expect("go registers one stone rule");
    registry.register_outcome(GoOutcomeRule);
    registry
}

pub fn empty_state(size: u16) -> GameState {
    let mut state = GameState::new(size, size).expect("valid Go board");
    state.add_player(PlayerState::new(BLACK)).unwrap();
    state.add_player(PlayerState::new(WHITE)).unwrap();
    state.set_active_players(vec![BLACK]).unwrap();
    state.ruleset_state.insert(CONSECUTIVE_PASSES, 0_u64);
    state
}

fn active_player(state: &GameState) -> Result<PlayerId, InteractionError> {
    let [player] = state.turn.active_players.as_slice() else {
        return Err(InteractionError::RuleViolation(
            "Go requires exactly one active player".into(),
        ));
    };
    Ok(*player)
}

fn opponent(player: PlayerId) -> Result<PlayerId, InteractionError> {
    match player {
        BLACK => Ok(WHITE),
        WHITE => Ok(BLACK),
        _ => Err(InteractionError::RuleViolation("unknown Go player".into())),
    }
}

fn neighbors(state: &GameState, position: Position) -> Vec<Position> {
    let mut result = Vec::with_capacity(4);
    if position.x > 0 {
        result.push(Position::new(position.x - 1, position.y));
    }
    if position.x + 1 < state.board.width() {
        result.push(Position::new(position.x + 1, position.y));
    }
    if position.y > 0 {
        result.push(Position::new(position.x, position.y - 1));
    }
    if position.y + 1 < state.board.height() {
        result.push(Position::new(position.x, position.y + 1));
    }
    result
}

fn group(state: &GameState, start: EntityId) -> Result<BTreeSet<EntityId>, InteractionError> {
    let owner = state.entity(start)?.owner;
    let mut seen = BTreeSet::new();
    let mut queue = VecDeque::from([start]);
    while let Some(entity) = queue.pop_front() {
        if !seen.insert(entity) {
            continue;
        }
        let position = state.entity(entity)?.position;
        for neighbor in neighbors(state, position) {
            let Some(other) = state.entity_at(neighbor)? else {
                continue;
            };
            if other.entity_type == STONE && other.owner == owner && !seen.contains(&other.id) {
                queue.push_back(other.id);
            }
        }
    }
    Ok(seen)
}

fn group_has_liberty(
    state: &GameState,
    group: &BTreeSet<EntityId>,
) -> Result<bool, InteractionError> {
    for entity in group {
        let position = state.entity(*entity)?.position;
        for neighbor in neighbors(state, position) {
            if state.entity_at(neighbor)?.is_none() {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn next_entity_id(state: &GameState) -> Result<EntityId, InteractionError> {
    let max = state.entities.keys().map(|id| id.get()).max().unwrap_or(0);
    let next = max.checked_add(1).ok_or_else(|| {
        InteractionError::RuleViolation("Go stone entity id space exhausted".into())
    })?;
    Ok(EntityId::new(next))
}

fn position_owner(state: &GameState, position: Position) -> Result<Option<PlayerId>, InteractionError> {
    Ok(state.entity_at(position)?.map(|entity| entity.owner))
}

fn same_board_position(a: &GameState, b: &GameState) -> Result<bool, InteractionError> {
    if a.board.width() != b.board.width() || a.board.height() != b.board.height() {
        return Ok(false);
    }
    for position in a.board.positions() {
        if position_owner(a, position)? != position_owner(b, position)? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn apply_placement(
    state: &mut GameState,
    player: PlayerId,
    position: Position,
) -> Result<EntityId, InteractionError> {
    if state.entity_at(position)?.is_some() {
        return Err(InteractionError::RuleViolation(
            "Go point is already occupied".into(),
        ));
    }
    let stone = next_entity_id(state)?;
    state.add_entity(EntityState::new(stone, STONE, player, position))?;

    let enemy = opponent(player)?;
    let adjacent_enemy = neighbors(state, position)
        .into_iter()
        .filter_map(|neighbor| state.entity_at(neighbor).ok().flatten())
        .filter(|entity| entity.owner == enemy)
        .map(|entity| entity.id)
        .collect::<BTreeSet<_>>();

    let mut captured = BTreeSet::new();
    for enemy_stone in adjacent_enemy {
        if !state.entities.contains_key(&enemy_stone) || captured.contains(&enemy_stone) {
            continue;
        }
        let enemy_group = group(state, enemy_stone)?;
        if !group_has_liberty(state, &enemy_group)? {
            captured.extend(enemy_group);
        }
    }
    for entity in captured {
        state.remove_entity(entity)?;
    }

    let own_group = group(state, stone)?;
    if !group_has_liberty(state, &own_group)? {
        return Err(InteractionError::RuleViolation(
            "Go suicide is not legal in this reference ruleset".into(),
        ));
    }
    Ok(stone)
}

fn placement_is_legal(
    state: &GameState,
    history: Option<&History>,
    player: PlayerId,
    position: Position,
) -> Result<bool, InteractionError> {
    if state.entity_at(position)?.is_some() {
        return Ok(false);
    }
    let mut candidate = state.clone();
    if apply_placement(&mut candidate, player, position).is_err() {
        return Ok(false);
    }
    if let Some(previous) = history.and_then(History::previous_turn) {
        if same_board_position(&candidate, &previous.before)? {
            return Ok(false);
        }
    }
    Ok(true)
}

#[derive(Clone, Default)]
pub struct GoInteractionRules {
    history: Option<History>,
}

impl GoInteractionRules {
    pub fn with_history(history: &History) -> Self {
        Self {
            history: Some(history.clone()),
        }
    }
}

impl InteractionRules for GoInteractionRules {
    fn choices(
        &self,
        turn: &TurnSession,
        _draft: &StateMap,
    ) -> Result<Vec<ChoiceSpec>, InteractionError> {
        let player = active_player(&turn.working)?;
        let mut choices = Vec::new();
        for position in turn.working.board.positions() {
            if placement_is_legal(&turn.working, self.history.as_ref(), player, position)? {
                choices.push(ChoiceSpec::position(position));
            }
        }
        choices.push(ChoiceSpec::option("pass").with_label("Pass"));
        Ok(choices)
    }

    fn apply_choice(
        &self,
        turn: &mut TurnSession,
        _draft: &mut StateMap,
        choice: &Choice,
    ) -> Result<InteractionFlow, InteractionError> {
        let player = active_player(&turn.working)?;
        match &choice.kind {
            ChoiceKind::SelectPosition { position } => {
                if !placement_is_legal(
                    &turn.working,
                    self.history.as_ref(),
                    player,
                    *position,
                )? {
                    return Err(InteractionError::RuleViolation(
                        "illegal Go placement".into(),
                    ));
                }
                let previous = self.history.as_ref().and_then(History::previous_turn).cloned();
                turn.apply_transaction(
                    RecordedAction::new("go.place"),
                    |transaction| -> Result<(), InteractionError> {
                        let stone = apply_placement(transaction.raw_state_mut(), player, *position)?;
                        if let Some(previous) = previous.as_ref() {
                            if same_board_position(transaction.state(), &previous.before)? {
                                return Err(InteractionError::RuleViolation(
                                    "simple ko forbids immediate recapture".into(),
                                ));
                            }
                        }
                        transaction
                            .ruleset_state_mut()
                            .insert(CONSECUTIVE_PASSES, 0_u64);
                        transaction
                            .raw_state_mut()
                            .set_active_players(vec![opponent(player)?])?;
                        let mut data = StateMap::new();
                        data.insert("stone", u64::from(stone.get()));
                        data.insert("x", u64::from(position.x));
                        data.insert("y", u64::from(position.y));
                        transaction.present(PresentationCue::new("go.place").with_data(data));
                        Ok(())
                    },
                )?;
                Ok(InteractionFlow::FinishTurn)
            }
            ChoiceKind::SelectOption { key } if key == "pass" => {
                let current_passes = turn
                    .working
                    .ruleset_state
                    .get(CONSECUTIVE_PASSES)
                    .and_then(StateValue::as_u64)
                    .unwrap_or(0);
                turn.apply_transaction(
                    RecordedAction::new("go.pass"),
                    |transaction| -> Result<(), InteractionError> {
                        transaction
                            .ruleset_state_mut()
                            .insert(CONSECUTIVE_PASSES, current_passes.saturating_add(1));
                        transaction
                            .raw_state_mut()
                            .set_active_players(vec![opponent(player)?])?;
                        Ok(())
                    },
                )?;
                Ok(InteractionFlow::FinishTurn)
            }
            _ => Err(InteractionError::RuleViolation(
                "unexpected Go choice".into(),
            )),
        }
    }
}

pub struct GoOutcomeRule;

impl OutcomeRule for GoOutcomeRule {
    fn evaluate(&self, context: RuleContext<'_>) -> Result<Option<GameOutcome>, RuleError> {
        let passes = context
            .state()
            .ruleset_state
            .get(CONSECUTIVE_PASSES)
            .and_then(StateValue::as_u64)
            .unwrap_or(0);
        if passes < 2 {
            return Ok(None);
        }

        // This is deliberately a compact architecture proof, not production Go
        // scoring. It uses living stone count after two passes to exercise a
        // terminal rule without embedding Go semantics in core.
        let black = context
            .state()
            .entities
            .values()
            .filter(|entity| entity.entity_type == STONE && entity.owner == BLACK)
            .count() as u64;
        let white = context
            .state()
            .entities
            .values()
            .filter(|entity| entity.entity_type == STONE && entity.owner == WHITE)
            .count() as u64;
        let mut data = StateMap::new();
        data.insert("black_stones", black);
        data.insert("white_stones", white);
        let mut outcome = GameOutcome::new("go.two_passes").with_data(data);
        if black > white {
            outcome = outcome.with_winner(BLACK).with_loser(WHITE);
        } else if white > black {
            outcome = outcome.with_winner(WHITE).with_loser(BLACK);
        }
        Ok(Some(outcome))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nydra_core::{GameTimeline, InteractionDriver};

    fn add_stone(
        state: &mut GameState,
        id: u32,
        owner: PlayerId,
        x: u16,
        y: u16,
    ) {
        state
            .add_entity(EntityState::new(
                EntityId::new(id),
                STONE,
                owner,
                Position::new(x, y),
            ))
            .unwrap();
    }

    #[test]
    fn placement_can_capture_a_multi_stone_group_without_move_semantics() {
        let mut state = empty_state(5);
        add_stone(&mut state, 1, WHITE, 2, 2);
        add_stone(&mut state, 2, WHITE, 2, 3);
        add_stone(&mut state, 3, BLACK, 1, 2);
        add_stone(&mut state, 4, BLACK, 3, 2);
        add_stone(&mut state, 5, BLACK, 2, 1);
        add_stone(&mut state, 6, BLACK, 1, 3);
        add_stone(&mut state, 7, BLACK, 3, 3);
        state.set_active_players(vec![BLACK]).unwrap();

        let turn = TurnSession::new(&state, BLACK).unwrap();
        let mut driver = InteractionDriver::new(GoInteractionRules::default(), turn).unwrap();
        let place = driver
            .interaction()
            .choices
            .iter()
            .find(|choice| {
                matches!(choice.kind, ChoiceKind::SelectPosition { position } if position == Position::new(2, 4))
            })
            .unwrap()
            .clone();
        driver.choose(place.id).unwrap();

        assert!(driver.turn().working.entity(EntityId::new(1)).is_err());
        assert!(driver.turn().working.entity(EntityId::new(2)).is_err());
        assert_eq!(
            driver
                .turn()
                .working
                .entity_at(Position::new(2, 4))
                .unwrap()
                .unwrap()
                .owner,
            BLACK
        );
        assert_eq!(driver.turn().working.turn.active_players, vec![WHITE]);
    }

    #[test]
    fn simple_ko_is_derived_from_previous_committed_position() {
        let mut state = empty_state(5);
        add_stone(&mut state, 1, BLACK, 1, 2);
        add_stone(&mut state, 2, BLACK, 3, 2);
        add_stone(&mut state, 3, BLACK, 2, 1);
        add_stone(&mut state, 4, WHITE, 2, 2);
        add_stone(&mut state, 5, WHITE, 1, 3);
        add_stone(&mut state, 6, WHITE, 3, 3);
        add_stone(&mut state, 7, WHITE, 2, 4);
        state.set_active_players(vec![BLACK]).unwrap();

        let mut timeline = GameTimeline::new(state).unwrap();
        let turn = timeline.begin_turn(BLACK).unwrap();
        let mut black = InteractionDriver::new(GoInteractionRules::default(), turn).unwrap();
        let capture = black
            .interaction()
            .choices
            .iter()
            .find(|choice| {
                matches!(choice.kind, ChoiceKind::SelectPosition { position } if position == Position::new(2, 3))
            })
            .unwrap()
            .clone();
        black.choose(capture.id).unwrap();
        timeline.commit_turn(black.into_turn().unwrap()).unwrap();

        let turn = timeline.begin_turn(WHITE).unwrap();
        let white = InteractionDriver::new(
            GoInteractionRules::with_history(timeline.history()),
            turn,
        )
        .unwrap();
        assert!(!white.interaction().choices.iter().any(|choice| {
            matches!(choice.kind, ChoiceKind::SelectPosition { position } if position == Position::new(2, 2))
        }));
    }

    #[test]
    fn two_passes_are_history_independent_terminal_game_state() {
        let mut timeline = GameTimeline::new(empty_state(5)).unwrap();
        for player in [BLACK, WHITE] {
            let turn = timeline.begin_turn(player).unwrap();
            let mut driver = InteractionDriver::new(
                GoInteractionRules::with_history(timeline.history()),
                turn,
            )
            .unwrap();
            let pass = driver
                .interaction()
                .choices
                .iter()
                .find(|choice| matches!(&choice.kind, ChoiceKind::SelectOption { key } if key == "pass"))
                .unwrap()
                .clone();
            driver.choose(pass.id).unwrap();
            timeline.commit_turn(driver.into_turn().unwrap()).unwrap();
        }

        let outcome = registry()
            .outcome(RuleContext::from_state(
                timeline.current(),
                Some(timeline.history()),
            ))
            .unwrap()
            .unwrap();
        assert_eq!(outcome.key, "go.two_passes");
    }

    #[test]
    fn stone_presentation_is_registered_as_an_entity_rule() {
        let mut state = empty_state(5);
        add_stone(&mut state, 1, BLACK, 0, 0);
        let presentation = registry()
            .presentation(
                RuleContext::from_state(&state, None),
                EntityId::new(1),
            )
            .unwrap();
        assert_eq!(presentation.asset_key, "go/black/stone");
    }
}
