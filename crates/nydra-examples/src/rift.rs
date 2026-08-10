use nydra_core::{
    AbilityId, AbilityRule, Choice, ChoiceKind, ChoiceSpec, EntityId, EntityPresentation,
    EntityRule, EntityRuleContext, EntityState, EntityTypeId, GameOutcome, GameState, History,
    InteractionError, InteractionFlow, InteractionRules, OutcomeRule, PlayerId, PlayerState,
    Position, PresentationCue, RecordedAction, RuleContext, RuleError, RuleRegistry, StateMap,
    StateValue, TeamId, TeamState, TurnSession,
};
use std::collections::BTreeSet;

pub const MAGE: EntityTypeId = EntityTypeId::new(1);
pub const FIREBALL: AbilityId = AbilityId::new(1);
pub const REWIND: AbilityId = AbilityId::new(2);
pub const HIJACK: AbilityId = AbilityId::new(3);

pub const PLAYER_ONE: PlayerId = PlayerId::new(1);
pub const PLAYER_TWO: PlayerId = PlayerId::new(2);
pub const PLAYER_THREE: PlayerId = PlayerId::new(3);
pub const TEAM_ALPHA: TeamId = TeamId::new(10);
pub const TEAM_BETA: TeamId = TeamId::new(20);

pub const MAGE_ONE: EntityId = EntityId::new(1);
pub const MAGE_TWO: EntityId = EntityId::new(2);
pub const MAGE_THREE: EntityId = EntityId::new(3);

const HP: &str = "rift.hp";
const MANA: &str = "rift.mana";
const SELECTED: &str = "rift.selected";
const MOVED: &str = "rift.moved";
const ABILITY: &str = "rift.ability";
const TARGET: &str = "rift.target";
const MODE: &str = "rift.mode";

pub struct MageRule;

impl MageRule {
    fn move_choices(
        state: &GameState,
        entity: EntityId,
    ) -> Result<Vec<ChoiceSpec>, InteractionError> {
        let position = state.entity(entity)?.position;
        let candidates = [
            (i32::from(position.x) - 1, i32::from(position.y)),
            (i32::from(position.x) + 1, i32::from(position.y)),
            (i32::from(position.x), i32::from(position.y) - 1),
            (i32::from(position.x), i32::from(position.y) + 1),
        ];
        let mut choices = Vec::new();
        for (x, y) in candidates {
            if x < 0 || y < 0 {
                continue;
            }
            let Ok(x) = u16::try_from(x) else {
                continue;
            };
            let Ok(y) = u16::try_from(y) else {
                continue;
            };
            let position = Position::new(x, y);
            if state.board.contains(position) && state.entity_at(position)?.is_none() {
                choices.push(ChoiceSpec::position(position));
            }
        }
        Ok(choices)
    }
}

impl EntityRule for MageRule {
    fn presentation(
        &self,
        context: EntityRuleContext<'_>,
    ) -> Result<EntityPresentation, RuleError> {
        let entity = context.entity();
        let mut data = StateMap::new();
        data.insert("hp", entity_u64(entity, HP).unwrap_or(0));
        data.insert("mana", entity_u64(entity, MANA).unwrap_or(0));
        Ok(EntityPresentation::new("rift/mage")
            .with_label(format!("Mage {}", entity.owner.get()))
            .with_data(data))
    }
}

pub struct FireballRule;
pub struct RewindRule;
pub struct HijackRule;

impl AbilityRule for FireballRule {
    fn choices(
        &self,
        context: RuleContext<'_>,
        actor: EntityId,
        draft: &StateMap,
    ) -> Result<Vec<ChoiceSpec>, RuleError> {
        if draft_entity(draft, TARGET).is_none() {
            let source = context.entity(actor)?;
            return Ok(context
                .state()
                .entities
                .values()
                .filter(|target| target.id != actor)
                .filter(|target| !same_team(context.state(), source.controller, target.controller))
                .map(|target| ChoiceSpec::entity(target.id))
                .collect());
        }
        if draft.get(MODE).and_then(StateValue::as_str).is_none() {
            return Ok(vec![
                ChoiceSpec::option("normal").with_label("Fireball"),
                ChoiceSpec::option("overcharge").with_label("Overcharge"),
            ]);
        }
        Ok(Vec::new())
    }

