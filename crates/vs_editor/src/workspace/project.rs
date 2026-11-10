//! Projects in the editor: opening a project, loading its catalog of components, and running
//! the game (preview) or building it for release. The dialogs that create projects and scripts
//! live in `panels/dialog.rs` and call into here.

use std::collections::BTreeMap;
use std::time::Duration;

use gpui_kit::{Context, Window};
use vs_graph::{Compiled, NodeId, Problem, Severity, access};
use vs_project::Project;

use crate::editor::Editor;
use crate::panels::BottomTab;
use crate::workspace::Outline;
use crate::workspace::tasks::{Outcome, Task, Then};

impl Editor {
    /// Picks a project folder, starting in the projects folder.
    pub fn open_project(&mut self, window: &mut Window) {
        if !self.confirm_discard_all() {
            return;
        }
        let mut picker = rfd::FileDialog::new().set_title("Open project");
        if self.workspace.config.projects_dir.exists() {
            picker = picker.set_directory(&self.workspace.config.projects_dir);
        }
        let Some(dir) = picker.pick_folder() else { return };
        match Project::open(&dir) {
            Ok(project) => self.enter_project(project, window),
            Err(e) => self.chrome.status = format!("{e}"),
        }
    }

    /// Makes `project` current and loads its catalog of components into the editor-wide store,
    /// which the compiler and the add-node menu read. Every change of project goes through here,
    /// so the catalog always belongs to the open project. An unreadable catalog leaves the project
    /// with only Bevy's common components; the error says why.
    pub fn set_project(&mut self, project: Project) -> Result<(), String> {
        let loaded = vs_graph::components::load_project(&project.root).map(|_| ());
        if loaded.is_err() {
            vs_graph::components::set_project(Vec::new());
        }
        self.workspace.project = Some(project);
        self.scan_systems();
        loaded
    }

    /// The open script's path in the project's scripts folder (`player/jump.lua`), as the catalog
    /// and messages name scripts; just its file name outside a project, or before it's saved.
    pub fn script_name(&self) -> String {
        match (&self.workspace.project, &self.doc.path) {
            (Some(project), Some(path)) if path.starts_with(project.scripts_dir()) => project.script_name(path),
            _ => self.doc.file_name(),
        }
    }

    /// Reads the project's scripts other than the open one: their systems, for
    /// [`Editor::ambiguities`], and their outlines, for the Explorer. Call it when the project,
    /// the open script or its name changes; the other scripts only change on disk when they're
    /// open, so that's enough.
    pub fn scan_systems(&mut self) {
        let own = self.script_name();
        self.workspace.others.clear();
        self.workspace.outlines.clear();
        let Some(project) = &self.workspace.project else { return };
        for path in project.scripts() {
            let file = project.script_name(&path);
            if file == own {
                continue;
            }
            let Some(graph) = std::fs::read_to_string(&path).ok().and_then(|text| vs_graph::load_from_str(&text).ok())
            else {
                continue;
            };
            let module = vs_graph::lower(&graph).module;
            let systems = access::systems(&module, &vs_graph::rust::names(&file));
            self.workspace.outlines.insert(path, Outline::of(&graph, &module, &file));
            self.workspace.others.push((file, systems));
        }
    }

    /// A warning on each event whose system Bevy's ambiguity detection would report together
    /// with systems of the project's other scripts (see [`access::ambiguities`]).
    pub fn ambiguities(&self, compiled: &Compiled) -> Vec<Problem> {
        let own: Vec<(NodeId, access::System)> = self.systems(compiled).into_iter().collect();
        let mut scripts = vec![own.iter().map(|(_, s)| s.clone()).collect::<Vec<_>>()];
        scripts.extend(self.workspace.others.iter().map(|(_, systems)| systems.clone()));
        // For each of the open script's systems, the others it's ambiguous with, grouped by the
        // components they conflict on: `Transform` → [`update_math` (math.lua), …].
        let mut found: Vec<BTreeMap<String, Vec<String>>> = vec![BTreeMap::new(); own.len()];
        for a in access::ambiguities(&scripts).into_iter().filter(|a| a.a.script == 0) {
            let (file, systems) = &self.workspace.others[a.b.script - 1];
            let other = format!("`{}` ({file})", systems[a.b.system].name);
            found[a.a.system].entry(a.components.join(", ")).or_default().push(other);
        }
        own.iter()
            .zip(found)
            .filter(|(_, groups)| !groups.is_empty())
            .map(|((node, system), groups)| {
                let groups: Vec<String> = groups
                    .into_iter()
                    .map(|(components, others)| {
                        let count = if others.len() == 1 { "1 system".to_string() } else { format!("{} systems", others.len()) };
                        format!("{count} on {components}: {}", others.join(", "))
                    })
                    .collect();
                Problem {
                    severity: Severity::Warning,
                    node: Some(*node),
                    message: format!(
                        "`{}` has no order with {}. Bevy's ambiguity detection reports each pair",
                        system.name,
                        groups.join("; "),
                    ),
                }
            })
            .collect()
    }

    /// Makes `project` current and opens its first script. Every tab closes, without asking: the
    /// callers asked already.
    pub fn enter_project(&mut self, project: Project, window: &mut Window) {
        self.tabs.close_others();
        self.reset(crate::document::Document::default());
        let first = project.scripts().into_iter().next();
        let catalog = self.set_project(project);
        match first {
            Some(path) => self.open_path(path, window),
            None => {
                self.reset(crate::document::Document::default());
                if let Err(e) = catalog {
                    self.chrome.status = e;
                }
                self.update_title(window);
            }
        }
    }

