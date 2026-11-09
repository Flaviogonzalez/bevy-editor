//! The panels around the canvas, one file each: the toolbar, the Explorer on the left, the
//! Inspector on the right, the bottom panel and the modal dialogs. Each reads the editor's state
//! and dispatches actions or calls `Editor` methods; none reaches into the canvas.




mod bottom;
mod dialog;
mod explorer;
mod inspector;
mod toolbar;

use crate::editor::{BottomTab, CodeView, Dialog};

/// The panels' own state: what they show, independent of the open script.
pub struct Chrome {
    /// Whether the panel under the canvas shows, and which of its tabs.
    pub show_bottom_panel: bool,
    pub bottom_tab: BottomTab,
    /// Which generated code the Code tab shows.
    pub code_view: CodeView,
    /// The line at the bottom of the window: the result of the last command.
    pub status: String,
    pub dialog: Option<Dialog>,
}

impl Default for Chrome {
    fn default() -> Self {
        Self {
            show_bottom_panel: true,
            bottom_tab: BottomTab::Code,
            code_view: CodeView::Lua,
            status: String::new(),
            dialog: None,
        }
    }
}
