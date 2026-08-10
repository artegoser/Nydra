use crate::{
    leaper_attacks, leaper_moves, standard_presentation, ChessError, ChessPieceContext,
    ChessPieceKind, ChessPieceRule, PseudoMove, KING,
};
use glorichess_core::{EntityPresentation, EntityRule, EntityRuleContext, EntityTypeId, Position, RuleError};

const OFFSETS: [(i16, i16); 8] = [
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
    (0, -1),
    (1, -1),
];

pub struct King;

impl EntityRule for King {
    fn presentation(
        &self,
        context: EntityRuleContext<'_>,
    ) -> Result<EntityPresentation, RuleError> {
        standard_presentation(context, ChessPieceKind::King)
    }
}

impl ChessPieceRule for King {
    fn entity_type(&self) -> EntityTypeId {
        KING
    }

    fn pseudo_moves(&self, context: ChessPieceContext<'_>) -> Result<Vec<PseudoMove>, ChessError> {
        leaper_moves(context, &OFFSETS)
    }

    fn attacks(&self, context: ChessPieceContext<'_>) -> Result<Vec<Position>, ChessError> {
        Ok(leaper_attacks(context, &OFFSETS))
    }
}
