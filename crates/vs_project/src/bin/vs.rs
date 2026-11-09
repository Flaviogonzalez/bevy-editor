//! The editor's project commands, from a terminal.
//!
//! ```sh
//! vs config [<projects folder>]   # show or set where projects are created
//! vs new <name> [--in <folder>]  # create a project there, or in another folder
//! vs export <project>             # write <project>/export, a plain Bevy project
//! vs build <project>              # export, build it in release, package it in <project>/dist
//! ```

use std::path::Path;

use anyhow::{Context, Result, bail};
use vs_project::{Config, Project};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["config"] => {
            println!("projects folder: {}", Config::load().projects_dir.display());
            println!("settings file:   {}", Config::path().display());
        }
        ["config", dir] => {
            let config = Config { projects_dir: dir.into() };
            config.save()?;
            println!("projects folder: {}", config.projects_dir.display());
        }
        ["new", name] | ["new", name, "--in", _] => {
            let dir = match args.get(3) {
                Some(dir) => dir.into(),
                None => Config::load().projects_dir,
            };
            let project = vs_project::create(&dir, name)?;
            println!("created {}", project.root.display());
        }
        ["export", dir] => {
            let project = Project::open(Path::new(dir))?;
            vs_project::export(&project, &project.export_dir())?;
            println!("exported to {}", project.export_dir().display());
        }
        ["build", dir] => {
            let project = Project::open(Path::new(dir))?;
            vs_project::export(&project, &project.export_dir())?;
            let status = vs_project::cargo(&project.export_dir(), &["build", "--release"])
                .status()
                .context("running cargo")?;
            if !status.success() {
                bail!("the release build failed");
            }
            let exe = vs_project::package(&project)?;
            println!("built {}", exe.display());
        }
        _ => bail!("usage: vs config [<dir>] | vs new <name> [--in <dir>] | vs export <project> | vs build <project>"),
    }
    Ok(())
}
