use crate::{
    ChessError, ChessRules, ChessSide, PseudoMove, BISHOP, KNIGHT, PAWN, QUEEN, ROOK,
};
use glorichess_core::{
    Choice, ChoiceKind, ChoiceSpec, EntityId, History, InteractionError, InteractionFlow,
    InteractionRules, StateMap, TurnSession,
};

const SELECTED_ENTITY: &str = "chess.selected_entity";
const PENDING_X: &str = "chess.pending_x";
const PENDING_Y: &str = "chess.pending_y";

pub struct ChessInteractionRules {
    rules: ChessRules,
    history: Option<History>,
}

impl ChessInteractionRules {
    pub fn new(rules: &ChessRules) -> Self {
        Self {
            rules: rules.clone(),
            history: None,
        }
    }

    pub fn with_history(rules: &ChessRules, history: &History) -> Self {
        Self {
            rules: rules.clone(),
            history: Some(history.clone()),
        }
    }

    fn side_to_move(&self, turn: &TurnSession) -> Result<ChessSide, InteractionError> {
        let [player] = turn.working.turn.active_players.as_slice() else {
            return Err(InteractionError::RuleViolation(
                "standard chess requires exactly one active player".into(),
            ));
        };
        ChessSide::from_player(*player).ok_or_else(|| {
            InteractionError::RuleViolation(format!("active player {player} is not a chess side"))
        })
    }

    pub fn selected_entity(draft: &StateMap) -> Option<EntityId> {
        let raw = draft.get(SELECTED_ENTITY)?.as_u64()?;
        let raw = u32::try_from(raw).ok()?;
        Some(EntityId::new(raw))
    }

    fn set_selected(draft: &mut StateMap, entity: EntityId) {
        draft.insert(SELECTED_ENTITY, u64::from(entity.get()));
        draft.remove(PENDING_X);
        draft.remove(PENDING_Y);
    }

    pub fn pending_target(draft: &StateMap) -> Option<glorichess_core::Position> {
        let x = u16::try_from(draft.get(PENDING_X)?.as_u64()?).ok()?;
        let y = u16::try_from(draft.get(PENDING_Y)?.as_u64()?).ok()?;
        Some(glorichess_core::Position::new(x, y))
    }

    fn set_pending_target(draft: &mut StateMap, target: glorichess_core::Position) {
        draft.insert(PENDING_X, u64::from(target.x));
        draft.insert(PENDING_Y, u64::from(target.y));
    }

    fn clear_selection(draft: &mut StateMap) {
        draft.remove(SELECTED_ENTITY);
        draft.remove(PENDING_X);
        draft.remove(PENDING_Y);
    }

    fn rule_error(error: ChessError) -> InteractionError {
        InteractionError::RuleViolation(error.to_string())
    }

    fn legal_moves(
        &self,
        turn: &TurnSession,
        entity: EntityId,
    ) -> Result<Vec<PseudoMove>, InteractionError> {
        self.rules
            .legal_moves_with_history(&turn.working, self.history.as_ref(), entity)
            .map_err(Self::rule_error)
    }

    fn promotion_type(key: &str) -> Option<glorichess_core::EntityTypeId> {
        match key {
            "queen" => Some(QUEEN),
            "rook" => Some(ROOK),
            "bishop" => Some(BISHOP),
            "knight" => Some(KNIGHT),
            _ => None,
        }
    }

    fn execute(
        &self,
        turn: &mut TurnSession,
        movement: PseudoMove,
        promotion: Option<glorichess_core::EntityTypeId>,
    ) -> Result<InteractionFlow, InteractionError> {
        self.rules
            .execute_move(turn, self.history.as_ref(), movement, promotion)
            .map_err(Self::rule_error)?;
        Ok(InteractionFlow::FinishTurn)
    }
}

