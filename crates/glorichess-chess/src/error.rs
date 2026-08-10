use glorichess_core::{CoreError, EntityId, EntityTypeId, PlayerId};
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum ChessError {
    #[error(transparent)]
    Core(#[from] CoreError),
    #[error("player {0} is not a standard chess side")]
    UnknownSide(PlayerId),
    #[error("entity type {0} has no registered chess piece rule")]
    PieceRuleNotFound(EntityTypeId),
    #[error("entity {entity} uses type {actual}, expected {expected}")]
    WrongPieceType {
        entity: EntityId,
        expected: EntityTypeId,
        actual: EntityTypeId,
    },
    #[error("chess piece rule {0} is already registered")]
    DuplicatePieceRule(EntityTypeId),
}
