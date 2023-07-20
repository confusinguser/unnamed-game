use bevy::prelude::{error, warn};

use crate::cell::{Board, CellCoordinates};
use crate::gamemanager::{self, Game};
use crate::units::*;
use crate::utils::RadialDirection;

pub(crate) struct GameMove {
    pub(crate) from: CellCoordinates,
    pub(crate) to: CellCoordinates,
}

pub(crate) fn get_unit_moves(unit: &Unit, board: &Board, units: &Units) -> Vec<CellCoordinates> {
    match unit.unit_type {
        UnitType::Rook => rook_movement(unit.coords, board, units),
        UnitType::Bishop => bishop_movement(unit.coords, board, units),
        UnitType::King => king_movement(unit.coords, board, units),
        UnitType::Pawn(direction, has_moved) => {
            pawn_movement(unit.coords, board, units, direction, has_moved)
        }
        UnitType::Knight => knight_movement(unit.coords, board, units),
        UnitType::Queen => queen_movement(unit.coords, board, units),
    }
}

fn king_movement(
    unit_coords: CellCoordinates,
    board: &Board,
    units: &Units,
) -> Vec<CellCoordinates> {
    let mut out = parts::get_straight(unit_coords, 1, 0, board.cube_side_length, units);
    out.append(&mut parts::get_diagonals(
        unit_coords,
        1,
        0,
        board.cube_side_length,
        units,
    ));
    out
}

fn bishop_movement(
    unit_coords: CellCoordinates,
    board: &Board,
    units: &Units,
) -> Vec<CellCoordinates> {
    parts::get_diagonals(unit_coords, u32::MAX, 1, board.cube_side_length, units)
}

fn rook_movement(
    unit_coords: CellCoordinates,
    board: &Board,
    units: &Units,
) -> Vec<CellCoordinates> {
    parts::get_straight(unit_coords, u32::MAX, 1, board.cube_side_length, units)
}

fn queen_movement(
    unit_coords: CellCoordinates,
    board: &Board,
    units: &Units,
) -> Vec<CellCoordinates> {
    let mut out = parts::get_straight(unit_coords, u32::MAX, 1, board.cube_side_length, units);
    out.append(&mut parts::get_diagonals(
        unit_coords,
        u32::MAX,
        1,
        board.cube_side_length,
        units,
    ));
    out
}

fn pawn_movement(
    unit_coords: CellCoordinates,
    board: &Board,
    units: &Units,
    direction: RadialDirection,
    has_moved: bool,
) -> Vec<CellCoordinates> {
    let mut output = Vec::new();
    let Some(one_step) = unit_coords.get_cell_in_radial_direction(direction, board.cube_side_length) else {
        error!("Pawn has a direction that can't be walked in");
        return output;
    };
    output.push(one_step.0);
    if has_moved {
        let two_steps = one_step
            .0
            .get_cell_in_radial_direction(direction, board.cube_side_length)
            .unwrap();
        output.push(two_steps.0);
    }
    output
}

fn knight_movement(
    unit_coords: CellCoordinates,
    board: &Board,
    units: &Units,
) -> Vec<CellCoordinates> {
    parts::get_knight_moves(unit_coords, 1, board.cube_side_length, units)
}

pub(crate) fn make_move(game_move: GameMove, game: &mut Game) -> bool {
    let Some(unit) = game.units.get_unit_mut(game_move.from) else {return false};
    if unit.team != game.turn {
        return false;
    }

    unit.move_unit_to(game_move.to);
    let Some(entity) = unit.entity else {warn!("Unit entity was None");return false;};
    game.units_to_move.push((entity, game_move.to));
    true
}

/// Parts to create full movement patterns with
mod parts {
    use std::collections::VecDeque;

    use crate::cell::{Board, CellCoordinates};
    use crate::units::Units;
    use crate::utils::{CartesianDirection, RadialDirection};

