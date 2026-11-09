mod scripts;

use bevy::prelude::*;
use scripts::Spinner;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, scripts::ScriptsPlugin))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.55, 0.95))),
        Transform::from_xyz(0.0, 0.5, 0.0),
        Spinner::default(),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(8.0, 8.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.22, 0.24, 0.27))),
    ));
    commands.spawn((
        DirectionalLight { shadow_maps_enabled: true, ..default() },
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-3.0, 3.0, 5.0).looking_at(Vec3::new(0.0, 0.5, 0.0), Vec3::Y),
    ));
}
