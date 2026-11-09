//! Projects in the editor: settings, creating and opening projects and scripts, and running the
//! game (preview) or building it for release.

use std::time::Duration;

use gpui::{Context, Window};
use vs_project::Project;

use crate::editor::{Dialog, Editor};
use crate::tasks::{Task, Then};

impl Editor {
    pub fn open_settings(&mut self) {
        self.dialog = Some(Dialog::Settings);
    }

    /// Asks for a new projects folder and saves it.
    pub fn change_projects_dir(&mut self) {
        let mut picker = rfd::FileDialog::new().set_title("Folder for new projects");
        if self.config.projects_dir.exists() {
            picker = picker.set_directory(&self.config.projects_dir);
        }
        let Some(dir) = picker.pick_folder() else { return };
        self.config.projects_dir = dir;
        match self.config.save() {
            Ok(()) => self.status = format!("New projects go in {}", self.config.projects_dir.display()),
            Err(e) => self.status = format!("Couldn't save the settings: {e}"),
        }
    }

    pub fn new_project_dialog(&mut self) {
        self.dialog = Some(Dialog::NewProject { buffer: String::new() });
    }

    pub fn new_script_dialog(&mut self) {
        if self.project.is_some() {
            self.dialog = Some(Dialog::NewScript { buffer: String::new() });
        }
    }

    pub fn dialog_key(&mut self, key: &str, typed: Option<String>, window: &mut Window) {
        match key {
            "escape" => self.dialog = None,
            "enter" => self.confirm_dialog(window),
            "backspace" => {
                if let Some(Dialog::NewProject { buffer } | Dialog::NewScript { buffer }) = &mut self.dialog {
                    buffer.pop();
                }
            }
            _ => {
                if let (Some(t), Some(Dialog::NewProject { buffer } | Dialog::NewScript { buffer })) = (typed, &mut self.dialog) {
                    buffer.push_str(&t);
                }
            }
        }
    }

    pub fn confirm_dialog(&mut self, window: &mut Window) {
        match self.dialog.take() {
            Some(Dialog::NewProject { buffer }) => {
                if !self.confirm_discard() {
                    return;
                }
                match vs_project::create(&self.config.projects_dir, &buffer) {
                    Ok(project) => {
                        self.enter_project(project, window);
                        self.status = format!("Created {}. Press Preview to run it", self.status_path());
                    }
                    Err(e) => {
                        self.status = format!("Couldn't create the project: {e}");
                        self.dialog = Some(Dialog::NewProject { buffer });
                    }
                }
            }
            Some(Dialog::NewScript { buffer }) => {
                let Some(project) = self.project.clone() else { return };
                if !self.confirm_discard() {
                    return;
                }
                match project.new_script(&buffer) {
                    Ok(path) => {
                        self.open_path(path, window);
                        if let Some(stem) = self.path.as_ref().and_then(|p| p.file_stem()) {
                            let ty = vs_graph::rust::names(&stem.to_string_lossy()).ty;
                            self.status = format!("Spawn `scripts::{ty}::default()` on an entity in src/main.rs to use it");
                        }
                    }
                    Err(e) => {
                        self.status = format!("Couldn't add the script: {e}");
                        self.dialog = Some(Dialog::NewScript { buffer });
                    }
                }
            }
            Some(Dialog::Settings) | None => {}
        }
    }

    fn status_path(&self) -> String {
        self.project.as_ref().map(|p| p.root.display().to_string()).unwrap_or_default()
    }

    /// Picks a project folder, starting in the projects folder.
    pub fn open_project(&mut self, window: &mut Window) {
        if !self.confirm_discard() {
            return;
        }
        let mut picker = rfd::FileDialog::new().set_title("Open project");
        if self.config.projects_dir.exists() {
            picker = picker.set_directory(&self.config.projects_dir);
        }
        let Some(dir) = picker.pick_folder() else { return };
        match Project::open(&dir) {
            Ok(project) => self.enter_project(project, window),
            Err(e) => self.status = format!("{e}"),
        }
    }

    /// Makes `project` current and opens its first script.
    pub fn enter_project(&mut self, project: Project, window: &mut Window) {
        let first = project.scripts().into_iter().next();
        if let Err(e) = vs_graph::components::load_project(&project.root) {
            self.status = e;
        }
        self.project = Some(project);
        match first {
            Some(path) => self.open_path(path, window),
            None => {
                *self = Self { graph: vs_graph::Graph::default(), path: None, ..self.reset() };
                self.update_title(window);
            }
        }
    }

    /// Saves the open script before a run, so the game sees what's on screen.
    fn save_for_run(&mut self, window: &mut Window) -> bool {
        if !self.dirty {
            return true;
        }
        match self.path.clone() {
            Some(_) => {
                self.save(window);
                !self.dirty
            }
            None => {
                self.status = "Save the script into the project first".into();
                false
            }
        }
    }

    pub fn task_running(&self) -> bool {
        self.task.borrow().as_ref().is_some_and(|t| t.finished().is_none())
    }

