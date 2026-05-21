use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use ctf_core::{OperationInput, OperationRegistry, OperationRequest, TaskLimits};

#[derive(Debug, Parser)]
#[command(name = "ctf-tools", version, about = "Rust + Python CTF toolbox")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    List {
        #[arg(long)]
        category: Option<String>,
    },
    Search {
        query: String,
    },
    Run {
        operation: String,
        #[arg(long)]
        text: Option<String>,
        #[arg(long)]
        file: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let registry = OperationRegistry::load_default().context("load operation registry")?;

    match cli.command {
        Commands::List { category } => {
            for op in registry.operations().iter().filter(|op| {
                category
                    .as_ref()
                    .map(|category| op.category.starts_with(category))
                    .unwrap_or(true)
            }) {
                println!(
                    "{}\t{}\t{}\t{}",
                    op.id, op.name_zh, op.category, op.priority
                );
            }
        }
        Commands::Search { query } => {
            for op in registry.search(&query) {
                println!("{}\t{}\t{}", op.id, op.name_zh, op.name_en);
            }
        }
        Commands::Run {
            operation,
            text,
            file,
        } => {
            let input = match (text, file) {
                (Some(value), None) => OperationInput {
                    kind: "text".to_string(),
                    value,
                },
                (None, Some(value)) => OperationInput {
                    kind: "file".to_string(),
                    value,
                },
                _ => anyhow::bail!("provide exactly one input: --text or --file"),
            };
            let runner = ctf_runner::default_runner().context("initialize operation runner")?;
            let response = runner.run(OperationRequest {
                operation,
                input,
                limits: TaskLimits::default(),
            })?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
    }

    Ok(())
}
