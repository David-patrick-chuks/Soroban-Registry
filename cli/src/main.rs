mod commands;
mod config;
mod export;
mod import;
mod manifest;
use anyhow::Result;
use clap::{Parser, Subcommand};
use config::Network;

const CLI_VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (rustc ",
    env!("RUSTC_VERSION"),
    ")"
);
#[derive(Parser)]
#[command(name = "soroban-registry")]
#[command(version = CLI_VERSION, long_version = CLI_VERSION)]
#[command(about = "CLI tool for the Soroban Contract Registry", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// API URL (defaults to http://localhost:3001)
    #[arg(long, env = "SOROBAN_REGISTRY_API_URL", default_value = "http://localhost:3001")]
    api_url: String,

    /// Network (mainnet, testnet, futurenet)
    #[arg(long, global = true)]
    network: Option<String>,
}
#[derive(Subcommand)]
enum Commands {
    /// Search for contracts
    Search {
        /// Search query
        query: String,

        /// Show only verified contracts
        #[arg(long)]
        verified_only: bool,
    },

    /// Get contract information
    Info {
        /// Contract ID
        contract_id: String,
    },

    /// Publish a contract to the registry
    Publish {
        /// Contract ID (Stellar address)
        #[arg(long)]
        contract_id: String,

        /// Contract name
        #[arg(long)]
        name: String,

        /// Contract description
        #[arg(long)]
        description: Option<String>,

        /// Category
        #[arg(long)]
        category: Option<String>,

        /// Tags (comma-separated)
        #[arg(long)]
        tags: Option<String>,

        /// Publisher Stellar address
        #[arg(long)]
        publisher: String,
    },

    /// List recent contracts
    List {
        /// Number of contracts to show
        #[arg(long, default_value = "10")]
        limit: usize,
    },
    /// Run a contract state migration
    Migrate {
        /// Contract ID
        #[arg(long)]
        contract_id: String,
        /// Path to the migration WASM file
        #[arg(long)]
        wasm: String,
        /// Simulates a failure for testing purposes
        #[arg(long)]
        simulate_fail: bool,
        /// Dry run (do not execute)
        #[arg(long)]
        dry_run: bool,
    },
    Export {
        id: String,
        #[arg(long, default_value = "contract.tar.gz")]
        output: String,
        #[arg(long, default_value = ".")]
        contract_dir: String,
    },
    Import {
        archive: String,
        #[arg(long, default_value = "./imported")]
        output_dir: String,
    },
}
#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Resolve network configuration
    let network = config::resolve_network(cli.network)?;

    match cli.command {
        Commands::Search { query, verified_only } => {
            commands::search(&cli.api_url, &query, network, verified_only).await?;
        }
        Commands::Info { contract_id } => {
            commands::info(&cli.api_url, &contract_id, network).await?;
        }
        Commands::Publish {
            contract_id,
            name,
            description,
            category,
            tags,
            publisher,
        } => {
            let tags_vec = tags
                .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_default();
            commands::publish(
                &cli.api_url,
                &contract_id,
                &name,
                description.as_deref(),
                network,
                category.as_deref(),
                tags_vec,
                &publisher,
            )
            .await?;
        }
        Commands::List { limit } => {
            commands::list(&cli.api_url, limit, network).await?;
        }
        Commands::Migrate {
            contract_id,
            wasm,
            simulate_fail,
            dry_run,
        } => {
            commands::migrate(&cli.api_url, &contract_id, &wasm, simulate_fail, dry_run).await?;
        }
        Commands::Export { id, output, contract_dir } => {
            commands::export(&cli.api_url, &id, &output, &contract_dir).await?;
        }
        Commands::Import { archive, output_dir } => {
            commands::import(&cli.api_url, &archive, network, &output_dir).await?;
        }
    }
    Ok(())
}
}