    fn execute(
        &self,
        context: RuleContext<'_>,
        actor: EntityId,
        transaction: &mut nydra_core::Transaction,
        input: &StateMap,
    ) -> Result<(), RuleError> {
        let target = draft_entity(input, TARGET)
            .ok_or_else(|| RuleError::Rejected("fireball requires a target".into()))?;
        let mode = input
            .get(MODE)
            .and_then(StateValue::as_str)
            .ok_or_else(|| RuleError::Rejected("fireball requires a mode".into()))?;
        let (damage, mana_cost) = match mode {
            "normal" => (40_u64, 1_u64),
            "overcharge" => (80_u64, 2_u64),
            _ => return Err(RuleError::Rejected("unknown fireball mode".into())),
        };
        let source = context.entity(actor)?;
        let victim = context.entity(target)?;
        if same_team(context.state(), source.controller, victim.controller) {
            return Err(RuleError::Rejected(
                "fireball cannot target a teammate".into(),
            ));
        }

        let mana = entity_u64(transaction.entity(actor)?, MANA).unwrap_or(0);
        if mana < mana_cost {
            return Err(RuleError::Rejected("not enough mana".into()));
        }
        transaction
            .entity_mut(actor)?
            .state
            .insert(MANA, mana - mana_cost);

        let hp = entity_u64(transaction.entity(target)?, HP).unwrap_or(0);
        if hp <= damage {
            transaction.remove_entity(target)?;
        } else {
            transaction
                .entity_mut(target)?
                .state
                .insert(HP, hp - damage);
        }

        let mut data = StateMap::new();
        data.insert("actor", u64::from(actor.get()));
        data.insert("target", u64::from(target.get()));
        data.insert("damage", damage);
        data.insert("mode", mode);
        transaction.present(PresentationCue::new("rift.fireball").with_data(data));
        Ok(())
    }
}

impl AbilityRule for RewindRule {
    fn choices(
        &self,
        _context: RuleContext<'_>,
        actor: EntityId,
        draft: &StateMap,
    ) -> Result<Vec<ChoiceSpec>, RuleError> {
        if draft_entity(draft, TARGET).is_none() {
            return Ok(vec![ChoiceSpec::entity(actor).with_label("Rewind self")]);
        }
        Ok(Vec::new())
    }

    fn execute(
        &self,
        context: RuleContext<'_>,
        _actor: EntityId,
        transaction: &mut nydra_core::Transaction,
        input: &StateMap,
    ) -> Result<(), RuleError> {
        let target = draft_entity(input, TARGET)
            .ok_or_else(|| RuleError::Rejected("rewind requires a target".into()))?;
        let previous = context
            .history()
            .and_then(History::previous_turn)
            .and_then(|turn| turn.before.entities.get(&target))
            .ok_or_else(|| RuleError::Rejected("rewind has no previous state".into()))?;
        let previous_hp = entity_u64(previous, HP)
            .ok_or_else(|| RuleError::Rejected("previous target has no hp".into()))?;
        transaction
            .entity_mut(target)?
            .state
            .insert(HP, previous_hp);

        let mut data = StateMap::new();
        data.insert("target", u64::from(target.get()));
        data.insert("restored_hp", previous_hp);
        transaction.present(PresentationCue::new("rift.rewind").with_data(data));
        Ok(())
    }
}

impl AbilityRule for HijackRule {
    fn choices(
        &self,
        context: RuleContext<'_>,
        actor: EntityId,
        draft: &StateMap,
    ) -> Result<Vec<ChoiceSpec>, RuleError> {
        if draft_entity(draft, TARGET).is_some() {
            return Ok(Vec::new());
        }
        let controller = context.entity(actor)?.controller;
        Ok(context
            .state()
            .entities
            .values()
            .filter(|target| target.id != actor && target.controller != controller)
            .map(|target| ChoiceSpec::entity(target.id))
            .collect())
    }

