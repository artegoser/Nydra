//! Browser boundary for the GloriChess Rust runtime.
#![forbid(unsafe_code)]

use glorichess_chess::{
    standard_chess_state, ChessInteractionRules, ChessOutcome, ChessRules, ChessSide,
};
use glorichess_core::{
    Choice, ChoiceId, ChoiceKind, EntityState, GameState, GameTimeline, InteractionDriver,
    InteractionUpdate, PlayerState, Position, PresentationCue, StateChange, StateDelta, StateMap,
    StepRecord, TeamState, TurnState,
};
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn runtime_name() -> String {
    "glorichess".to_owned()
}

#[wasm_bindgen]
pub fn new_chess() -> Result<GameHandle, JsValue> {
    GameHandle::new_standard().map_err(js_error)
}

#[wasm_bindgen]
pub fn from_fen(fen: &str) -> Result<GameHandle, JsValue> {
    GameHandle::new_from_fen(fen).map_err(js_error)
}

#[wasm_bindgen]
pub struct GameHandle {
    rules: ChessRules,
    timeline: GameTimeline,
    interaction: Option<InteractionDriver<ChessInteractionRules>>,
    undo_floor: usize,
}

impl GameHandle {
    fn new_standard() -> Result<Self, String> {
        let rules = ChessRules::standard();
        let timeline = GameTimeline::new(standard_chess_state().map_err(string_error)?)
            .map_err(string_error)?;
        let mut game = Self {
            rules,
            timeline,
            interaction: None,
            undo_floor: 0,
        };
        game.rebuild_interaction()?;
        Ok(game)
    }

    fn new_from_fen(fen: &str) -> Result<Self, String> {
        let rules = ChessRules::standard();
        let imported = rules.from_fen(fen).map_err(string_error)?;
        let mut game = Self {
            rules,
            timeline: imported.timeline,
            interaction: None,
            undo_floor: imported.synthetic_history_len,
        };
        game.rebuild_interaction()?;
        Ok(game)
    }

    fn rebuild_interaction(&mut self) -> Result<(), String> {
        let actor = self
            .timeline
            .current()
            .turn
            .active_players
            .first()
            .copied()
            .ok_or_else(|| "chess state has no active player".to_owned())?;
        let turn = self.timeline.begin_turn(actor).map_err(string_error)?;
        let rules = ChessInteractionRules::with_history(&self.rules, self.timeline.history());
        self.interaction = Some(InteractionDriver::new(rules, turn).map_err(string_error)?);
        Ok(())
    }

    fn visible_state(&self) -> &GameState {
        self.interaction
            .as_ref()
            .map(|interaction| &interaction.turn().working)
            .unwrap_or_else(|| self.timeline.current())
    }

    fn game_view(&self) -> Result<GameView, String> {
        build_game_view(
            &self.rules,
            self.visible_state(),
            self.timeline.history(),
            self.can_undo_internal(),
            self.timeline.can_redo(),
        )
    }

    fn interaction_view(&self) -> InteractionView {
        self.interaction
            .as_ref()
            .map(|driver| InteractionView::from_driver(driver))
            .unwrap_or_default()
    }

    fn can_undo_internal(&self) -> bool {
        self.timeline.history().len() > self.undo_floor
    }

    fn transition_view(
        &self,
        committed: bool,
        steps: &[StepRecord],
    ) -> Result<TransitionView, String> {
        Ok(TransitionView {
            committed,
            game: self.game_view()?,
            interaction: self.interaction_view(),
            changes: changes_from_steps(steps),
            presentation: presentation_from_steps(steps),
        })
    }

    fn transition_from_delta(&self, delta: StateDelta) -> Result<TransitionView, String> {
        Ok(TransitionView {
            committed: true,
            game: self.game_view()?,
            interaction: self.interaction_view(),
            changes: delta.changes.iter().map(ChangeView::from).collect(),
            presentation: Vec::new(),
        })
    }
}

