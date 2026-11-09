//! Bevy visual script editor: a node graph editor, built on GPUI through gpui-kit, that saves Lua
//! scripts.

mod actions;
mod app;
mod canvas;
mod document;
mod editor;
mod panels;
mod theme;
mod ui;
mod workspace;

use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::{App, AppContext, Bounds, QuitMode, TitlebarOptions, WindowBounds, WindowOptions, px, size};

fn main() {
    // The bundled icons (Lucide) used by gpui-component's buttons and empty states.
    gpui_kit::application().with_assets(gpui_kit::assets::Assets).with_quit_mode(QuitMode::LastWindowClosed).run(|cx: &mut App| {
        // Sets up gpui-kit's layers (base behavior, components, assets) before any window opens.
        gpui_kit::init(cx);
        // The editor is dark-only for now (see theme.rs); gpui-component starts in light mode.
        Theme::change(ThemeMode::Dark, None, cx);
        actions::bind_keys(cx);
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
