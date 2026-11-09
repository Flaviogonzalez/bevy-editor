//! Intrinsics: what a script can read from its frame and do to its entity.
//!
//! Unlike pure nodes these aren't generated: they're a small, fixed set. Both runtimes go through
//! these functions, so moving or rotating means the same thing in the preview and in the game.

use bevy::input::ButtonInput;
use bevy::log::info;
use bevy::math::EulerRot;
pub use bevy::color::Color;
pub use bevy::math::{Quat, Vec3};
use bevy::transform::components::Transform;

/// What a script sees during one update, and the transform it edits. Runtimes copy the entity's
/// transform in, run the script, and write it back if it changed.
#[derive(Clone, Debug, Default)]
pub struct Frame {
    pub delta: f32,
    pub time: f32,
    pub keys: Buttons,
    pub mouse: Buttons,
    pub transform: Transform,
    /// Set by the script: remove the entity once this frame's update is done.
    pub despawn: bool,
}

/// One device's buttons this frame, by variant name (`Space`, `Left`): the names the Rust
/// backend writes as `KeyCode::Space` or `MouseButton::Left`, so both runtimes ask the same.
#[derive(Clone, Debug, Default)]
pub struct Buttons {
    pub down: Vec<String>,
    pub just_pressed: Vec<String>,
    pub just_released: Vec<String>,
}

impl Buttons {
    pub fn of<T: Copy + Eq + std::hash::Hash + Send + Sync + std::fmt::Debug + 'static>(input: &ButtonInput<T>) -> Self {
        fn names<'a, T: std::fmt::Debug + 'a>(buttons: impl Iterator<Item = &'a T>) -> Vec<String> {
            buttons.map(|b| format!("{b:?}")).collect()
        }
        Buttons {
            down: names(input.get_pressed()),
            just_pressed: names(input.get_just_pressed()),
            just_released: names(input.get_just_released()),
        }
    }

    /// `press` is `down`, `pressed` or `released`, as the Lua backend writes it.
    pub fn query(&self, press: &str, name: &str) -> bool {
        let list = match press {
            "down" => &self.down,
            "pressed" => &self.just_pressed,
            "released" => &self.just_released,
            _ => return false,
        };
        list.iter().any(|b| b == name)
    }
}

pub fn translate(t: &mut Transform, x: f32, y: f32, z: f32) {
    t.translation += Vec3::new(x, y, z);
}

/// A rotation from Euler angles in degrees, XYZ order. Rotation literals and the Rotate node both
/// build their rotation this way.
pub fn euler(x: f32, y: f32, z: f32) -> Quat {
    Quat::from_euler(EulerRot::XYZ, x.to_radians(), y.to_radians(), z.to_radians())
}

/// Rotates by Euler angles in degrees, applied in the entity's local space.
pub fn rotate(t: &mut Transform, x: f32, y: f32, z: f32) {
    t.rotation = (t.rotation * euler(x, y, z)).normalize();
}

pub fn set_position(t: &mut Transform, x: f32, y: f32, z: f32) {
    t.translation = Vec3::new(x, y, z);
}

pub fn set_scale(t: &mut Transform, x: f32, y: f32, z: f32) {
    t.scale = Vec3::new(x, y, z);
}

pub fn print(script: &str, text: &str) {
    info!(target: "visual_script", "[{script}] {text}");
}

/// An sRGB color with alpha. Color literals are built this way.
pub fn srgba(r: f32, g: f32, b: f32, a: f32) -> Color {
    Color::srgba(r, g, b, a)
}