#[wasm_bindgen]
impl GameHandle {
    pub fn view(&self) -> Result<JsValue, JsValue> {
        to_js(&self.game_view().map_err(js_error)?)
    }

    pub fn interaction(&self) -> Result<JsValue, JsValue> {
        to_js(&self.interaction_view())
    }

    pub fn choose(&mut self, choice_id: &str) -> Result<JsValue, JsValue> {
        let raw = choice_id
            .parse::<u64>()
            .map_err(|_| js_error("choice id is not a valid integer"))?;
        let mut driver = self
            .interaction
            .take()
            .ok_or_else(|| js_error("interaction session is unavailable"))?;
        let previous_step_count = driver.turn().steps.len();
        let update = match driver.choose(ChoiceId::new(raw)) {
            Ok(update) => update,
            Err(error) => {
                self.interaction = Some(driver);
                return Err(js_error(error));
            }
        };
        let new_steps = driver.turn().steps[previous_step_count..].to_vec();

        match update {
            InteractionUpdate::Continued(_) => {
                self.interaction = Some(driver);
                let transition = self.transition_view(false, &new_steps).map_err(js_error)?;
                to_js(&transition)
            }
            InteractionUpdate::Finished => {
                let turn = driver.into_turn().map_err(js_error)?;
                self.timeline.commit_turn(turn).map_err(js_error)?;
                self.rebuild_interaction().map_err(js_error)?;
                let transition = self.transition_view(true, &new_steps).map_err(js_error)?;
                to_js(&transition)
            }
        }
    }

    #[wasm_bindgen(js_name = cancelSelection)]
    pub fn cancel_selection(&mut self) -> Result<JsValue, JsValue> {
        let driver = self
            .interaction
            .as_mut()
            .ok_or_else(|| js_error("interaction session is unavailable"))?;
        driver.reset_draft().map_err(js_error)?;
        to_js(&self.transition_view(false, &[]).map_err(js_error)?)
    }

    pub fn undo(&mut self) -> Result<JsValue, JsValue> {
        if !self.can_undo_internal() {
            return Err(js_error("there is no user turn to undo"));
        }
        let before = self.timeline.current().clone();
        self.timeline
            .undo()
            .ok_or_else(|| js_error("undo failed"))?;
        let delta = StateDelta::between(&before, self.timeline.current());
        self.rebuild_interaction().map_err(js_error)?;
        to_js(&self.transition_from_delta(delta).map_err(js_error)?)
    }

    pub fn redo(&mut self) -> Result<JsValue, JsValue> {
        let before = self.timeline.current().clone();
        self.timeline
            .redo()
            .ok_or_else(|| js_error("there is no turn to redo"))?;
        let delta = StateDelta::between(&before, self.timeline.current());
        self.rebuild_interaction().map_err(js_error)?;
        to_js(&self.transition_from_delta(delta).map_err(js_error)?)
    }

    pub fn fen(&self) -> Result<String, JsValue> {
        self.rules
            .to_fen(self.timeline.current(), self.timeline.history())
            .map_err(js_error)
    }

    pub fn history(&self) -> Result<JsValue, JsValue> {
        let turns = self
            .timeline
            .history()
            .turns()
            .iter()
            .enumerate()
            .filter(|(_, turn)| !turn.synthetic)
            .map(|(index, turn)| HistoryTurnView {
                index: u32::try_from(index).unwrap_or(u32::MAX),
                actor: turn.actor.get(),
                actions: turn
                    .steps
                    .iter()
                    .map(|step| ActionView {
                        kind: step.action.kind.clone(),
                        data: step.action.data.clone(),
                    })
                    .collect(),
            })
            .collect::<Vec<_>>();
        to_js(&turns)
    }

    #[wasm_bindgen(js_name = canUndo)]
    pub fn can_undo(&self) -> bool {
        self.can_undo_internal()
    }

