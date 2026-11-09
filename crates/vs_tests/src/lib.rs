//! The differential test's fixtures: the game-side code its scripts expect, as a real project
//! would have it. See `tests/differential.rs`.

use bevy::prelude::*;

/// A game's own state, which `components.lua` reads and writes like any Bevy component. Being
/// reflected with a `Default`, the preview tells the editor about it
/// (`.bevy-editor/components.json`), so scripts can Get and Set its fields.
#[derive(Component, Reflect, Default, Clone, Copy, Debug, PartialEq)]
#[reflect(Component, Default)]
pub struct Wobble {
    pub amount: f32,
    pub axis: Vec3,
    pub tint: Color,
    pub active: bool,
}
