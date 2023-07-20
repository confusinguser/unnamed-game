use bevy::prelude::*;
use bevy::scene::SceneInstance;
use bevy_mod_picking::prelude::*;
use std::f32::consts::PI;

use bevy::prelude::Vec3;

use crate::cell::{Cell, CellColor, CellCoordinates};
use crate::gamemanager::{self, Game};
use crate::materials;

pub(crate) fn construct_cube(
    side_length: u32,
    meshes: &mut ResMut<Assets<Mesh>>,
    commands: &mut Commands,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    material: &StandardMaterial,
    game: &mut ResMut<Game>,
) {
    fn choose_color(side_length: u32, i: u32, c1: CellColor, c2: CellColor) -> CellColor {
        #[allow(clippy::collapsible_else_if)]
        if side_length % 2 == 0 {
            if (i / side_length + i % 2) % 2 == 0 {
                c1
            } else {
                c2
            }
        } else {
            if i % 2 == 0 {
                c1
            } else {
                c2
            }
        }
    }

    let plane_mesh: Handle<Mesh> = meshes.add(shape::Plane::default().into());
    let spacing = 1. / (side_length) as f32;
    let offset = 0.5 - spacing / 2.;
    // The total side length of cube is always 1, so we offset
    // by 0.5 to get middle in origo. When cube at origo, half of its side is in negative
    // quadrant, so therefore we subtract the part that is already offset from this phenomenon.
    for side in 0..6 {
        //        lookup_planes.planes[side] = vec![None; side_length.pow(2) as usize];
        for i in 0..side_length.pow(2) {
            let translation;
            let mut rotation;
            let color: CellColor;
            #[allow(clippy::needless_late_init)]
            let coords;
            match side {
                0 | 1 => {
                    translation = Vec3::new(
                        (i % side_length) as f32 * spacing - offset,
                        if side % 2 == 0 { 0.5 } else { -0.5 },
                        (i / side_length % side_length) as f32 * spacing - offset,
                    );
                    rotation = Vec3::new(0., 0., 2.); // Up/down rotate 180 degrees, which is 2 turns
                    color = choose_color(side_length, i, CellColor::White, CellColor::Gray);
                    coords = CellCoordinates::new(
                        i % side_length + 1,
                        0,
                        i / side_length % side_length + 1,
                        side % 2 == 0,
                    )
                }
                2 | 3 => {
                    translation = Vec3::new(
                        (i % side_length) as f32 * spacing - offset,
                        (i / side_length % side_length) as f32 * spacing - offset,
                        if side % 2 == 1 { 0.5 } else { -0.5 },
                    );
                    rotation = Vec3::new(1., 0., 0.);
                    color = choose_color(side_length, i, CellColor::Black, CellColor::White);
                    coords = CellCoordinates::new(
                        i % side_length + 1,
                        i / side_length % side_length + 1,
                        0,
                        side % 2 == 1,
                    )
                }
                4 | 5 => {
                    translation = Vec3::new(
                        if side % 2 == 0 { 0.5 } else { -0.5 },
                        (i / side_length % side_length) as f32 * spacing - offset,
                        (i % side_length) as f32 * spacing - offset,
                    );
                    rotation = Vec3::new(0., 0., 1.);
                    color = choose_color(side_length, i, CellColor::Gray, CellColor::Black);
                    coords = CellCoordinates::new(
                        0,
                        i / side_length % side_length + 1,
                        i % side_length + 1,
                        side % 2 == 0,
                    )
                }
                _ => unreachable!(),
            }

            rotation *= Vec3::splat(PI / 2.);
            if side % 2 == 0 {
                rotation.x -= if rotation.x == 0. { 0. } else { PI };
                rotation.y -= if rotation.y == 0. { 0. } else { PI };
                rotation.z -= if rotation.z == 0. { 0. } else { PI };
            }

            let plane = commands
                .spawn((
                    PbrBundle {
                        mesh: plane_mesh.clone(),
                        material: materials.add(material.clone()),
                        transform: Transform::from_translation(translation)
                            .with_scale(Vec3::splat(spacing))
                            .with_rotation(Quat::from_scaled_axis(rotation)),
                        ..default()
                    },
                    PickableBundle::default(),
                    RaycastPickTarget::default(),
                    MainCube { coords },
                    OnPointer::<Click>::run_callback(gamemanager::on_cell_clicked),
                ))
                .id();

            let cell = Cell::new(plane, coords, color);
            game.board.new_cell(coords, cell);
        }
    }
}

