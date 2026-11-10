//! The editor's colors. Interface colors are read from gpui-component's active theme
//! (`cx.theme()`, shadcn-style tokens), so changing that theme restyles the whole editor; this
//! module only says which token each part of the editor uses. Pin and category colors encode
//! data (a pin's type, a node's category), not the look, so they stay fixed.

use gpui_kit::component::ActiveTheme;
use gpui_kit::{App, Hsla, Rgba, rgb};
use vs_graph::PinType;

pub fn canvas_bg(cx: &App) -> Hsla {
    cx.theme().background
}
pub fn grid_minor(cx: &App) -> Hsla {
    cx.theme().border.opacity(0.35)
}
pub fn grid_major(cx: &App) -> Hsla {
    cx.theme().border.opacity(0.8)
}
pub fn panel_bg(cx: &App) -> Hsla {
    cx.theme().sidebar
}
pub fn toolbar_bg(cx: &App) -> Hsla {
    cx.theme().title_bar
}
pub fn border(cx: &App) -> Hsla {
    cx.theme().border
}
pub fn text(cx: &App) -> Hsla {
    cx.theme().foreground
}
pub fn dim(cx: &App) -> Hsla {
    cx.theme().muted_foreground
}
/// Selection and focus outlines: selected nodes, the field being edited, the search box.
pub fn selection(cx: &App) -> Hsla {
    cx.theme().primary
}
/// The background of the highlighted row or the active tab.
pub fn highlight(cx: &App) -> Hsla {
    cx.theme().accent
}
/// A filled call-to-action, such as a dialog's confirm button, and the text on it.
pub fn primary(cx: &App) -> (Hsla, Hsla) {
    (cx.theme().primary, cx.theme().primary_foreground)
}
pub fn node_bg(cx: &App) -> Hsla {
    cx.theme().popover
}
pub fn field_bg(cx: &App) -> Hsla {
    cx.theme().background
}
pub fn button_bg(cx: &App) -> Hsla {
    cx.theme().secondary
}
pub fn button_hover(cx: &App) -> Hsla {
    cx.theme().secondary_hover
}
pub fn error(cx: &App) -> Hsla {
    cx.theme().danger
}
pub fn warning(cx: &App) -> Hsla {
    cx.theme().warning
}
pub fn ok(cx: &App) -> Hsla {
    cx.theme().success
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

/// The color of a script's `index`th system: its group's frame and the stripe on its nodes.
/// After the last one they repeat.
pub fn system(index: usize) -> Rgba {
    const COLORS: [u32; 6] = [0x4f9dde, 0xe0a13a, 0x5fbf7a, 0xc77ddb, 0xe0685f, 0x4fc4c4];
    rgb(COLORS[index % COLORS.len()])
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
