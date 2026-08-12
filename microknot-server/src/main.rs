use std::path::PathBuf;

use clap::{Parser, Subcommand};
use microknot_core::{Store, Workflow};

#[derive(Parser)]
#[command(name = "microknot", version, about = "A local-first workflow engine")]
struct Cli {
    /// Path to the SQLite database
    #[arg(long, global = true, env = "MICROKNOT_DB", default_value = "./microknot.db")]
    db: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Validate a workflow definition file
    Validate { file: PathBuf },
    /// List stored workflows
    List,
    /// Start the HTTP API server
    Serve {
        /// Address to listen on
        #[arg(long, env = "MICROKNOT_ADDR", default_value = "127.0.0.1:5050")]
        addr: String,
    },
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match &cli.command {
        Command::Validate { file } => run_validate(file),
        Command::List => run_list(&cli.db),
        Command::Serve { addr } => {
            let store = Store::open(&cli.db)?;
            println!("microknot listening on http://{addr}");
            microknot_server::serve(addr, store);
            Ok(())
        }
    }
}

fn run_validate(file: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(file)?;
    let wf: Workflow = serde_json::from_str(&text)?;
    match wf.validate() {
        Ok(()) => {
            println!(
                "'{}' is valid ({} knots, {} connections)",
                wf.name,
                wf.knots.len(),
                wf.connections.len()
            );
            Ok(())
        }
        Err(errors) => {
            for e in &errors {
                eprintln!("- {e}");
            }
            Err(format!(
                "'{}' is invalid ({} error{})",
                wf.name,
                errors.len(),
                if errors.len() == 1 { "" } else { "s" }
            )
            .into())
        }
    }
}

fn run_list(db: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let store = Store::open(db)?;
    let workflows = store.list_workflows()?;
    if workflows.is_empty() {
        println!("no workflows stored in {}", db.display());
        return Ok(());
    }
    println!("{:<40} {:<6} {:<30} KNOTS", "ID", "ACTIVE", "NAME");
    for wf in workflows {
        println!(
            "{:<40} {:<6} {:<30} {}",
            wf.id,
            wf.active,
            wf.name,
            wf.knots.len()
        );
    }
    Ok(())
}
