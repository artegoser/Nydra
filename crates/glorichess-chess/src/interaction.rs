use crate::{ChessError, ChessRules, ChessSide};
use glorichess_core::{
    Choice, ChoiceKind, ChoiceSpec, EntityId, InteractionError, InteractionFlow, InteractionRules,
    RecordedAction, StateMap, TurnSession,
};

const SELECTED_ENTITY: &str = "chess.selected_entity";

pub struct ChessInteractionRules<'a> {
    rules: &'a ChessRules,
}

impl<'a> ChessInteractionRules<'a> {
    pub const fn new(rules: &'a ChessRules) -> Self {
        Self { rules }
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

    fn selected(draft: &StateMap) -> Option<EntityId> {
        let raw = draft.get(SELECTED_ENTITY)?.as_u64()?;
        let raw = u32::try_from(raw).ok()?;
        Some(EntityId::new(raw))
    }

    fn set_selected(draft: &mut StateMap, entity: EntityId) {
        draft.insert(SELECTED_ENTITY, u64::from(entity.get()));
    }

    fn rule_error(error: ChessError) -> InteractionError {
        InteractionError::RuleViolation(error.to_string())
    }
}

impl InteractionRules for ChessInteractionRules<'_> {
    fn choices(
        &self,
        turn: &TurnSession,
        draft: &StateMap,
    ) -> Result<Vec<ChoiceSpec>, InteractionError> {
        let side = self.side_to_move(turn)?;
        let mut choices = Vec::new();

        for entity in turn
            .working
            .entities
            .values()
            .filter(|entity| entity.controller == side.player())
        {
            if !self
                .rules
                .legal_moves(&turn.working, entity.id)
                .map_err(Self::rule_error)?
                .is_empty()
            {
                choices.push(ChoiceSpec::entity(entity.id));
            }
        }

        if let Some(entity) = Self::selected(draft) {
            if turn.working.entity(entity).is_ok() {
                for movement in self
                    .rules
                    .legal_moves(&turn.working, entity)
                    .map_err(Self::rule_error)?
                {
                    let mut choice = ChoiceSpec::position(movement.to);
                    if let Some(capture) = movement.capture {
                        choice.data.insert("capture", u64::from(capture.get()));
                    }
                    choices.push(choice);
                }
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
        match choice.kind {
            ChoiceKind::SelectEntity { entity } => {
                let side = self.side_to_move(turn)?;
                let piece = turn.working.entity(entity)?;
                if piece.controller != side.player() {
                    return Err(InteractionError::RuleViolation(
                        "selected entity is not controlled by the active player".into(),
                    ));
                }
                if self
                    .rules
                    .legal_moves(&turn.working, entity)
                    .map_err(Self::rule_error)?
                    .is_empty()
                {
                    return Err(InteractionError::RuleViolation(
                        "selected entity has no legal moves".into(),
                    ));
                }
                Self::set_selected(draft, entity);
                Ok(InteractionFlow::Continue)
            }
            ChoiceKind::SelectPosition { position } => {
                let actor = Self::selected(draft).ok_or_else(|| {
                    InteractionError::RuleViolation("no chess entity is selected".into())
                })?;
                let movement = self
                    .rules
                    .legal_moves(&turn.working, actor)
                    .map_err(Self::rule_error)?
                    .into_iter()
                    .find(|movement| movement.to == position)
                    .ok_or_else(|| {
                        InteractionError::RuleViolation("selected destination is not legal".into())
                    })?;
                let side = ChessSide::from_player(turn.working.entity(actor)?.owner).ok_or_else(|| {
                    InteractionError::RuleViolation("selected entity has no chess side".into())
                })?;
                turn.apply_transaction(RecordedAction::new("chess_move"), |transaction| {
                    if let Some(captured) = movement.capture {
                        transaction.remove_entity(captured)?;
                    }
                    transaction.move_entity(movement.actor, movement.to)?;
                    transaction.turn_state_mut().active_players = vec![side.opponent().player()];
                    Ok::<_, ChessError>(())
                })
                .map_err(Self::rule_error)?;
                draft.remove(SELECTED_ENTITY);
                Ok(InteractionFlow::FinishTurn)
            }
            _ => Err(InteractionError::RuleViolation(
                "choice is not valid for the current chess interaction".into(),
            )),
        }
    }
}
