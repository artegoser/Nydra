//! Browser boundary for the Nydra Rust runtime.
#![forbid(unsafe_code)]

use nydra_checkers::{
    registry as checkers_registry, standard_state as standard_checkers_state, CheckersInteractionRules,
    BLACK as CHECKERS_BLACK, WHITE as CHECKERS_WHITE,
};
use nydra_chess::{
    standard_chess_state, ChessInteractionRules, ChessOutcome, ChessRules, ChessSide, STANDARD_FEN,
};
use nydra_core::{
    Choice, ChoiceId, ChoiceKind, EntityState, GameOutcome, GameState, GameTimeline, InteractionDriver,
    InteractionRules, InteractionUpdate, PlayerState, Position, PresentationCue, RuleContext, StateChange,
    StateDelta, StateMap, StepRecord, TeamState, TurnState,
};
use nydra_go::{
    empty_state as empty_go_state, registry as go_registry, GoInteractionRules, BLACK as GO_BLACK,
    WHITE as GO_WHITE,
};
use nydra_rift::{
    registry as rift_registry, standard_state as standard_rift_state, RiftInteractionRules,
};
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn runtime_name() -> String {
    "nydra".to_owned()
}

#[wasm_bindgen]
pub fn new_game(ruleset: &str) -> Result<GameHandle, JsValue> {
    GameHandle::new_ruleset(ruleset).map_err(js_error)
}

#[wasm_bindgen]
pub fn new_chess() -> Result<GameHandle, JsValue> {
    GameHandle::new_ruleset("chess").map_err(js_error)
}

#[wasm_bindgen]
pub fn from_fen(fen: &str) -> Result<GameHandle, JsValue> {
    Ok(GameHandle {
        inner: Runtime::Chess(ChessRuntime::from_fen(fen).map_err(js_error)?),
    })
}

#[wasm_bindgen]
pub fn from_pgn(pgn: &str) -> Result<GameHandle, JsValue> {
    Ok(GameHandle {
        inner: Runtime::Chess(ChessRuntime::from_pgn(pgn).map_err(js_error)?),
    })
}

#[wasm_bindgen]
pub struct GameHandle {
    inner: Runtime,
}

enum Runtime {
    Chess(ChessRuntime),
    Checkers(CheckersRuntime),
    Go(GoRuntime),
    Rift(RiftRuntime),
}

struct ChessRuntime {
    rules: ChessRules,
    timeline: GameTimeline,
    interaction: Option<InteractionDriver<ChessInteractionRules>>,
    undo_floor: usize,
    initial_fen: String,
}

struct CheckersRuntime {
    timeline: GameTimeline,
    interaction: Option<InteractionDriver<CheckersInteractionRules>>,
}

struct GoRuntime {
    timeline: GameTimeline,
    interaction: Option<InteractionDriver<GoInteractionRules>>,
}

struct RiftRuntime {
    timeline: GameTimeline,
    interaction: Option<InteractionDriver<RiftInteractionRules>>,
}

impl GameHandle {
    fn new_ruleset(ruleset: &str) -> Result<Self, String> {
        let inner = match ruleset {
            "chess" => Runtime::Chess(ChessRuntime::standard()?),
            "checkers" => Runtime::Checkers(CheckersRuntime::standard()?),
            "go" => Runtime::Go(GoRuntime::standard()?),
            "rift" => Runtime::Rift(RiftRuntime::standard()?),
            other => return Err(format!("unknown Nydra ruleset '{other}'")),
        };
        Ok(Self { inner })
    }

    fn visible_state(&self) -> &GameState {
        match &self.inner {
            Runtime::Chess(runtime) => visible_state(&runtime.timeline, runtime.interaction.as_ref()),
            Runtime::Checkers(runtime) => visible_state(&runtime.timeline, runtime.interaction.as_ref()),
            Runtime::Go(runtime) => visible_state(&runtime.timeline, runtime.interaction.as_ref()),
            Runtime::Rift(runtime) => visible_state(&runtime.timeline, runtime.interaction.as_ref()),
        }
    }

    fn timeline(&self) -> &GameTimeline {
        match &self.inner {
            Runtime::Chess(runtime) => &runtime.timeline,
            Runtime::Checkers(runtime) => &runtime.timeline,
            Runtime::Go(runtime) => &runtime.timeline,
            Runtime::Rift(runtime) => &runtime.timeline,
        }
    }

    fn game_view(&self) -> Result<GameView, String> {
        match &self.inner {
            Runtime::Chess(runtime) => runtime.game_view(),
            Runtime::Checkers(runtime) => runtime.game_view(),
            Runtime::Go(runtime) => runtime.game_view(),
            Runtime::Rift(runtime) => runtime.game_view(),
        }
    }

    fn interaction_view(&self) -> InteractionView {
        match &self.inner {
            Runtime::Chess(runtime) => runtime.interaction_view(),
            Runtime::Checkers(runtime) => runtime.interaction_view(),
            Runtime::Go(runtime) => runtime.interaction_view(),
            Runtime::Rift(runtime) => runtime.interaction_view(),
        }
    }

