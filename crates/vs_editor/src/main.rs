//! Bevy visual script editor: a node graph editor, built on GPUI through gpui-kit, that saves Lua
//! scripts.

mod editor;
mod history;
mod layout;
mod project;
mod render;
mod tasks;
mod theme;

use gpui_kit::{App, AppContext, Bounds, QuitMode, TitlebarOptions, WindowBounds, WindowOptions, px, size};

fn main() {
    gpui_kit::application().with_quit_mode(QuitMode::LastWindowClosed).run(|cx: &mut App| {
        // Sets up gpui-kit's layers (base behavior, components, assets) before any window opens.
        gpui_kit::init(cx);
        let bounds = Bounds::centered(None, size(px(1400.0), px(860.0)), cx);
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("untitled.lua - Bevy Visual Script Editor".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            cx,
            |window, cx| cx.new(|cx| editor::Editor::new(window, cx)),
        )
        .expect("failed to open the editor window");
        cx.activate(true);
    });
}