    fn execute(
        &self,
        context: RuleContext<'_>,
        actor: EntityId,
        transaction: &mut nydra_core::Transaction,
        input: &StateMap,
    ) -> Result<(), RuleError> {
        let target = draft_entity(input, TARGET)
            .ok_or_else(|| RuleError::Rejected("hijack requires a target".into()))?;
        let controller = context.entity(actor)?.controller;
        transaction
            .raw_state_mut()
            .set_controller(target, controller)?;
        let mut data = StateMap::new();
        data.insert("target", u64::from(target.get()));
        data.insert("controller", u64::from(controller.get()));
        transaction.present(PresentationCue::new("rift.hijack").with_data(data));
        Ok(())
    }
}

pub struct RiftOutcomeRule;

impl OutcomeRule for RiftOutcomeRule {
    fn evaluate(&self, context: RuleContext<'_>) -> Result<Option<GameOutcome>, RuleError> {
        let mut living_teams = BTreeSet::new();
        for entity in context.state().entities.values() {
            if let Some(team) = context
                .state()
                .players
                .get(&entity.owner)
                .and_then(|player| player.team)
            {
                living_teams.insert(team);
            }
        }
        if living_teams.len() != 1 || context.state().teams.len() < 2 {
            return Ok(None);
        }
        let winner = *living_teams.iter().next().expect("one living team");
        let mut outcome = GameOutcome::new("rift.last_team_standing").with_winning_team(winner);
        for player in context.state().players.values() {
            if player.team == Some(winner) {
                outcome.winners.push(player.id);
            } else {
                outcome.losers.push(player.id);
                if let Some(team) = player.team {
                    if !outcome.losing_teams.contains(&team) {
                        outcome.losing_teams.push(team);
                    }
                }
            }
        }
        Ok(Some(outcome))
    }
}

pub fn registry() -> RuleRegistry {
    let mut registry = RuleRegistry::new();
    registry
        .register_entity(MAGE, MageRule)
        .expect("rift registers one mage rule");
    registry.register_ability(FIREBALL, FireballRule).unwrap();
    registry.register_ability(REWIND, RewindRule).unwrap();
    registry.register_ability(HIJACK, HijackRule).unwrap();
    registry.register_outcome(RiftOutcomeRule);
    registry
}

pub fn standard_state() -> GameState {
    let mut state = GameState::new(5, 5).expect("valid Rift board");
    state.add_team(TeamState::new(TEAM_ALPHA)).unwrap();
    state.add_team(TeamState::new(TEAM_BETA)).unwrap();
    state
        .add_player(PlayerState::new(PLAYER_ONE).with_team(TEAM_ALPHA))
        .unwrap();
    state
        .add_player(PlayerState::new(PLAYER_TWO).with_team(TEAM_ALPHA))
        .unwrap();
    state
        .add_player(PlayerState::new(PLAYER_THREE).with_team(TEAM_BETA))
        .unwrap();
    state.set_active_players(vec![PLAYER_ONE]).unwrap();
    state
        .add_entity(mage(MAGE_ONE, PLAYER_ONE, Position::new(0, 0), 100, 3))
        .unwrap();
    state
        .add_entity(mage(MAGE_TWO, PLAYER_TWO, Position::new(4, 0), 100, 3))
        .unwrap();
    state
        .add_entity(mage(MAGE_THREE, PLAYER_THREE, Position::new(4, 4), 60, 3))
        .unwrap();
    state
}

fn mage(
    id: EntityId,
    owner: PlayerId,
    position: Position,
    hp: u64,
    mana: u64,
) -> EntityState {
    let mut entity = EntityState::new(id, MAGE, owner, position);
    entity.state.insert(HP, hp);
    entity.state.insert(MANA, mana);
    entity
}

fn entity_u64(entity: &EntityState, key: &str) -> Option<u64> {
    entity.state.get(key).and_then(StateValue::as_u64)
}

fn same_team(state: &GameState, a: PlayerId, b: PlayerId) -> bool {
    let a_team = state.players.get(&a).and_then(|player| player.team);
    let b_team = state.players.get(&b).and_then(|player| player.team);
    a_team.is_some() && a_team == b_team
}

fn active_player(state: &GameState) -> Result<PlayerId, InteractionError> {
    let [player] = state.turn.active_players.as_slice() else {
        return Err(InteractionError::RuleViolation(
            "Rift requires exactly one active player".into(),
        ));
    };
    Ok(*player)
}