    fn transition_view(&self, committed: bool, steps: &[StepRecord]) -> Result<TransitionView, String> {
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
    pub fn ruleset(&self) -> String {
        match &self.inner {
            Runtime::Chess(_) => "chess",
            Runtime::Checkers(_) => "checkers",
            Runtime::Go(_) => "go",
            Runtime::Rift(_) => "rift",
        }
        .to_owned()
    }

    pub fn view(&self) -> Result<JsValue, JsValue> {
        to_js(&self.game_view().map_err(js_error)?)
    }

    pub fn interaction(&self) -> Result<JsValue, JsValue> {
        to_js(&self.interaction_view())
    }

    pub fn choose(&mut self, choice_id: &str) -> Result<JsValue, JsValue> {
        let id = parse_choice_id(choice_id)?;
        let (committed, steps) = match &mut self.inner {
            Runtime::Chess(runtime) => runtime.choose(id).map_err(js_error)?,
            Runtime::Checkers(runtime) => runtime.choose(id).map_err(js_error)?,
            Runtime::Go(runtime) => runtime.choose(id).map_err(js_error)?,
            Runtime::Rift(runtime) => runtime.choose(id).map_err(js_error)?,
        };
        to_js(&self.transition_view(committed, &steps).map_err(js_error)?)
    }

    #[wasm_bindgen(js_name = cancelSelection)]
    pub fn cancel_selection(&mut self) -> Result<JsValue, JsValue> {
        match &mut self.inner {
            Runtime::Chess(runtime) => reset_draft(&mut runtime.interaction).map_err(js_error)?,
            Runtime::Checkers(runtime) => reset_draft(&mut runtime.interaction).map_err(js_error)?,
            Runtime::Go(runtime) => reset_draft(&mut runtime.interaction).map_err(js_error)?,
            Runtime::Rift(runtime) => reset_draft(&mut runtime.interaction).map_err(js_error)?,
        }
        to_js(&self.transition_view(false, &[]).map_err(js_error)?)
    }

    pub fn undo(&mut self) -> Result<JsValue, JsValue> {
        let before = self.visible_state().clone();
        match &mut self.inner {
            Runtime::Chess(runtime) => runtime.undo().map_err(js_error)?,
            Runtime::Checkers(runtime) => runtime.undo().map_err(js_error)?,
            Runtime::Go(runtime) => runtime.undo().map_err(js_error)?,
            Runtime::Rift(runtime) => runtime.undo().map_err(js_error)?,
        }
        let delta = StateDelta::between(&before, self.visible_state());
        to_js(&self.transition_from_delta(delta).map_err(js_error)?)
    }

    pub fn redo(&mut self) -> Result<JsValue, JsValue> {
        let before = self.visible_state().clone();
        match &mut self.inner {
            Runtime::Chess(runtime) => runtime.redo().map_err(js_error)?,
            Runtime::Checkers(runtime) => runtime.redo().map_err(js_error)?,
            Runtime::Go(runtime) => runtime.redo().map_err(js_error)?,
            Runtime::Rift(runtime) => runtime.redo().map_err(js_error)?,
        }
        let delta = StateDelta::between(&before, self.visible_state());
        to_js(&self.transition_from_delta(delta).map_err(js_error)?)
    }

    pub fn fen(&self) -> Result<String, JsValue> {
        let Runtime::Chess(runtime) = &self.inner else {
            return Err(js_error("FEN is only available for the chess ruleset"));
        };
        runtime
            .rules
            .to_fen(runtime.timeline.current(), runtime.timeline.history())
            .map_err(js_error)
    }

    pub fn pgn(&self) -> Result<String, JsValue> {
        let Runtime::Chess(runtime) = &self.inner else {
            return Err(js_error("PGN is only available for the chess ruleset"));
        };
        runtime
            .rules
            .to_pgn(&runtime.initial_fen, runtime.timeline.history())
            .map_err(js_error)
    }

    pub fn history(&self) -> Result<JsValue, JsValue> {
        let turns = match &self.inner {
            Runtime::Chess(runtime) => runtime.history_view().map_err(js_error)?,
            Runtime::Checkers(runtime) => generic_history_view(runtime.timeline.history(), "checkers"),
            Runtime::Go(runtime) => generic_history_view(runtime.timeline.history(), "go"),
            Runtime::Rift(runtime) => generic_history_view(runtime.timeline.history(), "rift"),
        };
        to_js(&turns)
    }

    #[wasm_bindgen(js_name = canUndo)]
    pub fn can_undo(&self) -> bool {
        match &self.inner {
            Runtime::Chess(runtime) => runtime.can_undo(),
            Runtime::Checkers(runtime) => runtime.timeline.can_undo(),
            Runtime::Go(runtime) => runtime.timeline.can_undo(),
            Runtime::Rift(runtime) => runtime.timeline.can_undo(),
        }
    }

    #[wasm_bindgen(js_name = canRedo)]
    pub fn can_redo(&self) -> bool {
        self.timeline().can_redo()
    }
}

impl ChessRuntime {
    fn standard() -> Result<Self, String> {
        let rules = ChessRules::standard();
        let timeline = GameTimeline::new(standard_chess_state().map_err(string_error)?)
            .map_err(string_error)?;
        let mut runtime = Self {
            rules,
            timeline,
            interaction: None,
            undo_floor: 0,
            initial_fen: STANDARD_FEN.to_owned(),
        };
        runtime.rebuild_interaction()?;
        Ok(runtime)
    }