    /// Runs the game with its scripts in Lua. Saving a script while it runs reloads it.
    pub fn run_preview(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(project) = self.project.clone() else { return };
        if self.task_running() || !self.save_for_run(window) {
            return;
        }
        let command = vs_project::cargo(&project.root, &["run", "--color", "never", "--features", "preview"]);
        self.start(Task::spawn(format!("Preview {}", project.name), command, None), cx);
        self.status = "Starting the preview. The first build takes a few minutes; later ones are quick".into();
    }

    /// Runs the game only to write its catalog of components (`.bevy-editor/components.json`),
    /// as its `.bevy-editor/catalog.toml` asks: no window, and it exits once done.
    pub fn refresh_catalog(&mut self, cx: &mut Context<Self>) {
        let Some(project) = self.project.clone() else { return };
        if self.task_running() {
            return;
        }
        let mut command = vs_project::cargo(&project.root, &["run", "--color", "never", "--features", "preview"]);
        command.env(vs_graph::components::CATALOG_ENV, "1");
        let label = format!("Catalog of {}", project.name);
        self.start(Task::spawn(label, command, Some(Then::Catalog(project.root.clone()))), cx);
        self.status = "Building the game to read its components…".into();
    }

    /// Opens the project's catalog config in the system's editor for `.toml` files, writing the
    /// default one first if there's none.
    pub fn edit_catalog(&mut self) {
        let Some(project) = &self.project else { return };
        let path = project.catalog_config();
        if !path.exists() {
            let written = std::fs::create_dir_all(path.parent().unwrap())
                .and_then(|()| std::fs::write(&path, vs_graph::components::DEFAULT_CONFIG));
            if let Err(e) = written {
                self.status = format!("Couldn't write {}: {e}", path.display());
                return;
            }
        }
        if let Err(e) = open_with_system(&path) {
            self.status = format!("Couldn't open {}: {e}", path.display());
        } else {
            self.status = "Edit the catalog, save it, then Refresh catalog".into();
        }
    }

    /// Exports the project as plain Bevy, builds it in release and packages it in `dist`.
    pub fn build(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(project) = self.project.clone() else { return };
        if self.task_running() || !self.save_for_run(window) {
            return;
        }
        if let Err(e) = vs_project::export(&project, &project.export_dir()) {
            self.status = format!("Couldn't export: {e}");
            return;
        }
        let command = vs_project::cargo(&project.export_dir(), &["build", "--release", "--color", "never"]);
        let label = format!("Build {}", project.name);
        self.start(Task::spawn(label, command, Some(Then::Package(project))), cx);
        self.status = "Exported. Building in release…".into();
    }

    pub fn stop_task(&mut self) {
        if let Some(task) = self.task.borrow().as_ref() {
            task.stop();
        }
        self.status = "Stopped".into();
    }

    fn start(&mut self, task: std::io::Result<Task>, cx: &mut Context<Self>) {
        match task {
            Ok(task) => {
                *self.task.borrow_mut() = Some(task);
                cx.spawn(async move |this, cx| {
                    loop {
                        cx.background_executor().timer(Duration::from_millis(250)).await;
                        let running = this.update(cx, |editor, cx| {
                            cx.notify();
                            editor.poll_task()
                        });
                        if !matches!(running, Ok(true)) {
                            break;
                        }
                    }
                })
                .detach();
            }
            Err(e) => self.status = format!("Couldn't start cargo: {e}"),
        }
    }

    /// Finishes a task that has exited. Returns whether it's still running.
    pub fn poll_task(&mut self) -> bool {
        let mut task = self.task.borrow_mut();
        let Some(t) = task.as_mut() else { return false };
        let Some(success) = t.finished() else { return true };
        let then = t.then.take();
        let label = t.label.clone();
        drop(task);
        self.status = match (success, then) {
            (true, Some(Then::Package(project))) => match vs_project::package(&project) {
                Ok(exe) => format!("Built {}. Its folder is ready to ship", exe.display()),
                Err(e) => format!("The build finished but couldn't be packaged: {e}"),
            },
            (true, Some(Then::Catalog(root))) => match vs_graph::components::load_project(&root) {
                Ok(n) => format!("Catalog updated: {n} component(s) from the game, besides Bevy's common ones"),
                Err(e) => format!("The game wrote a catalog the editor can't read: {e}"),
            },
            (true, None) => format!("{label} finished"),
            (false, _) => format!("{label} failed: see the output panel"),
        };
        false
    }
}

/// Opens `path` with the application the system associates with it.
fn open_with_system(path: &std::path::Path) -> std::io::Result<()> {
    #[cfg(windows)]
    let mut command = {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let mut c = std::process::Command::new("cmd");
        c.args(["/C", "start", ""]).arg(path).creation_flags(CREATE_NO_WINDOW);
        c
    };
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut c = std::process::Command::new("open");
        c.arg(path);
        c
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(path);
        c
    };
    command.spawn().map(|_| ())
}