fn next_player(state: &GameState, current: PlayerId) -> Result<PlayerId, InteractionError> {
    let players = state.players.keys().copied().collect::<Vec<_>>();
    let index = players
        .iter()
        .position(|player| *player == current)
        .ok_or_else(|| InteractionError::RuleViolation("active Rift player is unknown".into()))?;
    Ok(players[(index + 1) % players.len()])
}

fn draft_entity(draft: &StateMap, key: &str) -> Option<EntityId> {
    draft
        .get(key)
        .and_then(StateValue::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .map(EntityId::new)
}

fn set_draft_entity(draft: &mut StateMap, key: &str, entity: EntityId) {
    draft.insert(key, u64::from(entity.get()));
}

fn draft_ability(draft: &StateMap) -> Option<AbilityId> {
    draft
        .get(ABILITY)
        .and_then(StateValue::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .map(AbilityId::new)
}

fn rule_error(error: RuleError) -> InteractionError {
    InteractionError::RuleViolation(error.to_string())
}

fn ability_input(draft: &StateMap) -> StateMap {
    let mut input = StateMap::new();
    if let Some(target) = draft_entity(draft, TARGET) {
        input.insert(TARGET, u64::from(target.get()));
    }
    if let Some(mode) = draft.get(MODE).and_then(StateValue::as_str) {
        input.insert(MODE, mode);
    }
    input
}

pub struct RiftInteractionRules {
    registry: RuleRegistry,
    history: Option<History>,
}

impl Default for RiftInteractionRules {
    fn default() -> Self {
        Self {
            registry: registry(),
            history: None,
        }
    }
}

impl RiftInteractionRules {
    pub fn with_history(history: &History) -> Self {
        Self {
            registry: registry(),
            history: Some(history.clone()),
        }
    }

    fn execute_ability(
        &self,
        turn: &mut TurnSession,
        draft: &mut StateMap,
        ability: AbilityId,
    ) -> Result<InteractionFlow, InteractionError> {
        let actor = draft_entity(draft, SELECTED).ok_or_else(|| {
            InteractionError::RuleViolation("Rift ability has no selected actor".into())
        })?;
        let player = active_player(&turn.working)?;
        let next = next_player(&turn.working, player)?;
        let source = turn.working.clone();
        let context = RuleContext::from_state(&source, self.history.as_ref());
        let input = ability_input(draft);
        let rule = self.registry.ability_rule(ability).map_err(rule_error)?;
        let mut action_data = StateMap::new();
        action_data.insert("actor", u64::from(actor.get()));
        action_data.insert("ability", u64::from(ability.get()));
        let action = RecordedAction {
            kind: "rift.ability".into(),
            data: action_data,
        };
        turn.apply_transaction(action, |transaction| -> Result<(), InteractionError> {
            rule.execute(context, actor, transaction, &input)
                .map_err(rule_error)?;
            transaction
                .raw_state_mut()
                .set_active_players(vec![next])?;
            Ok(())
        })?;
        draft.remove(SELECTED);
        draft.remove(MOVED);
        draft.remove(ABILITY);
        draft.remove(TARGET);
        draft.remove(MODE);
        Ok(InteractionFlow::FinishTurn)
    }

    fn continuation_choices(
        &self,
        turn: &TurnSession,
        draft: &StateMap,
        ability: AbilityId,
    ) -> Result<Vec<ChoiceSpec>, InteractionError> {
        let actor = draft_entity(draft, SELECTED).ok_or_else(|| {
            InteractionError::RuleViolation("Rift ability has no selected actor".into())
        })?;
        self.registry
            .ability_rule(ability)
            .map_err(rule_error)?
            .choices(
                RuleContext::from_turn(turn, self.history.as_ref()),
                actor,
                draft,
            )
            .map_err(rule_error)
    }
}

impl InteractionRules for RiftInteractionRules {
    fn choices(
        &self,
        turn: &TurnSession,
        draft: &StateMap,
    ) -> Result<Vec<ChoiceSpec>, InteractionError> {
        let player = active_player(&turn.working)?;
        let Some(actor) = draft_entity(draft, SELECTED) else {
            return Ok(turn
                .working
                .entities
                .values()
                .filter(|entity| entity.controller == player && entity.entity_type == MAGE)
                .map(|entity| ChoiceSpec::entity(entity.id))
                .collect());
        };

        if !draft
            .get(MOVED)
            .and_then(StateValue::as_bool)
            .unwrap_or(false)
        {
            return Ok(MageRule::move_choices(&turn.working, actor)?);
        }

        if let Some(ability) = draft_ability(draft) {
            return self.continuation_choices(turn, draft, ability);
        }

        Ok(vec![
            ChoiceSpec::ability(FIREBALL).with_label("Fireball"),
            ChoiceSpec::ability(REWIND).with_label("Rewind"),
            ChoiceSpec::ability(HIJACK).with_label("Hijack"),
            ChoiceSpec::finish_turn(),
        ])
    }

    fn apply_choice(
        &self,
        turn: &mut TurnSession,
        draft: &mut StateMap,
        choice: &Choice,
    ) -> Result<InteractionFlow, InteractionError> {
        match &choice.kind {
            ChoiceKind::SelectEntity { entity } => {
                if draft_entity(draft, SELECTED).is_none() {
                    let player = active_player(&turn.working)?;
                    let actor = turn.working.entity(*entity)?;
                    if actor.controller != player || actor.entity_type != MAGE {
                        return Err(InteractionError::RuleViolation(
                            "selected Rift entity is not controlled by the active player".into(),
                        ));
                    }
                    set_draft_entity(draft, SELECTED, *entity);
                    return Ok(InteractionFlow::Continue);
                }

                let ability = draft_ability(draft).ok_or_else(|| {
                    InteractionError::RuleViolation("unexpected Rift entity choice".into())
                })?;
                set_draft_entity(draft, TARGET, *entity);
                if self.continuation_choices(turn, draft, ability)?.is_empty() {
                    self.execute_ability(turn, draft, ability)
                } else {
                    Ok(InteractionFlow::Continue)
                }
            }
            ChoiceKind::SelectPosition { position } => {
                let actor = draft_entity(draft, SELECTED).ok_or_else(|| {
                    InteractionError::RuleViolation("Rift move has no selected actor".into())
                })?;
                if draft
                    .get(MOVED)
                    .and_then(StateValue::as_bool)
                    .unwrap_or(false)
                    || !MageRule::move_choices(&turn.working, actor)?
                    .iter()
                    .any(|choice| matches!(choice.kind, ChoiceKind::SelectPosition { position: candidate } if candidate == *position))
                {
                    return Err(InteractionError::RuleViolation("illegal Rift move".into()));
                }
                let mut data = StateMap::new();
                data.insert("actor", u64::from(actor.get()));
                data.insert("x", u64::from(position.x));
                data.insert("y", u64::from(position.y));
                turn.apply_transaction(
                    RecordedAction {
                        kind: "rift.move".into(),
                        data,
                    },
                    |transaction| -> Result<(), InteractionError> {
                        transaction.move_entity(actor, *position)?;
                        Ok(())
                    },
                )?;
                draft.insert(MOVED, true);
                Ok(InteractionFlow::Continue)
            }
            ChoiceKind::SelectAbility { ability } => {
                self.registry.ability_rule(*ability).map_err(rule_error)?;
                draft.insert(ABILITY, u64::from(ability.get()));
                draft.remove(TARGET);
                draft.remove(MODE);
                Ok(InteractionFlow::Continue)
            }
            ChoiceKind::SelectOption { key } => {
                let ability = draft_ability(draft).ok_or_else(|| {
                    InteractionError::RuleViolation("Rift option has no active ability".into())
                })?;
                draft.insert(MODE, key.as_str());
                if self.continuation_choices(turn, draft, ability)?.is_empty() {
                    self.execute_ability(turn, draft, ability)
                } else {
                    Ok(InteractionFlow::Continue)
                }
            }
            ChoiceKind::FinishTurn => {
                let player = active_player(&turn.working)?;
                let next = next_player(&turn.working, player)?;
                turn.apply_transaction(
                    RecordedAction::new("rift.finish_turn"),
                    |transaction| -> Result<(), InteractionError> {
                        transaction
                            .raw_state_mut()
                            .set_active_players(vec![next])?;
                        Ok(())
                    },
                )?;
                Ok(InteractionFlow::FinishTurn)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nydra_core::{GameTimeline, InteractionDriver, InteractionUpdate, StateChange};

    fn choose_entity(
        driver: &InteractionDriver<RiftInteractionRules>,
        entity: EntityId,
    ) -> Choice {
        driver
            .interaction()
            .choices
            .iter()
            .find(|choice| matches!(choice.kind, ChoiceKind::SelectEntity { entity: id } if id == entity))
            .unwrap()
            .clone()
    }

    fn choose_ability(
        driver: &InteractionDriver<RiftInteractionRules>,
        ability: AbilityId,
    ) -> Choice {
        driver
            .interaction()
            .choices
            .iter()
            .find(|choice| matches!(choice.kind, ChoiceKind::SelectAbility { ability: id } if id == ability))
            .unwrap()
            .clone()
    }

    #[test]
    fn move_then_fireball_is_two_authoritative_steps_and_can_remove_target() {
        let state = standard_state();
        let turn = TurnSession::new(&state, PLAYER_ONE).unwrap();
        let mut driver = InteractionDriver::new(RiftInteractionRules::default(), turn).unwrap();

        let actor = choose_entity(&driver, MAGE_ONE);
        driver.choose(actor.id).unwrap();
        let movement = driver
            .interaction()
            .choices
            .iter()
            .find(|choice| {
                matches!(choice.kind, ChoiceKind::SelectPosition { position } if position == Position::new(1, 0))
            })
            .unwrap()
            .clone();
        driver.choose(movement.id).unwrap();
        let fireball = choose_ability(&driver, FIREBALL);
        driver.choose(fireball.id).unwrap();
        let target = choose_entity(&driver, MAGE_THREE);
        driver.choose(target.id).unwrap();
        let overcharge = driver
            .interaction()
            .choices
            .iter()
            .find(|choice| matches!(&choice.kind, ChoiceKind::SelectOption { key } if key == "overcharge"))
            .unwrap()
            .clone();
        assert_eq!(
            driver.choose(overcharge.id).unwrap(),
            InteractionUpdate::Finished
        );

        assert_eq!(driver.turn().steps.len(), 2);
        assert_eq!(driver.turn().steps[0].action.kind, "rift.move");
        assert_eq!(driver.turn().steps[1].action.kind, "rift.ability");
        assert!(driver.turn().working.entity(MAGE_THREE).is_err());
        assert_eq!(
            entity_u64(driver.turn().working.entity(MAGE_ONE).unwrap(), MANA),
            Some(1)
        );
        assert_eq!(driver.turn().working.turn.active_players, vec![PLAYER_TWO]);
        assert!(driver.turn().steps[1]
            .delta
            .changes
            .iter()
            .any(|change| matches!(change, StateChange::EntityRemoved { entity } if entity.id == MAGE_THREE)));
        assert_eq!(driver.turn().steps[1].presentation[0].kind, "rift.fireball");

        let outcome = registry()
            .outcome(RuleContext::from_state(&driver.turn().working, None))
            .unwrap()
            .unwrap();
        assert_eq!(outcome.key, "rift.last_team_standing");
        assert_eq!(outcome.winning_teams, vec![TEAM_ALPHA]);
        assert_eq!(outcome.winners, vec![PLAYER_ONE, PLAYER_TWO]);
    }

    #[test]
    fn normal_fireball_mutates_target_hp_without_removing_the_entity() {
        let state = standard_state();
        let turn = TurnSession::new(&state, PLAYER_ONE).unwrap();
        let mut driver = InteractionDriver::new(RiftInteractionRules::default(), turn).unwrap();
        let actor = choose_entity(&driver, MAGE_ONE);
        driver.choose(actor.id).unwrap();
        let movement = driver.interaction().choices[0].clone();
        driver.choose(movement.id).unwrap();
        let fireball = choose_ability(&driver, FIREBALL);
        driver.choose(fireball.id).unwrap();
        let target = choose_entity(&driver, MAGE_THREE);
        driver.choose(target.id).unwrap();
        let normal = driver
            .interaction()
            .choices
            .iter()
            .find(|choice| matches!(&choice.kind, ChoiceKind::SelectOption { key } if key == "normal"))
            .unwrap()
            .clone();
        driver.choose(normal.id).unwrap();

        assert_eq!(
            entity_u64(driver.turn().working.entity(MAGE_THREE).unwrap(), HP),
            Some(20)
        );
        assert!(driver.turn().steps[1]
            .delta
            .changes
            .iter()
            .any(|change| matches!(change, StateChange::EntityStateChanged { entity, .. } if *entity == MAGE_THREE)));
    }

    #[test]
    fn teams_are_independent_from_players_and_hijack_changes_only_controller() {
        let state = standard_state();
        assert_eq!(state.players[&PLAYER_ONE].team, Some(TEAM_ALPHA));
        assert_eq!(state.players[&PLAYER_TWO].team, Some(TEAM_ALPHA));
        assert_eq!(state.players[&PLAYER_THREE].team, Some(TEAM_BETA));

        let turn = TurnSession::new(&state, PLAYER_ONE).unwrap();
        let mut driver = InteractionDriver::new(RiftInteractionRules::default(), turn).unwrap();
        let actor = choose_entity(&driver, MAGE_ONE);
        driver.choose(actor.id).unwrap();
        let movement = driver.interaction().choices[0].clone();
        driver.choose(movement.id).unwrap();
        let hijack = choose_ability(&driver, HIJACK);
        driver.choose(hijack.id).unwrap();
        let target = choose_entity(&driver, MAGE_THREE);
        assert_eq!(driver.choose(target.id).unwrap(), InteractionUpdate::Finished);

        let target = driver.turn().working.entity(MAGE_THREE).unwrap();
        assert_eq!(target.owner, PLAYER_THREE);
        assert_eq!(target.controller, PLAYER_ONE);
        assert!(driver.turn().steps[1]
            .delta
            .changes
            .iter()
            .any(|change| matches!(change, StateChange::EntityControllerChanged { entity, from, to } if *entity == MAGE_THREE && *from == PLAYER_THREE && *to == PLAYER_ONE)));
    }

    #[test]
    fn rewind_reads_committed_history_and_restores_old_entity_data() {
        let mut timeline = GameTimeline::new(standard_state()).unwrap();
        let mut damage_turn = timeline.begin_turn(PLAYER_ONE).unwrap();
        damage_turn
            .apply_step(RecordedAction::new("test.damage"), |state| {
                state.entity_mut(MAGE_ONE)?.state.insert(HP, 25_u64);
                Ok(())
            })
            .unwrap();
        timeline.commit_turn(damage_turn).unwrap();
        assert_eq!(
            entity_u64(timeline.current().entity(MAGE_ONE).unwrap(), HP),
            Some(25)
        );

        let turn = timeline.begin_turn(PLAYER_ONE).unwrap();
        let mut driver = InteractionDriver::new(
            RiftInteractionRules::with_history(timeline.history()),
            turn,
        )
        .unwrap();
        let actor = choose_entity(&driver, MAGE_ONE);
        driver.choose(actor.id).unwrap();
        let movement = driver.interaction().choices[0].clone();
        driver.choose(movement.id).unwrap();
        let rewind = choose_ability(&driver, REWIND);
        driver.choose(rewind.id).unwrap();
        let self_target = choose_entity(&driver, MAGE_ONE);
        assert_eq!(
            driver.choose(self_target.id).unwrap(),
            InteractionUpdate::Finished
        );

        assert_eq!(
            entity_u64(driver.turn().working.entity(MAGE_ONE).unwrap(), HP),
            Some(100)
        );
        assert_eq!(driver.turn().steps.len(), 2);
        assert_eq!(driver.turn().steps[1].presentation[0].kind, "rift.rewind");
    }

    #[test]
    fn entity_and_ability_rules_are_registered_without_mechanic_specific_core_types() {
        let state = standard_state();
        let registry = registry();
        let presentation = registry
            .presentation(RuleContext::from_state(&state, None), MAGE_ONE)
            .unwrap();
        assert_eq!(presentation.asset_key, "rift/mage");
        assert_eq!(
            presentation.data.get("hp").and_then(StateValue::as_u64),
            Some(100)
        );
        assert!(registry.ability_rule(FIREBALL).is_ok());
        assert!(registry.ability_rule(REWIND).is_ok());
        assert!(registry.ability_rule(HIJACK).is_ok());
    }
}
