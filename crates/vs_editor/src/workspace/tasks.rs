//! Cargo runs started from the editor (preview, build), with their output collected for the
//! side panel.

use std::io::{BufRead, BufReader, Read};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
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

/// How a task ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Succeeded,
    Failed,
    /// Ended by [`Task::stop`], so its exit status says nothing about the run.
    Stopped,
}

/// A cargo process: running while [`Task::finished`] is `None`, then ended with an [`Outcome`].
pub struct Task {
    pub label: String,
    pub lines: Arc<Mutex<Vec<String>>>,
    /// `Some` once the process has exited.
    result: Arc<Mutex<Option<Outcome>>>,
    /// Set by [`Task::stop`] before it kills the process, so the exit reads as stopped.
    stopped: Arc<AtomicBool>,
    child: Arc<Mutex<Child>>,
    /// Runs only after [`Outcome::Succeeded`].
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
        let child = Arc::new(Mutex::new(child));
        let result = Arc::new(Mutex::new(None));
        let stopped = Arc::new(AtomicBool::new(false));
        {
            let (child, result, stopped) = (child.clone(), result.clone(), stopped.clone());
            std::thread::spawn(move || {
                loop {
                    let status = child.lock().unwrap().try_wait();
                    let success = match status {
                        Ok(Some(status)) => status.success(),
                        Ok(None) => {
                            std::thread::sleep(Duration::from_millis(100));
                            continue;
                        }
                        Err(_) => false,
                    };
                    let outcome = if stopped.load(Ordering::SeqCst) {
                        Outcome::Stopped
                    } else if success {
                        Outcome::Succeeded
                    } else {
                        Outcome::Failed
                    };
                    *result.lock().unwrap() = Some(outcome);
                    return;
                }
            });
        }
        Ok(Self { label: label.into(), lines, result, stopped, child, then })
    }

    pub fn finished(&self) -> Option<Outcome> {
        *self.result.lock().unwrap()
    }

    /// Stops the process and everything it started (`cargo run` starts the game).
    pub fn stop(&self) {
        self.stopped.store(true, Ordering::SeqCst);
        let mut child = self.child.lock().unwrap();
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

#[cfg(test)]
mod tests {
    use super::*;

    fn shell(script: &str) -> Command {
        #[cfg(windows)]
        let mut c = {
            let mut c = Command::new("cmd");
            c.arg("/C");
            c
        };
        #[cfg(not(windows))]
        let mut c = {
            let mut c = Command::new("sh");
            c.arg("-c");
            c
        };
        c.arg(script);
        c
    }

    fn wait(task: &Task) -> Outcome {
        for _ in 0..200 {
            if let Some(outcome) = task.finished() {
                return outcome;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        panic!("the task didn't finish");
    }

    #[test]
    fn exit_status_decides_the_outcome() {
        let ok = Task::spawn("ok", shell("exit 0"), None).unwrap();
        assert_eq!(wait(&ok), Outcome::Succeeded);
        let failed = Task::spawn("failed", shell("exit 3"), None).unwrap();
        assert_eq!(wait(&failed), Outcome::Failed);
    }

    #[test]
    fn a_stopped_task_reads_as_stopped() {
        #[cfg(windows)]
        let long = "ping -n 30 127.0.0.1 >NUL";
        #[cfg(not(windows))]
        let long = "sleep 30";
        let task = Task::spawn("long", shell(long), None).unwrap();
        assert_eq!(task.finished(), None);
        task.stop();
        assert_eq!(wait(&task), Outcome::Stopped);
    }
}