    pub(crate) fn get_straight(
        coords: CellCoordinates,
        max_dist: u32,
        max_edge_crossings: u32,
        cube_side_length: u32,
        units: &Units,
    ) -> Vec<CellCoordinates> {
        let mut output = Vec::new();
        for direction in RadialDirection::directions() {
            let mut latest_cell = coords;
            let mut dist = 0;
            let mut edge_crossings = 0;
            loop {
                let next_cell =
                    latest_cell.get_cell_in_radial_direction(direction, cube_side_length);
                if next_cell.is_none() {
                    break;
                }
                let next_cell = next_cell.unwrap();

                if output.iter().any(|cell| *cell == next_cell.0) {
                    break;
                }

                dist += 1;
                if next_cell.1 {
                    edge_crossings += 1;
                }

                if dist > max_dist || edge_crossings > max_edge_crossings {
                    break;
                }

                output.push(next_cell.0);

                if units.is_unit_at(next_cell.0) {
                    break;
                }

                latest_cell = next_cell.0;
            }
        }
        output
    }

    fn all_cells_on_side(coords: CellCoordinates, board: &Board) -> Vec<CellCoordinates> {
        let mut output = Vec::new();
        for cell in board.get_all_cells() {
            if cell.coords.normal_direction() == coords.normal_direction() {
                output.push(cell.coords);
            }
        }
        output
    }
    pub(crate) fn get_cells_max_dist(
        coords: CellCoordinates,
        max_dist: u32,
        only_walkable: bool,
        board: &Board,
    ) -> Vec<CellCoordinates> {
        let mut output = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back((coords, 0));
        while !queue.is_empty() {
            let entry = queue.pop_front().unwrap();
            if entry.1 > max_dist {
                break;
            }
            output.push(entry.0);

            for adjacent in entry.0.get_adjacent(board.cube_side_length) {
                if only_walkable && !board.get_cell(adjacent).unwrap().cell_type.walkable() {
                    continue;
                }
                if !output.iter().any(|cell| *cell == entry.0) {
                    continue;
                }
                queue.push_back((adjacent, entry.1 + 1));
            }
        }
        output
    }

    // TODO: Use two RadialDirection to represent a radial diagonal
    pub(crate) fn get_diagonals(
        coords: CellCoordinates,
        max_dist: u32,
        max_edge_crossings: u32,
        cube_side_length: u32,
        units: &Units,
    ) -> Vec<CellCoordinates> {
        let mut output = Vec::new();
        for diagonal in CartesianDirection::diagonals() {
            let mut latest_cell = coords;
            let mut dist = 0;
            let mut edge_crossings = 0;
            loop {
                let Some(next_cell) = latest_cell.get_diagonal(diagonal, cube_side_length) else {break;};

                if output.iter().any(|cell| *cell == next_cell.0) {
                    break;
                }

                dist += 1;
                if next_cell.1 {
                    edge_crossings += 1;
                }

                if dist > max_dist || edge_crossings > max_edge_crossings {
                    break;
                }

                output.push(next_cell.0);

                if units.is_unit_at(next_cell.0) {
                    break;
                }

                latest_cell = next_cell.0;
            }
        }
        output
    }

    pub(crate) fn get_knight_moves(
        coords: CellCoordinates,
        max_edge_crossings: u32,
        cube_side_length: u32,
        units: &Units,
    ) -> Vec<CellCoordinates> {
        let mut output = Vec::new();
        for radial_direction in RadialDirection::directions() {
            let Some(mut forward_two) = coords.get_cell_in_radial_direction(radial_direction, cube_side_length) else {continue;};
            let mut edge_crossings = 0;

            if forward_two.1 {
                edge_crossings += 1;
            }
            // If we didn't get a None the first time, we are guaranteed to still be on the same
            // ring after the first transformation => Safe to unwrap
            forward_two = forward_two
                .0
                .get_cell_in_radial_direction(radial_direction, cube_side_length)
                .unwrap();

            if forward_two.1 {
                edge_crossings += 1;
            }

            // Gets the left/right axis
            let left_right_axis = radial_direction
                .to_cartesian_direction(coords.normal_direction())
                .unwrap()
                .get_perpendicular_axis(coords.normal_direction())
                .unwrap();

            if edge_crossings > max_edge_crossings {
                continue;
            }

            for direction_2 in [left_right_axis, left_right_axis.negate()] {
                let endpoint = forward_two
                    .0
                    .get_cell_in_direction(direction_2, cube_side_length)
                    .unwrap();
                if endpoint.1 && edge_crossings + 1 > max_edge_crossings {
                    // Will go over the max if add this one
                    continue;
                }
                output.push(endpoint.0);
            }
        }

        output
    }
}
