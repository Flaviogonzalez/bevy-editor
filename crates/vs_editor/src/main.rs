//! Bevy visual script editor: a node graph editor, built on gpui-ce, that saves Lua scripts.

mod editor;
mod layout;
mod project;
mod render;
mod tasks;
mod theme;

use gpui::{App, AppContext, Bounds, QuitMode, TitlebarOptions, WindowBounds, WindowOptions, px, size};

fn main() {
    gpui_platform::application().with_quit_mode(QuitMode::LastWindowClosed).run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1400.0), px(860.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("untitled.lua - Bevy Visual Script Editor".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |window, cx| cx.new(|cx| editor::Editor::new(window, cx)),
        )
        .expect("failed to open the editor window");
        cx.activate(true);
    });
}