    fn from_fen(fen: &str) -> Result<Self, String> {
        let rules = ChessRules::standard();
        let imported = rules.from_fen(fen).map_err(string_error)?;
        let initial_fen = rules
            .to_fen(imported.timeline.current(), imported.timeline.history())
            .map_err(string_error)?;
        let mut runtime = Self {
            rules,
            timeline: imported.timeline,
            interaction: None,
            undo_floor: imported.synthetic_history_len,
            initial_fen,
        };
        runtime.rebuild_interaction()?;
        Ok(runtime)
    }

    fn from_pgn(pgn: &str) -> Result<Self, String> {
        let rules = ChessRules::standard();
        let imported = rules.from_pgn(pgn).map_err(string_error)?;
        let undo_floor = imported
            .timeline
            .history()
            .turns()
            .iter()
            .take_while(|turn| turn.synthetic)
            .count();
        let mut runtime = Self {
            rules,
            timeline: imported.timeline,
            interaction: None,
            undo_floor,
            initial_fen: imported.initial_fen,
        };
        runtime.rebuild_interaction()?;
        Ok(runtime)
    }

    fn can_undo(&self) -> bool {
        self.timeline.history().len() > self.undo_floor
    }

    fn rebuild_interaction(&mut self) -> Result<(), String> {
        if self.rules.status(self.timeline.current(), self.timeline.history()).map_err(string_error)?.outcome.is_some() {
            self.interaction = None;
            return Ok(());
        }
        let actor = active_actor(self.timeline.current(), "chess")?;
        let turn = self.timeline.begin_turn(actor).map_err(string_error)?;
        let rules = ChessInteractionRules::with_history(&self.rules, self.timeline.history());
        self.interaction = Some(InteractionDriver::new(rules, turn).map_err(string_error)?);
        Ok(())
    }

    fn choose(&mut self, id: ChoiceId) -> Result<(bool, Vec<StepRecord>), String> {
        let result = choose_driver(&mut self.interaction, &mut self.timeline, id)?;
        if result.0 { self.rebuild_interaction()?; }
        Ok(result)
    }

    fn undo(&mut self) -> Result<(), String> {
        if !self.can_undo() { return Err("there is no user turn to undo".into()); }
        self.timeline.undo().ok_or_else(|| "undo failed".to_owned())?;
        self.rebuild_interaction()
    }

    fn redo(&mut self) -> Result<(), String> {
        self.timeline.redo().ok_or_else(|| "there is no turn to redo".to_owned())?;
        self.rebuild_interaction()
    }

    fn game_view(&self) -> Result<GameView, String> {
        let state = visible_state(&self.timeline, self.interaction.as_ref());
        let mut entities = Vec::with_capacity(state.entities.len());
        for entity in state.entities.values() {
            let presentation = self.rules.presentation(state, Some(self.timeline.history()), entity.id).map_err(string_error)?;
            entities.push(entity_view(entity, presentation));
        }
        let status = self.rules.status(state, self.timeline.history()).map_err(string_error)?;
        let checked = if status.in_check {
            self.rules.king(state, status.side_to_move).ok().and_then(|id| state.entity(id).ok()).map(|entity| entity.position.into())
        } else { None };
        let outcome = status.outcome.as_ref().map(chess_outcome_to_generic);
        let mut details = StateMap::new();
        details.insert("repetition_count", status.repetition_count as u64);
        details.insert("halfmove_clock", u64::from(status.halfmove_clock));
        details.insert("can_claim_threefold_repetition", status.can_claim_threefold_repetition);
        details.insert("can_claim_fifty_move_rule", status.can_claim_fifty_move_rule);
        Ok(GameView::new(
            "chess", "Chess", "checkerboard", state, entities,
            if status.outcome.is_some() { outcome.as_ref().map(outcome_text).unwrap_or_default() } else { format!("{} to move{}", side_name(status.side_to_move), if status.in_check { " · check" } else { "" }) },
            outcome,
            checked,
            details,
            self.can_undo(), self.timeline.can_redo(),
            last_action_view(self.timeline.history()),
        ))
    }

    fn interaction_view(&self) -> InteractionView {
        let Some(driver) = self.interaction.as_ref() else { return InteractionView::default(); };
        interaction_view(
            driver.interaction().generation,
            ChessInteractionRules::selected_entity(driver.draft()),
            ChessInteractionRules::pending_target(driver.draft()),
            None,
            &driver.interaction().choices,
        )
    }