impl InteractionRules for ChessInteractionRules {
    fn choices(
        &self,
        turn: &TurnSession,
        draft: &StateMap,
    ) -> Result<Vec<ChoiceSpec>, InteractionError> {
        let side = self.side_to_move(turn)?;
        let empty_history = History::default();
        let history = self.history.as_ref().unwrap_or(&empty_history);
        if self.rules.status(&turn.working, history).map_err(Self::rule_error)?.outcome.is_some() {
            return Ok(Vec::new());
        }

        if let Some(target) = Self::pending_target(draft) {
            let actor = Self::selected_entity(draft).ok_or_else(|| {
                InteractionError::RuleViolation("promotion target has no selected pawn".into())
            })?;
            let piece = turn.working.entity(actor)?;
            let side = ChessSide::from_player(piece.owner).ok_or_else(|| {
                InteractionError::RuleViolation("promotion pawn has no chess side".into())
            })?;
            let side_name = match side {
                ChessSide::White => "white",
                ChessSide::Black => "black",
            };
            let mut choices = Vec::new();
            for (key, label, entity_type) in [
                ("queen", "Queen", QUEEN),
                ("rook", "Rook", ROOK),
                ("bishop", "Bishop", BISHOP),
                ("knight", "Knight", KNIGHT),
            ] {
                let mut choice = ChoiceSpec::option(key).with_label(label);
                choice.data.insert("actor", u64::from(actor.get()));
                choice.data.insert("target_x", u64::from(target.x));
                choice.data.insert("target_y", u64::from(target.y));
                choice
                    .data
                    .insert("entity_type", u64::from(entity_type.get()));
                choice
                    .data
                    .insert("asset_key", format!("chess/{side_name}/{key}"));
                choices.push(choice);
            }
            return Ok(choices);
        }

        let mut choices = Vec::new();
        for entity in turn
            .working
            .entities
            .values()
            .filter(|entity| entity.controller == side.player())
        {
            let legal_moves = self.legal_moves(turn, entity.id)?;
            if legal_moves.is_empty() {
                continue;
            }
            choices.push(ChoiceSpec::entity(entity.id));
            for movement in legal_moves {
                let mut choice = ChoiceSpec::position(movement.to);
                choice.data.insert("actor", u64::from(entity.id.get()));
                if let Some(capture) = movement.capture {
                    choice.data.insert("capture", u64::from(capture.get()));
                }
                choice.data.insert("move_kind", match movement.kind {
                    crate::ChessMoveKind::Normal => "normal",
                    crate::ChessMoveKind::EnPassant { .. } => "en_passant",
                    crate::ChessMoveKind::Castle { .. } => "castle",
                });
                choices.push(choice);
            }
        }

        Ok(choices)
    }

    fn apply_choice(
        &self,
        turn: &mut TurnSession,
        draft: &mut StateMap,
        choice: &Choice,
    ) -> Result<InteractionFlow, InteractionError> {
        match &choice.kind {
            ChoiceKind::SelectEntity { entity } => {
                if Self::selected_entity(draft) == Some(*entity) {
                    Self::clear_selection(draft);
                    return Ok(InteractionFlow::Continue);
                }
                let side = self.side_to_move(turn)?;
                let piece = turn.working.entity(*entity)?;
                if piece.controller != side.player() {
                    return Err(InteractionError::RuleViolation(
                        "selected entity is not controlled by the active player".into(),
                    ));
                }
                if self.legal_moves(turn, *entity)?.is_empty() {
                    return Err(InteractionError::RuleViolation(
                        "selected entity has no legal moves".into(),
                    ));
                }
                Self::set_selected(draft, *entity);
                Ok(InteractionFlow::Continue)
            }
            ChoiceKind::SelectPosition { position } => {
                let actor_from_choice = choice
                    .data
                    .get("actor")
                    .and_then(glorichess_core::StateValue::as_u64)
                    .and_then(|value| u32::try_from(value).ok())
                    .map(EntityId::new)
                    .ok_or_else(|| {
                        InteractionError::RuleViolation("destination choice has no actor".into())
                    })?;
                let actor = match Self::selected_entity(draft) {
                    Some(selected) if selected != actor_from_choice => {
                        return Err(InteractionError::RuleViolation(
                            "destination belongs to a different selected entity".into(),
                        ));
                    }
                    Some(selected) => selected,
                    None => {
                        Self::set_selected(draft, actor_from_choice);
                        actor_from_choice
                    }
                };
                let movement = self
                    .legal_moves(turn, actor)?
                    .into_iter()
                    .find(|movement| movement.to == *position)
                    .ok_or_else(|| {
                        InteractionError::RuleViolation("selected destination is not legal".into())
                    })?;
                let piece = turn.working.entity(actor)?;
                let side = ChessSide::from_player(piece.owner).ok_or_else(|| {
                    InteractionError::RuleViolation("selected entity has no chess side".into())
                })?;
                if piece.entity_type == PAWN && position.y == side.promotion_rank() {
                    Self::set_pending_target(draft, *position);
                    return Ok(InteractionFlow::Continue);
                }
                let result = self.execute(turn, movement, None)?;
                Self::clear_selection(draft);
                Ok(result)
            }
            ChoiceKind::SelectOption { key } => {
                let actor = Self::selected_entity(draft).ok_or_else(|| {
                    InteractionError::RuleViolation("no pawn is selected for promotion".into())
                })?;
                let target = Self::pending_target(draft).ok_or_else(|| {
                    InteractionError::RuleViolation("no promotion is pending".into())
                })?;
                let promotion = Self::promotion_type(key).ok_or_else(|| {
                    InteractionError::RuleViolation("invalid promotion choice".into())
                })?;
                let movement = self
                    .legal_moves(turn, actor)?
                    .into_iter()
                    .find(|movement| movement.to == target)
                    .ok_or_else(|| {
                        InteractionError::RuleViolation("promotion move is no longer legal".into())
                    })?;
                let result = self.execute(turn, movement, Some(promotion))?;
                Self::clear_selection(draft);
                Ok(result)
            }
            _ => Err(InteractionError::RuleViolation(
                "choice is not valid for the current chess interaction".into(),
            )),
        }
    }
}
