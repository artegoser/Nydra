use crate::{
    piece::{offset, standard_presentation},
    ChessError, ChessPieceContext, ChessPieceKind, ChessPieceRule, PseudoMove, PAWN,
};
use glorichess_core::{
    EntityId, EntityPresentation, EntityRule, EntityRuleContext, EntityTypeId, Position, RuleError,
    StateChange, StateValue, TurnRecord,
};

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
                if on_start {
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

        if let Some(previous) = context.history().and_then(|history| history.previous_turn()) {
            moves.extend(en_passant_moves_from_previous(context, previous)?);
        }

        Ok(moves)
    }

    fn attacks(&self, context: ChessPieceContext<'_>) -> Result<Vec<Position>, ChessError> {
        ensure_type(context, PAWN)?;
        let side = context.side()?;
        Ok([-1_i16, 1_i16]
            .into_iter()
            .filter_map(|dx| {
                offset(
                    context.state(),
                    context.entity().position,
                    dx,
                    side.forward(),
                )
            })
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


pub(crate) fn turn_records_entity_move(
    previous: &TurnRecord,
    entity: EntityId,
    from: Position,
    to: Position,
) -> bool {
    previous.steps.iter().any(|step| {
        let action_actor = step
            .action
            .data
            .get("actor")
            .and_then(StateValue::as_u64);
        step.action.kind == "chess_move"
            && action_actor == Some(u64::from(entity.get()))
            && step.delta.changes.iter().any(|change| {
                matches!(
                    change,
                    StateChange::EntityMoved { entity: moved, from: actual_from, to: actual_to }
                        if *moved == entity && *actual_from == from && *actual_to == to
                )
            })
    })
}

pub(crate) fn en_passant_moves_from_previous(
    context: ChessPieceContext<'_>,
    previous: &TurnRecord,
) -> Result<Vec<PseudoMove>, ChessError> {
    let side = context.side()?;
    let from = context.entity().position;
    let mut moves = Vec::new();

    for dx in [-1_i16, 1_i16] {
        let Some(adjacent_position) = offset(context.state(), from, dx, 0) else {
            continue;
        };
        let Some(adjacent) = context.entity_at(adjacent_position)? else {
            continue;
        };
        if adjacent.entity_type != PAWN || adjacent.owner == context.entity().owner {
            continue;
        }
        let Some(enemy_side) = crate::ChessSide::from_player(adjacent.owner) else {
            continue;
        };
        if previous.actor != adjacent.controller {
            continue;
        }

        let Some(before) = previous.before.entities.get(&adjacent.id) else {
            continue;
        };
        let Some(after) = previous.after.entities.get(&adjacent.id) else {
            continue;
        };
        if before.entity_type != PAWN
            || after.entity_type != PAWN
            || before.position.x != after.position.x
            || before.position.y != enemy_side.pawn_start_rank()
            || after.position != adjacent_position
        {
            continue;
        }
        let expected_y = i32::from(before.position.y) + i32::from(enemy_side.forward() * 2);
        if expected_y != i32::from(after.position.y) {
            continue;
        }

        if !turn_records_entity_move(
            previous,
            adjacent.id,
            before.position,
            after.position,
        ) {
            continue;
        }

        let Some(target) = offset(context.state(), from, dx, side.forward()) else {
            continue;
        };
        if !context.is_empty(target)? {
            continue;
        }
        moves.push(PseudoMove::en_passant(
            context.entity().id,
            from,
            target,
            adjacent.id,
        ));
    }

    Ok(moves)
}