    fn history_view(&self) -> Result<Vec<HistoryTurnView>, String> {
        let mut prefix = nydra_core::History::default();
        let mut turns = Vec::new();
        for (index, turn) in self.timeline.history().turns().iter().enumerate() {
            if turn.synthetic {
                prefix = prefix.with_appended(turn.clone()).map_err(string_error)?;
                continue;
            }
            let side = ChessSide::from_player(turn.actor).ok_or_else(|| "history actor is not a chess side".to_owned())?;
            let san = self.rules.san_for_turn(turn, &prefix).map_err(string_error)?;
            turns.push(HistoryTurnView {
                index: u32::try_from(index).unwrap_or(u32::MAX),
                actor: turn.actor.get(),
                turn_number: self.rules.fullmove_number(&turn.before),
                actor_label: side_name(side).to_owned(),
                notation: san,
                actions: action_views(turn),
            });
            prefix = prefix.with_appended(turn.clone()).map_err(string_error)?;
        }
        Ok(turns)
    }
}

impl CheckersRuntime {
    fn standard() -> Result<Self, String> {
        let timeline = GameTimeline::new(standard_checkers_state()).map_err(string_error)?;
        let mut runtime = Self { timeline, interaction: None };
        runtime.rebuild_interaction()?;
        Ok(runtime)
    }
    fn rebuild_interaction(&mut self) -> Result<(), String> {
        let registry = checkers_registry();
        if registry.outcome(RuleContext::from_state(self.timeline.current(), Some(self.timeline.history()))).map_err(string_error)?.is_some() {
            self.interaction = None; return Ok(());
        }
        let actor = active_actor(self.timeline.current(), "checkers")?;
        let turn = self.timeline.begin_turn(actor).map_err(string_error)?;
        self.interaction = Some(InteractionDriver::new(CheckersInteractionRules, turn).map_err(string_error)?);
        Ok(())
    }
    fn choose(&mut self, id: ChoiceId) -> Result<(bool, Vec<StepRecord>), String> { let result = choose_driver(&mut self.interaction, &mut self.timeline, id)?; if result.0 { self.rebuild_interaction()?; } Ok(result) }
    fn undo(&mut self) -> Result<(), String> { self.timeline.undo().ok_or_else(|| "there is no turn to undo".to_owned())?; self.rebuild_interaction() }
    fn redo(&mut self) -> Result<(), String> { self.timeline.redo().ok_or_else(|| "there is no turn to redo".to_owned())?; self.rebuild_interaction() }
    fn game_view(&self) -> Result<GameView, String> {
        let state = visible_state(&self.timeline, self.interaction.as_ref());
        let registry = checkers_registry();
        let entities = presentation_entities(&registry, state, self.timeline.history())?;
        let outcome = registry.outcome(RuleContext::from_state(state, Some(self.timeline.history()))).map_err(string_error)?;
        let active = state.turn.active_players.first().copied();
        let status = match active { Some(CHECKERS_WHITE) => "White to move", Some(CHECKERS_BLACK) => "Black to move", _ => "Checkers" };
        Ok(GameView::new("checkers", "Checkers", "checkerboard", state, entities, outcome.as_ref().map(outcome_text).unwrap_or_else(|| status.to_owned()), outcome, None, StateMap::new(), self.timeline.can_undo(), self.timeline.can_redo(), last_action_view(self.timeline.history())))
    }
    fn interaction_view(&self) -> InteractionView {
        let Some(driver) = self.interaction.as_ref() else { return InteractionView::default(); };
        interaction_view(driver.interaction().generation, CheckersInteractionRules::selected_entity(driver.draft()), None, None, &driver.interaction().choices)
    }
}

impl GoRuntime {
    fn standard() -> Result<Self, String> {
        let timeline = GameTimeline::new(empty_go_state(9)).map_err(string_error)?;
        let mut runtime = Self { timeline, interaction: None };
        runtime.rebuild_interaction()?;
        Ok(runtime)
    }
    fn rebuild_interaction(&mut self) -> Result<(), String> {
        let registry = go_registry();
        if registry.outcome(RuleContext::from_state(self.timeline.current(), Some(self.timeline.history()))).map_err(string_error)?.is_some() {
            self.interaction = None; return Ok(());
        }
        let actor = active_actor(self.timeline.current(), "go")?;
        let turn = self.timeline.begin_turn(actor).map_err(string_error)?;
        self.interaction = Some(InteractionDriver::new(GoInteractionRules::with_history(self.timeline.history()), turn).map_err(string_error)?);
        Ok(())
    }
    fn choose(&mut self, id: ChoiceId) -> Result<(bool, Vec<StepRecord>), String> { let result = choose_driver(&mut self.interaction, &mut self.timeline, id)?; if result.0 { self.rebuild_interaction()?; } Ok(result) }
    fn undo(&mut self) -> Result<(), String> { self.timeline.undo().ok_or_else(|| "there is no turn to undo".to_owned())?; self.rebuild_interaction() }
    fn redo(&mut self) -> Result<(), String> { self.timeline.redo().ok_or_else(|| "there is no turn to redo".to_owned())?; self.rebuild_interaction() }
    fn game_view(&self) -> Result<GameView, String> {
        let state = visible_state(&self.timeline, self.interaction.as_ref());
        let registry = go_registry();
        let entities = presentation_entities(&registry, state, self.timeline.history())?;
        let outcome = registry.outcome(RuleContext::from_state(state, Some(self.timeline.history()))).map_err(string_error)?;
        let active = state.turn.active_players.first().copied();
        let status = match active { Some(GO_BLACK) => "Black to place", Some(GO_WHITE) => "White to place", _ => "Go" };
        Ok(GameView::new("go", "Go 9×9", "go", state, entities, outcome.as_ref().map(outcome_text).unwrap_or_else(|| status.to_owned()), outcome, None, StateMap::new(), self.timeline.can_undo(), self.timeline.can_redo(), last_action_view(self.timeline.history())))
    }
    fn interaction_view(&self) -> InteractionView {
        let Some(driver) = self.interaction.as_ref() else { return InteractionView::default(); };
        interaction_view(driver.interaction().generation, None, None, None, &driver.interaction().choices)
    }
}

impl RiftRuntime {
    fn standard() -> Result<Self, String> {
        let timeline = GameTimeline::new(standard_rift_state()).map_err(string_error)?;
        let mut runtime = Self { timeline, interaction: None };
        runtime.rebuild_interaction()?;
        Ok(runtime)
    }
    fn rebuild_interaction(&mut self) -> Result<(), String> {
        let registry = rift_registry();
        if registry.outcome(RuleContext::from_state(self.timeline.current(), Some(self.timeline.history()))).map_err(string_error)?.is_some() {
            self.interaction = None; return Ok(());
        }
        let actor = active_actor(self.timeline.current(), "rift")?;
        let turn = self.timeline.begin_turn(actor).map_err(string_error)?;
        self.interaction = Some(InteractionDriver::new(RiftInteractionRules::with_history(self.timeline.history()), turn).map_err(string_error)?);
        Ok(())
    }
    fn choose(&mut self, id: ChoiceId) -> Result<(bool, Vec<StepRecord>), String> { let result = choose_driver(&mut self.interaction, &mut self.timeline, id)?; if result.0 { self.rebuild_interaction()?; } Ok(result) }
    fn undo(&mut self) -> Result<(), String> { self.timeline.undo().ok_or_else(|| "there is no turn to undo".to_owned())?; self.rebuild_interaction() }
    fn redo(&mut self) -> Result<(), String> { self.timeline.redo().ok_or_else(|| "there is no turn to redo".to_owned())?; self.rebuild_interaction() }
    fn game_view(&self) -> Result<GameView, String> {
        let state = visible_state(&self.timeline, self.interaction.as_ref());
        let registry = rift_registry();
        let entities = presentation_entities(&registry, state, self.timeline.history())?;
        let outcome = registry.outcome(RuleContext::from_state(state, Some(self.timeline.history()))).map_err(string_error)?;
        let active = state.turn.active_players.first().map(|player| player.get());
        let status = active.map(|player| format!("Player {player} to act")).unwrap_or_else(|| "Rift".to_owned());
        Ok(GameView::new("rift", "Rift", "checkerboard", state, entities, outcome.as_ref().map(outcome_text).unwrap_or(status), outcome, None, StateMap::new(), self.timeline.can_undo(), self.timeline.can_redo(), last_action_view(self.timeline.history())))
    }
    fn interaction_view(&self) -> InteractionView {
        let Some(driver) = self.interaction.as_ref() else { return InteractionView::default(); };
        let selected = RiftInteractionRules::selected_entity(driver.draft());
        let target = RiftInteractionRules::pending_target(driver.draft()).and_then(|entity| driver.turn().working.entity(entity).ok()).map(|entity| entity.position);
        let ability = RiftInteractionRules::active_ability(driver.draft()).map(|ability| ability.get());
        interaction_view(driver.interaction().generation, selected, target, ability, &driver.interaction().choices)
    }
}

fn choose_driver<R: InteractionRules>(interaction: &mut Option<InteractionDriver<R>>, timeline: &mut GameTimeline, id: ChoiceId) -> Result<(bool, Vec<StepRecord>), String> {
    let mut driver = interaction.take().ok_or_else(|| "interaction session is unavailable".to_owned())?;
    let previous_step_count = driver.turn().steps.len();
    let update = match driver.choose(id) {
        Ok(update) => update,
        Err(error) => { *interaction = Some(driver); return Err(error.to_string()); }
    };
    let new_steps = driver.turn().steps[previous_step_count..].to_vec();
    match update {
        InteractionUpdate::Continued(_) => { *interaction = Some(driver); Ok((false, new_steps)) }
        InteractionUpdate::Finished => { let turn = driver.into_turn().map_err(string_error)?; timeline.commit_turn(turn).map_err(string_error)?; Ok((true, new_steps)) }
    }
}

fn reset_draft<R: InteractionRules>(interaction: &mut Option<InteractionDriver<R>>) -> Result<(), String> {
    let driver = interaction.as_mut().ok_or_else(|| "interaction session is unavailable".to_owned())?;
    if !driver.turn().steps.is_empty() {
        return Ok(());
    }
    driver.reset_draft().map_err(string_error)?;
    Ok(())
}

fn visible_state<'a, R: InteractionRules>(timeline: &'a GameTimeline, interaction: Option<&'a InteractionDriver<R>>) -> &'a GameState {
    interaction.map(|driver| &driver.turn().working).unwrap_or_else(|| timeline.current())
}

fn active_actor(state: &GameState, ruleset: &str) -> Result<nydra_core::PlayerId, String> {
    state.turn.active_players.first().copied().ok_or_else(|| format!("{ruleset} state has no active player"))
}

fn parse_choice_id(choice_id: &str) -> Result<ChoiceId, JsValue> {
    let raw = choice_id.parse::<u64>().map_err(|_| js_error("choice id is not a valid integer"))?;
    Ok(ChoiceId::new(raw))
}

fn presentation_entities(registry: &nydra_core::RuleRegistry, state: &GameState, history: &nydra_core::History) -> Result<Vec<EntityView>, String> {
    state.entities.values().map(|entity| {
        let presentation = registry.presentation(RuleContext::from_state(state, Some(history)), entity.id).map_err(string_error)?;
        Ok(entity_view(entity, presentation))
    }).collect()
}

fn entity_view(entity: &EntityState, presentation: nydra_core::EntityPresentation) -> EntityView {
    EntityView {
        id: entity.id.get(), entity_type: entity.entity_type.get(), owner: entity.owner.get(), controller: entity.controller.get(),
        position: entity.position.into(), asset_key: presentation.asset_key, label: presentation.label,
        presentation_data: presentation.data, state: entity.state.clone(),
    }
}

fn interaction_view(generation: u64, selected: Option<nydra_core::EntityId>, pending_target: Option<Position>, active_ability: Option<u32>, choices: &[Choice]) -> InteractionView {
    InteractionView {
        generation: generation.to_string(),
        selected_entity: selected.map(|entity| entity.get()),
        pending_target: pending_target.map(Into::into),
        active_ability,
        choices: choices.iter().map(ChoiceView::from).collect(),
    }
}

fn last_action_view(history: &nydra_core::History) -> Option<MoveEndpointsView> {
    let turn = history.turns().iter().rev().find(|turn| !turn.synthetic)?;
    for step in turn.steps.iter().rev() {
        for change in step.delta.changes.iter().rev() {
            match change {
                StateChange::EntityMoved { from, to, .. } => return Some(MoveEndpointsView { from: (*from).into(), to: (*to).into() }),
                StateChange::EntityAdded { entity } => return Some(MoveEndpointsView { from: entity.position.into(), to: entity.position.into() }),
                _ => {}
            }
        }
    }
    None
}

fn generic_history_view(history: &nydra_core::History, ruleset: &str) -> Vec<HistoryTurnView> {
    history.turns().iter().enumerate().filter(|(_, turn)| !turn.synthetic).map(|(index, turn)| {
        let notation = turn.steps.iter().map(|step| step.action.kind.strip_prefix(&format!("{ruleset}.")).unwrap_or(&step.action.kind)).collect::<Vec<_>>().join(" → ");
        HistoryTurnView { index: u32::try_from(index).unwrap_or(u32::MAX), actor: turn.actor.get(), turn_number: u32::try_from(index + 1).unwrap_or(u32::MAX), actor_label: format!("P{}", turn.actor.get()), notation, actions: action_views(turn) }
    }).collect()
}

fn action_views(turn: &nydra_core::TurnRecord) -> Vec<ActionView> {
    turn.steps.iter().map(|step| ActionView { kind: step.action.kind.clone(), data: step.action.data.clone() }).collect()
}

fn chess_outcome_to_generic(outcome: &ChessOutcome) -> GameOutcome {
    match outcome {
        ChessOutcome::Checkmate { winner, loser } => GameOutcome::new("chess.checkmate").with_winner(*winner).with_loser(*loser),
        ChessOutcome::Stalemate => GameOutcome::new("chess.stalemate"),
        ChessOutcome::Resignation { winner, loser } => GameOutcome::new("chess.resignation").with_winner(*winner).with_loser(*loser),
        ChessOutcome::DrawAgreement => GameOutcome::new("chess.draw_agreement"),
        ChessOutcome::ThreefoldRepetition => GameOutcome::new("chess.threefold_repetition"),
        ChessOutcome::FivefoldRepetition => GameOutcome::new("chess.fivefold_repetition"),
        ChessOutcome::FiftyMoveRule => GameOutcome::new("chess.fifty_move_rule"),
        ChessOutcome::SeventyFiveMoveRule => GameOutcome::new("chess.seventy_five_move_rule"),
        ChessOutcome::DeadPosition => GameOutcome::new("chess.dead_position"),
    }
}

fn side_name(side: ChessSide) -> &'static str { match side { ChessSide::White => "white", ChessSide::Black => "black" } }

fn outcome_text(outcome: &GameOutcome) -> String {
    outcome
        .key
        .rsplit_once('.')
        .map(|(_, key)| key)
        .unwrap_or(&outcome.key)
        .replace('_', " ")
}

#[derive(Clone, Debug, Serialize)]
pub struct PositionView { pub x: u16, pub y: u16 }
impl From<Position> for PositionView { fn from(value: Position) -> Self { Self { x: value.x, y: value.y } } }

#[derive(Clone, Debug, Serialize)]
pub struct EntityView {
    pub id: u32, pub entity_type: u32, pub owner: u32, pub controller: u32, pub position: PositionView,
    pub asset_key: String, pub label: Option<String>, pub presentation_data: StateMap, pub state: StateMap,
}

#[derive(Clone, Debug, Serialize)]
pub struct MoveEndpointsView { pub from: PositionView, pub to: PositionView }

#[derive(Clone, Debug, Serialize)]
pub struct OutcomeView {
    pub key: String, pub winners: Vec<u32>, pub losers: Vec<u32>, pub winning_teams: Vec<u32>, pub losing_teams: Vec<u32>, pub data: StateMap,
}
impl From<&GameOutcome> for OutcomeView {
    fn from(value: &GameOutcome) -> Self { Self { key: value.key.clone(), winners: value.winners.iter().map(|id| id.get()).collect(), losers: value.losers.iter().map(|id| id.get()).collect(), winning_teams: value.winning_teams.iter().map(|id| id.get()).collect(), losing_teams: value.losing_teams.iter().map(|id| id.get()).collect(), data: value.data.clone() } }
}

#[derive(Clone, Debug, Serialize)]
pub struct StatusView { pub text: String, pub outcome: Option<OutcomeView>, pub checked_position: Option<PositionView>, pub details: StateMap }

#[derive(Clone, Debug, Serialize)]
pub struct GameView {
    pub ruleset: String, pub title: String, pub board_style: String, pub width: u16, pub height: u16,
    pub entities: Vec<EntityView>, pub last_move: Option<MoveEndpointsView>, pub active_players: Vec<u32>,
    pub status: StatusView, pub can_undo: bool, pub can_redo: bool,
}
impl GameView {
    #[allow(clippy::too_many_arguments)]
    fn new(ruleset: &str, title: &str, board_style: &str, state: &GameState, entities: Vec<EntityView>, text: String, outcome: Option<GameOutcome>, checked: Option<PositionView>, details: StateMap, can_undo: bool, can_redo: bool, last_move: Option<MoveEndpointsView>) -> Self {
        Self { ruleset: ruleset.into(), title: title.into(), board_style: board_style.into(), width: state.board.width(), height: state.board.height(), entities, last_move, active_players: state.turn.active_players.iter().map(|id| id.get()).collect(), status: StatusView { text, outcome: outcome.as_ref().map(OutcomeView::from), checked_position: checked, details }, can_undo, can_redo }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct InteractionView { pub generation: String, pub selected_entity: Option<u32>, pub pending_target: Option<PositionView>, pub active_ability: Option<u32>, pub choices: Vec<ChoiceView> }

#[derive(Clone, Debug, Serialize)]
pub struct ChoiceView {
    pub id: String, pub kind: String, pub entity: Option<u32>, pub position: Option<PositionView>, pub ability: Option<u32>, pub option_key: Option<String>,
    pub label: Option<String>, pub actor: Option<u32>, pub capture: Option<u32>, pub move_kind: Option<String>, pub target_position: Option<PositionView>, pub option_entity_type: Option<u32>, pub asset_key: Option<String>, pub data: StateMap,
}
impl From<&Choice> for ChoiceView {
    fn from(choice: &Choice) -> Self {
        let mut view = Self { id: choice.id.get().to_string(), kind: String::new(), entity: None, position: None, ability: None, option_key: None, label: choice.label.clone(), actor: state_u32(&choice.data, "actor"), capture: state_u32(&choice.data, "capture"), move_kind: choice.data.get("move_kind").and_then(nydra_core::StateValue::as_str).map(str::to_owned), target_position: match (state_u16(&choice.data, "target_x"), state_u16(&choice.data, "target_y")) { (Some(x), Some(y)) => Some(PositionView { x, y }), _ => None }, option_entity_type: state_u32(&choice.data, "entity_type"), asset_key: choice.asset_key.clone(), data: choice.data.clone() };
        match &choice.kind {
            ChoiceKind::SelectEntity { entity } => { view.kind = "select_entity".into(); view.entity = Some(entity.get()); }
            ChoiceKind::SelectPosition { position } => { view.kind = "select_position".into(); view.position = Some((*position).into()); }
            ChoiceKind::SelectAbility { ability } => { view.kind = "select_ability".into(); view.ability = Some(ability.get()); }
            ChoiceKind::SelectOption { key } => { view.kind = "select_option".into(); view.option_key = Some(key.clone()); }
            ChoiceKind::FinishTurn => { view.kind = "finish_turn".into(); }
        }
        view
    }
}
fn state_u32(data: &StateMap, key: &str) -> Option<u32> { data.get(key).and_then(nydra_core::StateValue::as_u64).and_then(|value| u32::try_from(value).ok()) }
fn state_u16(data: &StateMap, key: &str) -> Option<u16> { data.get(key).and_then(nydra_core::StateValue::as_u64).and_then(|value| u16::try_from(value).ok()) }

#[derive(Clone, Debug, Serialize)]
pub struct EntitySnapshotView { pub id: u32, pub entity_type: u32, pub owner: u32, pub controller: u32, pub position: PositionView, pub state: StateMap }
impl From<&EntityState> for EntitySnapshotView { fn from(entity: &EntityState) -> Self { Self { id: entity.id.get(), entity_type: entity.entity_type.get(), owner: entity.owner.get(), controller: entity.controller.get(), position: entity.position.into(), state: entity.state.clone() } } }
#[derive(Clone, Debug, Serialize)]
pub struct PlayerSnapshotView { pub id: u32, pub team: Option<u32>, pub state: StateMap }
impl From<&PlayerState> for PlayerSnapshotView { fn from(player: &PlayerState) -> Self { Self { id: player.id.get(), team: player.team.map(|team| team.get()), state: player.state.clone() } } }
#[derive(Clone, Debug, Serialize)]
pub struct TeamSnapshotView { pub id: u32, pub state: StateMap }
impl From<&TeamState> for TeamSnapshotView { fn from(team: &TeamState) -> Self { Self { id: team.id.get(), state: team.state.clone() } } }
#[derive(Clone, Debug, Serialize)]
pub struct TurnSnapshotView { pub active_players: Vec<u32>, pub state: StateMap }
impl From<&TurnState> for TurnSnapshotView { fn from(turn: &TurnState) -> Self { Self { active_players: turn.active_players.iter().map(|player| player.get()).collect(), state: turn.state.clone() } } }

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChangeView {
    EntityAdded { entity: EntitySnapshotView }, EntityRemoved { entity: EntitySnapshotView }, EntityMoved { entity: u32, from: PositionView, to: PositionView },
    EntityTypeChanged { entity: u32, from: u32, to: u32 }, EntityOwnerChanged { entity: u32, from: u32, to: u32 }, EntityControllerChanged { entity: u32, from: u32, to: u32 },
    EntityStateChanged { entity: u32, before: StateMap, after: StateMap }, PlayerAdded { player: PlayerSnapshotView }, PlayerRemoved { player: PlayerSnapshotView },
    PlayerChanged { player: u32, before: StateMap, after: StateMap, before_team: Option<u32>, after_team: Option<u32> }, TeamAdded { team: TeamSnapshotView }, TeamRemoved { team: TeamSnapshotView },
    TeamChanged { team: u32, before: StateMap, after: StateMap }, TurnChanged { before: TurnSnapshotView, after: TurnSnapshotView }, RulesetStateChanged { before: StateMap, after: StateMap },
}
impl From<&StateChange> for ChangeView {
    fn from(change: &StateChange) -> Self { match change {
        StateChange::EntityAdded { entity } => Self::EntityAdded { entity: entity.into() }, StateChange::EntityRemoved { entity } => Self::EntityRemoved { entity: entity.into() },
        StateChange::EntityMoved { entity, from, to } => Self::EntityMoved { entity: entity.get(), from: (*from).into(), to: (*to).into() },
        StateChange::EntityTypeChanged { entity, from, to } => Self::EntityTypeChanged { entity: entity.get(), from: from.get(), to: to.get() },
        StateChange::EntityOwnerChanged { entity, from, to } => Self::EntityOwnerChanged { entity: entity.get(), from: from.get(), to: to.get() },
        StateChange::EntityControllerChanged { entity, from, to } => Self::EntityControllerChanged { entity: entity.get(), from: from.get(), to: to.get() },
        StateChange::EntityStateChanged { entity, before, after } => Self::EntityStateChanged { entity: entity.get(), before: before.clone(), after: after.clone() },
        StateChange::PlayerAdded { player } => Self::PlayerAdded { player: player.into() }, StateChange::PlayerRemoved { player } => Self::PlayerRemoved { player: player.into() },
        StateChange::PlayerChanged { player, before, after, before_team, after_team } => Self::PlayerChanged { player: player.get(), before: before.clone(), after: after.clone(), before_team: before_team.map(|team| team.get()), after_team: after_team.map(|team| team.get()) },
        StateChange::TeamAdded { team } => Self::TeamAdded { team: team.into() }, StateChange::TeamRemoved { team } => Self::TeamRemoved { team: team.into() },
        StateChange::TeamChanged { team, before, after } => Self::TeamChanged { team: team.get(), before: before.clone(), after: after.clone() },
        StateChange::TurnChanged { before, after } => Self::TurnChanged { before: before.into(), after: after.into() }, StateChange::RulesetStateChanged { before, after } => Self::RulesetStateChanged { before: before.clone(), after: after.clone() },
    } }
}

#[derive(Clone, Debug, Serialize)]
pub struct PresentationView { pub kind: String, pub data: StateMap }
impl From<&PresentationCue> for PresentationView { fn from(cue: &PresentationCue) -> Self { Self { kind: cue.kind.clone(), data: cue.data.clone() } } }
#[derive(Clone, Debug, Serialize)]
pub struct TransitionView { pub committed: bool, pub game: GameView, pub interaction: InteractionView, pub changes: Vec<ChangeView>, pub presentation: Vec<PresentationView> }
#[derive(Clone, Debug, Serialize)]
pub struct ActionView { pub kind: String, pub data: StateMap }
#[derive(Clone, Debug, Serialize)]
pub struct HistoryTurnView { pub index: u32, pub actor: u32, pub turn_number: u32, pub actor_label: String, pub notation: String, pub actions: Vec<ActionView> }

fn changes_from_steps(steps: &[StepRecord]) -> Vec<ChangeView> { steps.iter().flat_map(|step| step.delta.changes.iter().map(ChangeView::from)).collect() }
fn presentation_from_steps(steps: &[StepRecord]) -> Vec<PresentationView> { steps.iter().flat_map(|step| step.presentation.iter().map(PresentationView::from)).collect() }
fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsValue> { serde_wasm_bindgen::to_value(value).map_err(js_error) }
fn js_error(error: impl ToString) -> JsValue { JsValue::from_str(&error.to_string()) }
fn string_error(error: impl ToString) -> String { error.to_string() }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_builtin_ruleset_exposes_a_playable_runtime() {
        for ruleset in ["chess", "checkers", "go", "rift"] {
            let game = GameHandle::new_ruleset(ruleset).unwrap();
            assert_eq!(game.game_view().unwrap().ruleset, ruleset);
            assert!(!game.interaction_view().choices.is_empty());
        }
    }

    #[test]
    fn imported_chess_history_keeps_synthetic_turn_hidden_from_undo() {
        let runtime = ChessRuntime::from_fen("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 17").unwrap();
        assert!(!runtime.can_undo());
        assert_eq!(runtime.timeline.history().len(), 1);
        assert!(runtime.timeline.history().previous_turn().unwrap().synthetic);
    }

    #[test]
    fn pgn_runtime_rebuilds_authoritative_chess_timeline() {
        let runtime = ChessRuntime::from_pgn("1. e4 e5 2. Nf3 Nc6 *").unwrap();
        assert_eq!(runtime.timeline.history().turns().iter().filter(|turn| !turn.synthetic).count(), 4);
        assert_eq!(runtime.initial_fen, STANDARD_FEN);
    }
}
