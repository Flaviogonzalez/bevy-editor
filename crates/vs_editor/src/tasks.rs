//! Cargo runs started from the editor (preview, build), with their output collected for the
//! side panel.

use std::io::{BufRead, BufReader, Read};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use vs_project::Project;

/// Lines of output kept for the panel.
const KEEP: usize = 400;

/// What to do when a task succeeds.
pub enum Then {
    /// Copy the release build and its assets into the project's `dist` folder.
    Package(Project),
    /// Read the catalog the game just wrote into this project root.
    Catalog(std::path::PathBuf),
}

pub struct Task {
    pub label: String,
    pub lines: Arc<Mutex<Vec<String>>>,
    /// `Some(success)` once the process has exited.
    pub result: Arc<Mutex<Option<bool>>>,
    child: Arc<Mutex<Option<Child>>>,
    pub then: Option<Then>,
}

fn collect(stream: impl Read + Send + 'static, lines: Arc<Mutex<Vec<String>>>) {
    std::thread::spawn(move || {
        for line in BufReader::new(stream).lines().map_while(Result::ok) {
            let mut lines = lines.lock().unwrap();
            lines.push(line);
            let extra = lines.len().saturating_sub(KEEP);
            lines.drain(..extra);
        }
    });
}

impl Task {
    pub fn spawn(label: impl Into<String>, mut command: Command, then: Option<Then>) -> std::io::Result<Self> {
        let mut child = command.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
        let lines = Arc::new(Mutex::new(Vec::new()));
        collect(child.stdout.take().unwrap(), lines.clone());
        collect(child.stderr.take().unwrap(), lines.clone());
        let child = Arc::new(Mutex::new(Some(child)));
        let result = Arc::new(Mutex::new(None));
        {
            let (child, result) = (child.clone(), result.clone());
            std::thread::spawn(move || {
                loop {
                    let status = match child.lock().unwrap().as_mut() {
                        Some(c) => c.try_wait(),
                        None => return,
                    };
                    match status {
                        Ok(Some(status)) => {
                            *result.lock().unwrap() = Some(status.success());
                            return;
                        }
                        Ok(None) => std::thread::sleep(Duration::from_millis(100)),
                        Err(_) => {
                            *result.lock().unwrap() = Some(false);
                            return;
                        }
                    }
                }
            });
        }
        Ok(Self { label: label.into(), lines, result, child, then })
    }

    pub fn finished(&self) -> Option<bool> {
        *self.result.lock().unwrap()
    }

    /// Stops the process and everything it started (`cargo run` starts the game).
    pub fn stop(&self) {
        if let Some(child) = self.child.lock().unwrap().as_mut() {
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                let _ = Command::new("taskkill")
                    .args(["/T", "/F", "/PID", &child.id().to_string()])
                    .creation_flags(0x0800_0000)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
            }
            let _ = child.kill();
        }
    }

    pub fn tail(&self, n: usize) -> Vec<String> {
        let lines = self.lines.lock().unwrap();
        lines[lines.len().saturating_sub(n)..].to_vec()
    }
}

impl Drop for Task {
    fn drop(&mut self) {
        if self.finished().is_none() {
            self.stop();
        }
    }
}
