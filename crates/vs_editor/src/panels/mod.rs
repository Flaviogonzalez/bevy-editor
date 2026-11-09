//! The panels around the canvas, one file each: the toolbar, the Explorer on the left, the
//! Inspector on the right, the bottom panel and the modal dialogs. Each reads the editor's state
//! and dispatches actions or calls `Editor` methods; none reaches into the canvas.




mod bottom;
mod dialog;
mod explorer;
mod inspector;
mod toolbar;