    #[wasm_bindgen(js_name = canRedo)]
    pub fn can_redo(&self) -> bool {
        self.timeline.can_redo()
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct PositionView {
    pub x: u16,
    pub y: u16,
}

impl From<Position> for PositionView {
    fn from(value: Position) -> Self {
        Self {
            x: value.x,
            y: value.y,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct EntityView {
    pub id: u32,
    pub entity_type: u32,
    pub owner: u32,
    pub controller: u32,
    pub position: PositionView,
    pub move_count: u32,
    pub asset_key: String,
    pub label: Option<String>,
    pub presentation_data: StateMap,
    pub state: StateMap,
}

#[derive(Clone, Debug, Serialize)]
pub struct MoveEndpointsView {
    pub from: PositionView,
    pub to: PositionView,
}

#[derive(Clone, Debug, Serialize)]
pub struct StatusView {
    pub side_to_move: String,
    pub in_check: bool,
    pub checked_king: Option<PositionView>,
    pub outcome: Option<String>,
    pub winner: Option<u32>,
    pub loser: Option<u32>,
    pub repetition_count: usize,
    pub halfmove_clock: u16,
    pub can_claim_threefold_repetition: bool,
    pub can_claim_fifty_move_rule: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct GameView {
    pub width: u16,
    pub height: u16,
    pub entities: Vec<EntityView>,
    pub last_move: Option<MoveEndpointsView>,
    pub active_players: Vec<u32>,
    pub status: StatusView,
    pub can_undo: bool,
    pub can_redo: bool,
}

fn build_game_view(
    rules: &ChessRules,
    state: &GameState,
    history: &glorichess_core::History,
    can_undo: bool,
    can_redo: bool,
) -> Result<GameView, String> {
    let mut entities = Vec::with_capacity(state.entities.len());
    for entity in state.entities.values() {
        let presentation = rules
            .presentation(state, Some(history), entity.id)
            .map_err(string_error)?;
        entities.push(EntityView {
            id: entity.id.get(),
            entity_type: entity.entity_type.get(),
            owner: entity.owner.get(),
            controller: entity.controller.get(),
            position: entity.position.into(),
            move_count: entity.move_count,
            asset_key: presentation.asset_key,
            label: presentation.label,
            presentation_data: presentation.data,
            state: entity.state.clone(),
        });
    }

    let status = rules.status(state, history).map_err(string_error)?;
    let (outcome, winner, loser) = status
        .outcome
        .as_ref()
        .map(outcome_view)
        .unwrap_or((None, None, None));
    let checked_king = if status.in_check {
        rules
            .king(state, status.side_to_move)
            .ok()
            .and_then(|king| state.entity(king).ok())
            .map(|king| king.position.into())
    } else {
        None
    };

    Ok(GameView {
        width: state.board.width(),
        height: state.board.height(),
        entities,
        last_move: last_move_view(history),
        active_players: state
            .turn
            .active_players
            .iter()
            .map(|player| player.get())
            .collect(),
        status: StatusView {
            side_to_move: side_name(status.side_to_move).into(),
            in_check: status.in_check,
            checked_king,
            outcome,
            winner,
            loser,
            repetition_count: status.repetition_count,
            halfmove_clock: status.halfmove_clock,
            can_claim_threefold_repetition: status.can_claim_threefold_repetition,
            can_claim_fifty_move_rule: status.can_claim_fifty_move_rule,
        },
        can_undo,
        can_redo,
    })
}

fn last_move_view(history: &glorichess_core::History) -> Option<MoveEndpointsView> {
    let step = history
        .turns()
        .iter()
        .rev()
        .find(|turn| !turn.synthetic)?
        .steps
        .last()?;
    if step.action.kind != "chess_move" {
        return None;
    }
    let from = Position::new(
        u16::try_from(step.action.data.get("from_x")?.as_u64()?).ok()?,
        u16::try_from(step.action.data.get("from_y")?.as_u64()?).ok()?,
    );
    let to = Position::new(
        u16::try_from(step.action.data.get("to_x")?.as_u64()?).ok()?,
        u16::try_from(step.action.data.get("to_y")?.as_u64()?).ok()?,
    );
    Some(MoveEndpointsView {
        from: from.into(),
        to: to.into(),
    })
}

fn side_name(side: ChessSide) -> &'static str {
    match side {
        ChessSide::White => "white",
        ChessSide::Black => "black",
    }
}

fn outcome_view(outcome: &ChessOutcome) -> (Option<String>, Option<u32>, Option<u32>) {
    match outcome {
        ChessOutcome::Checkmate { winner, loser } => (
            Some("checkmate".into()),
            Some(winner.get()),
            Some(loser.get()),
        ),
        ChessOutcome::Stalemate => (Some("stalemate".into()), None, None),
        ChessOutcome::Resignation { winner, loser } => (
            Some("resignation".into()),
            Some(winner.get()),
            Some(loser.get()),
        ),
        ChessOutcome::DrawAgreement => (Some("draw_agreement".into()), None, None),
        ChessOutcome::ThreefoldRepetition => {
            (Some("threefold_repetition".into()), None, None)
        }
        ChessOutcome::FivefoldRepetition => (Some("fivefold_repetition".into()), None, None),
        ChessOutcome::FiftyMoveRule => (Some("fifty_move_rule".into()), None, None),
        ChessOutcome::SeventyFiveMoveRule => {
            (Some("seventy_five_move_rule".into()), None, None)
        }
        ChessOutcome::DeadPosition => (Some("dead_position".into()), None, None),
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct InteractionView {
    pub generation: String,
    pub selected_entity: Option<u32>,
    pub pending_target: Option<PositionView>,
    pub choices: Vec<ChoiceView>,
}

impl InteractionView {
    fn from_driver(driver: &InteractionDriver<ChessInteractionRules>) -> Self {
        Self {
            generation: driver.interaction().generation.to_string(),
            selected_entity: ChessInteractionRules::selected_entity(driver.draft())
                .map(|entity| entity.get()),
            pending_target: ChessInteractionRules::pending_target(driver.draft()).map(Into::into),
            choices: driver
                .interaction()
                .choices
                .iter()
                .map(ChoiceView::from)
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ChoiceView {
    pub id: String,
    pub kind: String,
    pub entity: Option<u32>,
    pub position: Option<PositionView>,
    pub ability: Option<u32>,
    pub option_key: Option<String>,
    pub label: Option<String>,
    pub actor: Option<u32>,
    pub capture: Option<u32>,
    pub move_kind: Option<String>,
    pub target_position: Option<PositionView>,
    pub option_entity_type: Option<u32>,
    pub asset_key: Option<String>,
    pub data: StateMap,
}

impl From<&Choice> for ChoiceView {
    fn from(choice: &Choice) -> Self {
        let mut view = Self {
            id: choice.id.get().to_string(),
            kind: String::new(),
            entity: None,
            position: None,
            ability: None,
            option_key: None,
            label: choice.label.clone(),
            actor: state_u32(&choice.data, "actor"),
            capture: state_u32(&choice.data, "capture"),
            move_kind: choice
                .data
                .get("move_kind")
                .and_then(glorichess_core::StateValue::as_str)
                .map(str::to_owned),
            target_position: match (
                state_u16(&choice.data, "target_x"),
                state_u16(&choice.data, "target_y"),
            ) {
                (Some(x), Some(y)) => Some(PositionView { x, y }),
                _ => None,
            },
            option_entity_type: state_u32(&choice.data, "entity_type"),
            asset_key: choice
                .data
                .get("asset_key")
                .and_then(glorichess_core::StateValue::as_str)
                .map(str::to_owned),
            data: choice.data.clone(),
        };
        match &choice.kind {
            ChoiceKind::SelectEntity { entity } => {
                view.kind = "select_entity".into();
                view.entity = Some(entity.get());
            }
            ChoiceKind::SelectPosition { position } => {
                view.kind = "select_position".into();
                view.position = Some((*position).into());
            }
            ChoiceKind::SelectAbility { ability } => {
                view.kind = "select_ability".into();
                view.ability = Some(ability.get());
            }
            ChoiceKind::SelectOption { key } => {
                view.kind = "select_option".into();
                view.option_key = Some(key.clone());
            }
            ChoiceKind::FinishTurn => {
                view.kind = "finish_turn".into();
            }
        }
        view
    }
}

fn state_u32(data: &StateMap, key: &str) -> Option<u32> {
    data.get(key)
        .and_then(glorichess_core::StateValue::as_u64)
        .and_then(|value| u32::try_from(value).ok())
}

fn state_u16(data: &StateMap, key: &str) -> Option<u16> {
    data.get(key)
        .and_then(glorichess_core::StateValue::as_u64)
        .and_then(|value| u16::try_from(value).ok())
}

#[derive(Clone, Debug, Serialize)]
pub struct EntitySnapshotView {
    pub id: u32,
    pub entity_type: u32,
    pub owner: u32,
    pub controller: u32,
    pub position: PositionView,
    pub move_count: u32,
    pub state: StateMap,
}

impl From<&EntityState> for EntitySnapshotView {
    fn from(entity: &EntityState) -> Self {
        Self {
            id: entity.id.get(),
            entity_type: entity.entity_type.get(),
            owner: entity.owner.get(),
            controller: entity.controller.get(),
            position: entity.position.into(),
            move_count: entity.move_count,
            state: entity.state.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct PlayerSnapshotView {
    pub id: u32,
    pub team: Option<u32>,
    pub state: StateMap,
}

impl From<&PlayerState> for PlayerSnapshotView {
    fn from(player: &PlayerState) -> Self {
        Self {
            id: player.id.get(),
            team: player.team.map(|team| team.get()),
            state: player.state.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct TeamSnapshotView {
    pub id: u32,
    pub state: StateMap,
}

impl From<&TeamState> for TeamSnapshotView {
    fn from(team: &TeamState) -> Self {
        Self {
            id: team.id.get(),
            state: team.state.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct TurnSnapshotView {
    pub active_players: Vec<u32>,
    pub state: StateMap,
}

impl From<&TurnState> for TurnSnapshotView {
    fn from(turn: &TurnState) -> Self {
        Self {
            active_players: turn
                .active_players
                .iter()
                .map(|player| player.get())
                .collect(),
            state: turn.state.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChangeView {
    EntityAdded { entity: EntitySnapshotView },
    EntityRemoved { entity: EntitySnapshotView },
    EntityMoved { entity: u32, from: PositionView, to: PositionView },
    EntityTypeChanged { entity: u32, from: u32, to: u32 },
    EntityOwnerChanged { entity: u32, from: u32, to: u32 },
    EntityControllerChanged { entity: u32, from: u32, to: u32 },
    EntityMoveCountChanged { entity: u32, from: u32, to: u32 },
    EntityStateChanged { entity: u32, before: StateMap, after: StateMap },
    PlayerAdded { player: PlayerSnapshotView },
    PlayerRemoved { player: PlayerSnapshotView },
    PlayerChanged {
        player: u32,
        before: StateMap,
        after: StateMap,
        before_team: Option<u32>,
        after_team: Option<u32>,
    },
    TeamAdded { team: TeamSnapshotView },
    TeamRemoved { team: TeamSnapshotView },
    TeamChanged { team: u32, before: StateMap, after: StateMap },
    TurnChanged { before: TurnSnapshotView, after: TurnSnapshotView },
    RulesetStateChanged { before: StateMap, after: StateMap },
}

impl From<&StateChange> for ChangeView {
    fn from(change: &StateChange) -> Self {
        match change {
            StateChange::EntityAdded { entity } => Self::EntityAdded {
                entity: entity.into(),
            },
            StateChange::EntityRemoved { entity } => Self::EntityRemoved {
                entity: entity.into(),
            },
            StateChange::EntityMoved { entity, from, to } => Self::EntityMoved {
                entity: entity.get(),
                from: (*from).into(),
                to: (*to).into(),
            },
            StateChange::EntityTypeChanged { entity, from, to } => Self::EntityTypeChanged {
                entity: entity.get(),
                from: from.get(),
                to: to.get(),
            },
            StateChange::EntityOwnerChanged { entity, from, to } => Self::EntityOwnerChanged {
                entity: entity.get(),
                from: from.get(),
                to: to.get(),
            },
            StateChange::EntityControllerChanged { entity, from, to } => {
                Self::EntityControllerChanged {
                    entity: entity.get(),
                    from: from.get(),
                    to: to.get(),
                }
            }
            StateChange::EntityMoveCountChanged { entity, from, to } => {
                Self::EntityMoveCountChanged {
                    entity: entity.get(),
                    from: *from,
                    to: *to,
                }
            }
            StateChange::EntityStateChanged { entity, before, after } => Self::EntityStateChanged {
                entity: entity.get(),
                before: before.clone(),
                after: after.clone(),
            },
            StateChange::PlayerAdded { player } => Self::PlayerAdded {
                player: player.into(),
            },
            StateChange::PlayerRemoved { player } => Self::PlayerRemoved {
                player: player.into(),
            },
            StateChange::PlayerChanged {
                player,
                before,
                after,
                before_team,
                after_team,
            } => Self::PlayerChanged {
                player: player.get(),
                before: before.clone(),
                after: after.clone(),
                before_team: before_team.map(|team| team.get()),
                after_team: after_team.map(|team| team.get()),
            },
            StateChange::TeamAdded { team } => Self::TeamAdded { team: team.into() },
            StateChange::TeamRemoved { team } => Self::TeamRemoved { team: team.into() },
            StateChange::TeamChanged { team, before, after } => Self::TeamChanged {
                team: team.get(),
                before: before.clone(),
                after: after.clone(),
            },
            StateChange::TurnChanged { before, after } => Self::TurnChanged {
                before: before.into(),
                after: after.into(),
            },
            StateChange::RulesetStateChanged { before, after } => Self::RulesetStateChanged {
                before: before.clone(),
                after: after.clone(),
            },
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct PresentationView {
    pub kind: String,
    pub data: StateMap,
}

impl From<&PresentationCue> for PresentationView {
    fn from(cue: &PresentationCue) -> Self {
        Self {
            kind: cue.kind.clone(),
            data: cue.data.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct TransitionView {
    pub committed: bool,
    pub game: GameView,
    pub interaction: InteractionView,
    pub changes: Vec<ChangeView>,
    pub presentation: Vec<PresentationView>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ActionView {
    pub kind: String,
    pub data: StateMap,
}

#[derive(Clone, Debug, Serialize)]
pub struct HistoryTurnView {
    pub index: u32,
    pub actor: u32,
    pub actions: Vec<ActionView>,
}

fn changes_from_steps(steps: &[StepRecord]) -> Vec<ChangeView> {
    steps
        .iter()
        .flat_map(|step| step.delta.changes.iter().map(ChangeView::from))
        .collect()
}

fn presentation_from_steps(steps: &[StepRecord]) -> Vec<PresentationView> {
    steps
        .iter()
        .flat_map(|step| step.presentation.iter().map(PresentationView::from))
        .collect()
}

fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(value).map_err(js_error)
}

fn js_error(error: impl ToString) -> JsValue {
    JsValue::from_str(&error.to_string())
}

fn string_error(error: impl ToString) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_runtime_exposes_board_and_choices() {
        let game = GameHandle::new_standard().unwrap();
        let view = game.game_view().unwrap();
        assert_eq!(view.entities.len(), 32);
        assert_eq!(view.status.side_to_move, "white");
        assert!(!game.interaction_view().choices.is_empty());
    }

    #[test]
    fn imported_synthetic_history_is_not_user_undoable_or_visible() {
        let game = GameHandle::new_from_fen(
            "4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 17",
        )
        .unwrap();
        assert!(!game.can_undo_internal());
        assert_eq!(game.timeline.history().len(), 1);
        assert!(game.timeline.history().previous_turn().unwrap().synthetic);
    }
}
