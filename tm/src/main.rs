use anyhow::Result;
use clap::{Parser, Subcommand};
use tm_core::{Config, TaskNew, Vault};
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[arg(long)]
    vault: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    Tui,
    Gui,
    Ls { #[arg(short, long)] project: Option<String> },
    Add { title: String, #[arg(long)] project: Option<String>, #[arg(long)] due: Option<String>, #[arg(long, value_delimiter=',')] tags: Option<Vec<String>> },
    Init,
    /// Set status: todo|doing|done
    Status { id: String, value: String },
    /// Shortcut: set status to 'doing'
    Start { id: String },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut cfg = Config::load_default()?;
    if let Some(v) = cli.vault { cfg.vault_path = v; }
    let vault = Vault::new(cfg.clone())?;

    match cli.command.unwrap_or(Cmd::Tui) {
        Cmd::Tui => tm_ui::run_tui(vault)?,
        Cmd::Gui => tm_gui::run_gui(vault)?,
        Cmd::Ls { project } => {
            let tasks = vault.list_tasks(project.as_deref())?;
            for t in tasks { println!("{} [{}] {}", t.id, t.status, t.title); }
        }
        Cmd::Add { title, project, due, tags } => {
            let id = vault.create_task(TaskNew {
                title,
                project: project.unwrap_or_else(|| "inbox".into() ),
                due,
                tags: tags.unwrap_or_default(),
            })?;
            println!("Created task {id}");
        }
        Cmd::Init => {
            vault.init_dirs()?;
            println!("Initialized vault at {}", vault.cfg.vault_path.display());
        }

        Cmd::Status { id, value } => {
        let st = match value.as_str() {
            "todo" => tm_core::Status::Todo,
            "doing" | "in-progress" => tm_core::Status::Doing,
            "done" => tm_core::Status::Done,
            other => anyhow::bail!("unknown status: {other} (use: todo|doing|done)"),
            };
        vault.set_status(&id, st)?;
        }
    Cmd::Start { id } => {
        vault.set_status(&id, tm_core::Status::Doing)?;
    }
}
    Ok(())
}