#[derive(Component)]
pub(crate) struct MainCube {
    pub(crate) coords: CellCoordinates,
}

pub(crate) fn update_cell_colors(
    query: Query<(&mut Handle<StandardMaterial>, &MainCube)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    game: ResMut<Game>,
) {
    for cell in game.board.get_all_cells() {
        let plane = cell.plane;

        let query_result = query.get(plane).unwrap();
        let material = materials.get_mut(query_result.0).unwrap();
        if game.selected_cell.map_or(false, |x| x == cell.coords) {
            materials::select_cell_material(material, cell.color);
        } else if cell.selected_unit_can_move_to {
            materials::can_go_cell_material(material, cell.color);
        } else {
            materials::normal_cell_material(material, cell.color);
        }
    }
}

/// A "flag" to make a separate system add the pickable tasks to our unit entities
#[derive(Component, Default, Debug)]
pub(crate) struct AddPickable;

pub(crate) fn spawn_unit(
    commands: &mut Commands,
    asset_server: Res<AssetServer>,
    model_name: &str,
) -> Entity {
    let entity = commands
        .spawn((
            SceneBundle {
                scene: asset_server.load(format!("models/{}.glb#Scene0", model_name)),
                ..default()
            },
            AddPickable,
        ))
        .id();
    entity
}

#[derive(Component)]
pub(crate) struct SceneChild {
    pub(crate) parent_entity: Entity,
}

/// Add pickable and change material color
pub(crate) fn add_pickable_to_unit(
    mut commands: Commands,
    mut unloaded_instances: Query<(Entity, &SceneInstance), With<AddPickable>>,
    mut material_query: Query<&mut Handle<StandardMaterial>>,
    game: Res<Game>,
    scene_manager: Res<SceneSpawner>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (parent_entity, instance) in unloaded_instances.iter_mut() {
        if scene_manager.instance_is_ready(**instance) {
            commands.entity(parent_entity).remove::<AddPickable>();

            let unit = game.units.get_unit_from_entity(parent_entity);
            let color = unit.unwrap().team.color();

            // Iterate over all entities in scene (once it's loaded)
            let handles = scene_manager.iter_instance_entities(**instance);
            for entity in handles {
                commands.entity(entity).insert((
                    PickableBundle::default(),
                    RaycastPickTarget::default(),
                    OnPointer::<Click>::run_callback(gamemanager::on_unit_clicked),
                    SceneChild { parent_entity },
                ));

                let material = material_query.get_mut(entity);
                if let Ok(material) = material {
                    let material = materials.get_mut(material.into_inner());
                    if let Some(material) = material {
                        material.base_color = color;
                    }
                }
            }
        }
    }
}

pub(crate) fn kill_unit(commands: &mut Commands, entity: Entity) {
    commands.entity(entity).despawn_recursive();
}

pub(crate) fn move_unit_entities(
    mut query: Query<(Option<&MainCube>, &mut Transform)>,
    mut game: ResMut<Game>,
) {
    for unit_to_move in &game.units_to_move {
        dbg!(unit_to_move);
        let plane = game.board.get_cell(unit_to_move.1).unwrap().plane;
        let target_translation = query.get(plane).unwrap().1.translation;
        let scale = 3. / game.board.cube_side_length as f32;
        let rotation =
            Quat::from_rotation_arc(Vec3::Y, unit_to_move.1.normal_direction().as_vec3());

        let mut transform_unit = query.get_mut(unit_to_move.0).unwrap().1;
        transform_unit.translation = target_translation;
        transform_unit.scale = Vec3::splat(scale);
        transform_unit.rotation = rotation;
    }
    game.units_to_move.clear();
}
