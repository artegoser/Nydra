use crate::{
    AbilityId, ChoiceSpec, CoreError, EntityId, EntityState, EntityTypeId, GameState, History,
    PlayerId, StateMap, Transaction, TurnSession,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EntityPresentation {
    /// Stable presentation key resolved by the frontend, e.g. `chess/white/knight`.
    pub asset_key: String,
    pub label: Option<String>,
    pub data: StateMap,
}

impl EntityPresentation {
    pub fn new(asset_key: impl Into<String>) -> Self {
        Self {
            asset_key: asset_key.into(),
            label: None,
            data: StateMap::new(),
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_data(mut self, data: StateMap) -> Self {
        self.data = data;
        self
    }
}

#[derive(Clone, Copy)]
pub struct RuleContext<'a> {
    state: &'a GameState,
    history: Option<&'a History>,
    turn: Option<&'a TurnSession>,
}

impl<'a> RuleContext<'a> {
    pub fn from_state(state: &'a GameState, history: Option<&'a History>) -> Self {
        Self {
            state,
            history,
            turn: None,
        }
    }

    pub fn from_turn(turn: &'a TurnSession, history: Option<&'a History>) -> Self {
        Self {
            state: &turn.working,
            history,
            turn: Some(turn),
        }
    }

    pub fn state(self) -> &'a GameState {
        self.state
    }

    pub fn history(self) -> Option<&'a History> {
        self.history
    }

    pub fn turn(self) -> Option<&'a TurnSession> {
        self.turn
    }

    pub fn entity(self, entity: EntityId) -> Result<&'a EntityState, CoreError> {
        self.state.entity(entity)
    }

    pub fn entity_context(self, entity: EntityId) -> Result<EntityRuleContext<'a>, CoreError> {
        Ok(EntityRuleContext {
            world: self,
            entity: self.entity(entity)?,
        })
    }
}

#[derive(Clone, Copy)]
pub struct EntityRuleContext<'a> {
    world: RuleContext<'a>,
    entity: &'a EntityState,
}

impl<'a> EntityRuleContext<'a> {
    pub fn world(self) -> RuleContext<'a> {
        self.world
    }

    pub fn state(self) -> &'a GameState {
        self.world.state()
    }

    pub fn history(self) -> Option<&'a History> {
        self.world.history()
    }

    pub fn turn(self) -> Option<&'a TurnSession> {
        self.world.turn()
    }

    pub fn entity(self) -> &'a EntityState {
        self.entity
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum RuleError {
    #[error(transparent)]
    Core(#[from] CoreError),
    #[error("entity rule {0} is already registered")]
    DuplicateEntityRule(EntityTypeId),
    #[error("entity rule {0} is not registered")]
    EntityRuleNotFound(EntityTypeId),
    #[error("ability rule {0} is already registered")]
    DuplicateAbilityRule(AbilityId),
    #[error("ability rule {0} is not registered")]
    AbilityRuleNotFound(AbilityId),
    #[error("rule rejected operation: {0}")]
    Rejected(String),
}

/// Generic behavior attached to an entity type. Game-specific crates may add
/// richer traits on top of this one without teaching core those concepts.
pub trait EntityRule {
    fn presentation(&self, context: EntityRuleContext<'_>)
        -> Result<EntityPresentation, RuleError>;

    fn choices(
        &self,
        _context: EntityRuleContext<'_>,
        _draft: &StateMap,
    ) -> Result<Vec<ChoiceSpec>, RuleError> {
        Ok(Vec::new())
    }
}

/// Reserved extension point for explicit abilities. An ability may mutate the
/// whole transactional state and is not constrained to a closed effect enum.
pub trait AbilityRule {
    fn choices(
        &self,
        _context: RuleContext<'_>,
        _actor: EntityId,
        _draft: &StateMap,
    ) -> Result<Vec<ChoiceSpec>, RuleError> {
        Ok(Vec::new())
    }

    fn execute(
        &self,
        context: RuleContext<'_>,
        actor: EntityId,
        transaction: &mut Transaction,
        input: &StateMap,
    ) -> Result<(), RuleError>;
}

/// Ruleset-wide hooks for constraints that do not belong to one entity.
pub trait GameRule {
    fn validate(&self, _context: RuleContext<'_>) -> Result<(), RuleError> {
        Ok(())
    }

    fn choices(
        &self,
        _context: RuleContext<'_>,
        _actor: PlayerId,
        _draft: &StateMap,
    ) -> Result<Vec<ChoiceSpec>, RuleError> {
        Ok(Vec::new())
    }
}

#[derive(Default)]
pub struct RuleRegistry {
    entity_rules: BTreeMap<EntityTypeId, Box<dyn EntityRule>>,
    ability_rules: BTreeMap<AbilityId, Box<dyn AbilityRule>>,
    game_rule: Option<Box<dyn GameRule>>,
}

impl RuleRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_entity<R>(
        &mut self,
        entity_type: EntityTypeId,
        rule: R,
    ) -> Result<(), RuleError>
    where
        R: EntityRule + 'static,
    {
        if self.entity_rules.contains_key(&entity_type) {
            return Err(RuleError::DuplicateEntityRule(entity_type));
        }
        self.entity_rules.insert(entity_type, Box::new(rule));
        Ok(())
    }

    pub fn entity_rule(&self, entity_type: EntityTypeId) -> Result<&dyn EntityRule, RuleError> {
        self.entity_rules
            .get(&entity_type)
            .map(Box::as_ref)
            .ok_or(RuleError::EntityRuleNotFound(entity_type))
    }

    pub fn presentation(
        &self,
        context: RuleContext<'_>,
        entity: EntityId,
    ) -> Result<EntityPresentation, RuleError> {
        let entity_context = context.entity_context(entity)?;
        self.entity_rule(entity_context.entity().entity_type)?
            .presentation(entity_context)
    }

    pub fn register_ability<R>(&mut self, ability: AbilityId, rule: R) -> Result<(), RuleError>
    where
        R: AbilityRule + 'static,
    {
        if self.ability_rules.contains_key(&ability) {
            return Err(RuleError::DuplicateAbilityRule(ability));
        }
        self.ability_rules.insert(ability, Box::new(rule));
        Ok(())
    }

    pub fn ability_rule(&self, ability: AbilityId) -> Result<&dyn AbilityRule, RuleError> {
        self.ability_rules
            .get(&ability)
            .map(Box::as_ref)
            .ok_or(RuleError::AbilityRuleNotFound(ability))
    }

    pub fn set_game_rule<R>(&mut self, rule: R)
    where
        R: GameRule + 'static,
    {
        self.game_rule = Some(Box::new(rule));
    }

    pub fn game_rule(&self) -> Option<&dyn GameRule> {
        self.game_rule.as_deref()
    }
}
