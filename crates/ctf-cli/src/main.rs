use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use ctf_core::{OperationInput, OperationRegistry, OperationRequest, TaskLimits};
use ctf_launcher::{
    ALL_ID, LauncherStore, bind_javafx_tools, bootstrap_from_th_tools, filter_tools,
    scan_all_environments,
};

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
    Launcher {
        #[command(subcommand)]
        command: LauncherCommands,
    },
}

#[derive(Debug, Subcommand)]
enum LauncherCommands {
    List,
    Search { query: String },
    ScanEnvs,
    ImportAsutools,
    BootstrapTh,
    BindJavafx,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::List { category } => {
            let registry = OperationRegistry::load_default().context("load operation registry")?;
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
            let registry = OperationRegistry::load_default().context("load operation registry")?;
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
        Commands::Launcher { command } => run_launcher_command(command)?,
    }

    Ok(())
}

fn run_launcher_command(command: LauncherCommands) -> Result<()> {
    let store = LauncherStore::new();
    match command {
        LauncherCommands::List => {
            for tool in store.load_tools() {
                println!(
                    "{}\t{}\t{}\t{}",
                    tool.id,
                    tool.name,
                    tool.tool_type.as_str(),
                    tool.path
                );
            }
        }
        LauncherCommands::Search { query } => {
            for tool in filter_tools(&store.load_tools(), ALL_ID, &query) {
                println!(
                    "{}\t{}\t{}\t{}",
                    tool.id,
                    tool.name,
                    tool.tool_type.as_str(),
                    tool.path
                );
            }
        }
        LauncherCommands::ScanEnvs => {
            let mut registry = store.load_environments();
            registry.merge_scanned(scan_all_environments());
            store.save_environments(&registry)?;
            println!("{}", serde_json::to_string_pretty(&registry.environments)?);
        }
        LauncherCommands::ImportAsutools => {
            let summary = store.import_asutools_data()?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "source_dir": summary.source_dir.map(|path| path.display().to_string()),
                    "tools": summary.tools,
                    "categories": summary.categories,
                    "environments": summary.environments,
                    "settings": summary.settings,
                }))?
            );
        }
        LauncherCommands::BootstrapTh => {
            let summary = bootstrap_from_th_tools(&store)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "tools": summary.tools,
                    "categories": summary.categories,
                    "environments": summary.environments,
                    "python_default": summary.python_default,
                    "java_default": summary.java_default,
                }))?
            );
        }
        LauncherCommands::BindJavafx => {
            let mut tools = store.load_tools();
            let environments = store.load_environments();
            let changed = bind_javafx_tools(&mut tools, &environments.environments);
            if changed > 0 {
                store.save_tools(&tools)?;
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "changed": changed,
                }))?
            );
        }
    }
    Ok(())
}
