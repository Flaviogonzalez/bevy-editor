//! Does Bevy's hot patching make a Rust backend iterate as fast as Lua?
//!
//! `demo_script` below is what a Rust backend could generate from `demo.lua`: the script's
//! variables and flip-flop state become a component, and its events become one system. Run with
//! `dx serve --hot-patch`, edit the system while the window is open, and see what applies live.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (demo_script, probe, extra))
        .run();
}

fn extra(mut done: Local<bool>) {
    if !*done {
        *done = true;
        info!(target: "probe", "extra system ran");
    }
}

/// Generated from the graph's variables and stateful nodes.
#[derive(Component)]
struct DemoScript {
    speed: f32,
    flipflop_14: bool,
    bob: f32,
}

/// Generated from On Update [3] and On Key Pressed [13].
fn demo_script(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut DemoScript, &mut Transform)>,
) {
    let delta = time.delta_secs();
    for (mut script, mut transform) in &mut query {
        // Rotate [6]
        let step = (script.speed * delta).to_radians();
        let r = Quat::from_euler(EulerRot::XYZ, step, step, 0.0);
        transform.rotation = (transform.rotation * r).normalize();
        // Set Position [12]
        transform.translation = Vec3::new(0.0, (time.elapsed_secs() * 2.0).sin() * script.bob + 1.0, 0.0);
        if time.elapsed_secs() % 2.0 < delta {
            info!(target: "probe", "bob = {}", script.bob);
        }
        // On Key Pressed [13] -> Flip Flop [14]
        if keys.just_pressed(KeyCode::Space) {
            script.flipflop_14 = !script.flipflop_14;
            transform.scale = if script.flipflop_14 { Vec3::splat(1.6) } else { Vec3::ONE };
        }
    }
}

/// A marker in the log every two seconds, to tell which version of the code is running.
fn probe(time: Res<Time>, mut next: Local<f32>) {
    if time.elapsed_secs() >= *next {
        *next = time.elapsed_secs() + 2.0;
        info!(target: "probe", "probe v2");
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.55, 0.95))),
        Transform::from_xyz(0.0, 1.0, 0.0),
        DemoScript { speed: 90.0, flipflop_14: false, bob: 0.5 },
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
        Transform::from_xyz(-3.0, 3.0, 5.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
    ));
}