    /// Saves the open scripts before a run, so the game sees what's on screen, in every tab.
    fn save_for_run(&mut self, window: &mut Window) -> bool {
        for doc in self.tabs.others_mut().filter(|d| d.history.is_dirty()) {
            let Some(path) = doc.path.clone() else { continue };
            if let Err(e) = doc.write_to(&path) {
                self.chrome.status = format!("Couldn't save {}: {e}", path.display());
                return false;
            }
        }
        if !self.doc.history.is_dirty() {
            return true;
        }
        match self.doc.path.clone() {
            Some(_) => {
                self.save(window);
                !self.doc.history.is_dirty()
            }
            None => {
                self.chrome.status = "Save the script into the project first".into();
                false
            }
        }
    }

    pub fn task_running(&self) -> bool {
        self.workspace.task.borrow().as_ref().is_some_and(|t| t.finished().is_none())
    }

    /// Runs the game with its scripts in Lua. Saving a script while it runs reloads it.
    pub fn run_preview(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(project) = self.workspace.project.clone() else { return };
        if self.task_running() || !self.save_for_run(window) {
            return;
        }
        let command = vs_project::cargo(&project.root, &["run", "--color", "never", "--features", "preview"]);
        self.start(Task::spawn(format!("Preview {}", project.name), command, None), cx);
        self.chrome.status = "Starting the preview. The first build takes a few minutes; later ones are quick".into();
    }

    /// Runs the game only to write its catalog of components (`.bevy-editor/components.json`),
    /// as its `.bevy-editor/catalog.toml` asks: no window, and it exits once done.
    pub fn refresh_catalog(&mut self, cx: &mut Context<Self>) {
        let Some(project) = self.workspace.project.clone() else { return };
        if self.task_running() {
            return;
        }
        let mut command = vs_project::cargo(&project.root, &["run", "--color", "never", "--features", "preview"]);
        command.env(vs_graph::components::CATALOG_ENV, "1");
        let label = format!("Catalog of {}", project.name);
        self.start(Task::spawn(label, command, Some(Then::Catalog(project.root.clone()))), cx);
        self.chrome.status = "Building the game to read its components…".into();
    }

    /// Opens the project's catalog config in the system's editor for `.toml` files, writing the
    /// default one first if there's none.
    pub fn edit_catalog(&mut self) {
        let Some(project) = &self.workspace.project else { return };
        let path = project.catalog_config();
        if !path.exists() {
            let written = std::fs::create_dir_all(path.parent().unwrap())
                .and_then(|()| std::fs::write(&path, vs_graph::components::DEFAULT_CONFIG));
            if let Err(e) = written {
                self.chrome.status = format!("Couldn't write {}: {e}", path.display());
                return;
            }
        }
        if let Err(e) = open_with_system(&path) {
            self.chrome.status = format!("Couldn't open {}: {e}", path.display());
        } else {
            self.chrome.status = "Edit the catalog, save it, then Refresh catalog".into();
        }
    }

    /// Exports the project as plain Bevy, builds it in release and packages it in `dist`.
    pub fn build(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(project) = self.workspace.project.clone() else { return };
        if self.task_running() || !self.save_for_run(window) {
            return;
        }
        if let Err(e) = vs_project::export(&project, &project.export_dir()) {
            self.chrome.status = format!("Couldn't export: {e}");
            return;
        }
        let command = vs_project::cargo(&project.export_dir(), &["build", "--release", "--color", "never"]);
        let label = format!("Build {}", project.name);
        self.start(Task::spawn(label, command, Some(Then::Package(project))), cx);
        self.chrome.status = "Exported. Building in release…".into();
    }

    pub fn stop_task(&mut self) {
        if let Some(task) = self.workspace.task.borrow().as_ref() {
            task.stop();
        }
        self.chrome.status = "Stopped".into();
    }

    fn start(&mut self, task: std::io::Result<Task>, cx: &mut Context<Self>) {
        match task {
            Ok(task) => {
                *self.workspace.task.borrow_mut() = Some(task);
                // Show what the run prints.
                self.chrome.show_bottom_panel = true;
                self.chrome.bottom_tab = BottomTab::Output;
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
            Err(e) => self.chrome.status = format!("Couldn't start cargo: {e}"),
        }
    }

    /// Finishes a task that has exited. Returns whether it's still running.
    pub fn poll_task(&mut self) -> bool {
        let mut task = self.workspace.task.borrow_mut();
        let Some(t) = task.as_mut() else { return false };
        let Some(outcome) = t.finished() else { return true };
        let then = t.then.take();
        let label = t.label.clone();
        drop(task);
        self.chrome.status = match (outcome, then) {
            (Outcome::Succeeded, Some(Then::Package(project))) => match vs_project::package(&project) {
                Ok(exe) => format!("Built {}. Its folder is ready to ship", exe.display()),
                Err(e) => format!("The build finished but couldn't be packaged: {e}"),
            },
            // The catalog is global to the editor: only load it if its project is still the open one.
            (Outcome::Succeeded, Some(Then::Catalog(root))) if !self.is_current_project(&root) => {
                format!("The catalog of {} was updated. Open that project to use it", root.display())
            }
            (Outcome::Succeeded, Some(Then::Catalog(root))) => match vs_graph::components::load_project(&root) {
                Ok(n) => format!("Catalog updated: {n} component(s) from the game, besides Bevy's common ones"),
                Err(e) => format!("The game wrote a catalog the editor can't read: {e}"),
            },
            (Outcome::Succeeded, None) => format!("{label} finished"),
            (Outcome::Failed, _) => format!("{label} failed: see the output panel"),
            (Outcome::Stopped, _) => format!("{label} stopped"),
        };
        false
    }

    fn is_current_project(&self, root: &std::path::Path) -> bool {
        self.workspace.project.as_ref().is_some_and(|p| p.root == root)
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
