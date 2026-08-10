use crate::{
    Bishop, ChessError, ChessPieceContext, ChessPieceRule, ChessSide, King, Knight, Pawn,
    PseudoMove, Queen, Rook, BISHOP, KING, KNIGHT, PAWN, QUEEN, ROOK,
};
use glorichess_core::{
    EntityId, EntityState, EntityTypeId, GameState, PlayerId, PlayerState, Position, TeamId,
    TeamState,
};
use std::collections::BTreeMap;

pub const WHITE_PLAYER: PlayerId = PlayerId::new(1);
pub const BLACK_PLAYER: PlayerId = PlayerId::new(2);
pub const WHITE_TEAM: TeamId = TeamId::new(1);
pub const BLACK_TEAM: TeamId = TeamId::new(2);

impl ChessSide {
    pub const fn player(self) -> PlayerId {
        match self {
            Self::White => WHITE_PLAYER,
            Self::Black => BLACK_PLAYER,
        }
    }

    pub const fn team(self) -> TeamId {
        match self {
            Self::White => WHITE_TEAM,
            Self::Black => BLACK_TEAM,
        }
    }

    pub const fn from_player(player: PlayerId) -> Option<Self> {
        if player.get() == WHITE_PLAYER.get() {
            Some(Self::White)
        } else if player.get() == BLACK_PLAYER.get() {
            Some(Self::Black)
        } else {
            None
        }
    }

    pub const fn forward(self) -> i16 {
        match self {
            Self::White => 1,
            Self::Black => -1,
        }
    }

    pub const fn pawn_start_rank(self) -> u16 {
        match self {
            Self::White => 1,
            Self::Black => 6,
        }
    }
}

pub fn empty_chess_state() -> Result<GameState, ChessError> {
    let mut state = GameState::new(8, 8)?;
    state.add_team(TeamState::new(WHITE_TEAM))?;
    state.add_team(TeamState::new(BLACK_TEAM))?;
    state.add_player(PlayerState::new(WHITE_PLAYER).with_team(WHITE_TEAM))?;
    state.add_player(PlayerState::new(BLACK_PLAYER).with_team(BLACK_TEAM))?;
    state.set_active_players(vec![WHITE_PLAYER])?;
    Ok(state)
}

pub fn standard_chess_state() -> Result<GameState, ChessError> {
    let mut state = empty_chess_state()?;
    let back_rank = [ROOK, KNIGHT, BISHOP, QUEEN, KING, BISHOP, KNIGHT, ROOK];
    let mut next_id = 1_u32;

    for (x, entity_type) in back_rank.iter().copied().enumerate() {
        add_piece(
            &mut state,
            &mut next_id,
            entity_type,
            ChessSide::White,
            Position::new(x as u16, 0),
        )?;
    }
    for x in 0..8_u16 {
        add_piece(
            &mut state,
            &mut next_id,
            PAWN,
            ChessSide::White,
            Position::new(x, 1),
        )?;
    }
    for x in 0..8_u16 {
        add_piece(
            &mut state,
            &mut next_id,
            PAWN,
            ChessSide::Black,
            Position::new(x, 6),
        )?;
    }
    for (x, entity_type) in back_rank.iter().copied().enumerate() {
        add_piece(
            &mut state,
            &mut next_id,
            entity_type,
            ChessSide::Black,
            Position::new(x as u16, 7),
        )?;
    }

    state.validate()?;
    Ok(state)
}

fn add_piece(
    state: &mut GameState,
    next_id: &mut u32,
    entity_type: EntityTypeId,
    side: ChessSide,
    position: Position,
) -> Result<EntityId, ChessError> {
    let id = EntityId::new(*next_id);
    *next_id = (*next_id).saturating_add(1);
    state.add_entity(EntityState::new(id, entity_type, side.player(), position))?;
    Ok(id)
}

pub struct ChessRules {
    pieces: BTreeMap<EntityTypeId, Box<dyn ChessPieceRule>>,
}

impl ChessRules {
    pub fn standard() -> Self {
        let mut rules = Self {
            pieces: BTreeMap::new(),
        };
        rules.register(Pawn).expect("pawn type is unique");
        rules.register(Knight).expect("knight type is unique");
        rules.register(Bishop).expect("bishop type is unique");
        rules.register(Rook).expect("rook type is unique");
        rules.register(Queen).expect("queen type is unique");
        rules.register(King).expect("king type is unique");
        rules
    }

    pub fn register<R>(&mut self, rule: R) -> Result<(), ChessError>
    where
        R: ChessPieceRule + 'static,
    {
        let entity_type = rule.entity_type();
        if self.pieces.contains_key(&entity_type) {
            return Err(ChessError::DuplicatePieceRule(entity_type));
        }
        self.pieces.insert(entity_type, Box::new(rule));
        Ok(())
    }

    pub fn piece_rule(&self, entity_type: EntityTypeId) -> Result<&dyn ChessPieceRule, ChessError> {
        self.pieces
            .get(&entity_type)
            .map(Box::as_ref)
            .ok_or(ChessError::PieceRuleNotFound(entity_type))
    }

    pub fn pseudo_moves(
        &self,
        state: &GameState,
        entity: EntityId,
    ) -> Result<Vec<PseudoMove>, ChessError> {
        let context = ChessPieceContext::new(state, entity)?;
        self.piece_rule(context.entity().entity_type)?
            .pseudo_moves(context)
    }

    pub fn attacks(
        &self,
        state: &GameState,
        entity: EntityId,
    ) -> Result<Vec<Position>, ChessError> {
        let context = ChessPieceContext::new(state, entity)?;
        self.piece_rule(context.entity().entity_type)?.attacks(context)
    }
}

impl Default for ChessRules {
    fn default() -> Self {
        Self::standard()
    }
}
