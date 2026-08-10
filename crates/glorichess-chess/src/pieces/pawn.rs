use crate::{
    offset, standard_presentation, ChessError, ChessPieceContext, ChessPieceKind, ChessPieceRule,
    PseudoMove, PAWN,
};
use glorichess_core::{EntityPresentation, EntityRule, EntityRuleContext, EntityTypeId, Position, RuleError};

pub struct Pawn;

impl EntityRule for Pawn {
    fn presentation(
        &self,
        context: EntityRuleContext<'_>,
    ) -> Result<EntityPresentation, RuleError> {
        standard_presentation(context, ChessPieceKind::Pawn)
    }
}

impl ChessPieceRule for Pawn {
    fn entity_type(&self) -> EntityTypeId {
        PAWN
    }

    fn pseudo_moves(&self, context: ChessPieceContext<'_>) -> Result<Vec<PseudoMove>, ChessError> {
        ensure_type(context, PAWN)?;
        let side = context.side()?;
        let from = context.entity().position;
        let mut moves = Vec::new();

        if let Some(one) = offset(context.state(), from, 0, side.forward()) {
            if context.is_empty(one)? {
                moves.push(context.pseudo_move(one)?);

                let on_start = from.y == side.pawn_start_rank();
                if on_start && context.entity().move_count == 0 {
                    if let Some(two) = offset(context.state(), from, 0, side.forward() * 2) {
                        if context.is_empty(two)? {
                            moves.push(context.pseudo_move(two)?);
                        }
                    }
                }
            }
        }

        for dx in [-1_i16, 1_i16] {
            let Some(target) = offset(context.state(), from, dx, side.forward()) else {
                continue;
            };
            if context.capture_at(target)?.is_some() {
                moves.push(context.pseudo_move(target)?);
            }
        }

        Ok(moves)
    }

    fn attacks(&self, context: ChessPieceContext<'_>) -> Result<Vec<Position>, ChessError> {
        ensure_type(context, PAWN)?;
        let side = context.side()?;
        Ok([-1_i16, 1_i16]
            .into_iter()
            .filter_map(|dx| offset(context.state(), context.entity().position, dx, side.forward()))
            .collect())
    }
}

fn ensure_type(context: ChessPieceContext<'_>, expected: EntityTypeId) -> Result<(), ChessError> {
    let actual = context.entity().entity_type;
    if actual == expected {
        Ok(())
    } else {
        Err(ChessError::WrongPieceType {
            entity: context.entity().id,
            expected,
            actual,
        })
    }
}
