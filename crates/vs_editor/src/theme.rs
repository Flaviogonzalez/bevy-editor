//! Colors. One dark palette for now.

use gpui_kit::{Rgba, rgb};
use vs_graph::PinType;

pub fn canvas_bg() -> Rgba {
    rgb(0x1a1c21)
}
pub fn grid_minor() -> Rgba {
    rgb(0x22252b)
}
pub fn grid_major() -> Rgba {
    rgb(0x2b2f37)
}
pub fn panel_bg() -> Rgba {
    rgb(0x22252b)
}
pub fn toolbar_bg() -> Rgba {
    rgb(0x1e2025)
}
pub fn border() -> Rgba {
    rgb(0x353a44)
}
pub fn text() -> Rgba {
    rgb(0xd8dbe1)
}
pub fn dim() -> Rgba {
    rgb(0x8a909b)
}
pub fn accent() -> Rgba {
    rgb(0x4d8df7)
}
pub fn node_bg() -> Rgba {
    rgb(0x2a2e36)
}
pub fn field_bg() -> Rgba {
    rgb(0x1b1e23)
}
pub fn button_bg() -> Rgba {
    rgb(0x2c3038)
}
pub fn error() -> Rgba {
    rgb(0xe06c6c)
}
pub fn warning() -> Rgba {
    rgb(0xd9a74a)
}
pub fn ok() -> Rgba {
    rgb(0x6cc070)
}

pub fn category(category: &str) -> Rgba {
    rgb(match category {
        "Events" => 0x8f3434,
        "Flow" => 0x545a66,
        "Actions" => 0x2d6894,
        "Transform" => 0x2b7d61,
        "Input" => 0x86662a,
        "Time" => 0x644a92,
        "Math" => 0x3b6a4a,
        "Logic" => 0x733a63,
        "Text" => 0x82496a,
        "Variables" => 0x485a94,
        "Vector" => 0x8a7a2c,
        "Rotation" => 0x2f6f8f,
        "Color" => 0x94573a,
        "Components" => 0x3d7a7a,
        "Entity" => 0x7a3d5c,
        _ => 0x444444,
    })
}

pub fn pin(ty: PinType) -> Rgba {
    rgb(match ty {
        PinType::Exec => 0xe8e8e8,
        PinType::Number => 0x7fd67f,
        PinType::Bool => 0xe06c6c,
        PinType::String => 0xd77fd7,
        PinType::Vec3 => 0xf0c94a,
        PinType::Quat => 0x6fb7e8,
        PinType::Color => 0xff9a5a,
        PinType::Any => 0x9aa0aa,
    })
}
