use anyhow::Result;
use clap::Parser;

mod commands;
mod config;
mod error;
mod messages;
mod server;
mod transport;

use commands::{create, dev, serve};

#[derive(Parser)]
#[command(name = "lp-cli")]
#[command(about = "LightPlayer CLI - Server and client modes")]
enum Cli {
    /// Run server from a directory
    Serve {
        /// Server directory (defaults to current directory)
        dir: Option<std::path::PathBuf>,
        /// Initialize server directory (create server.json if missing)
        #[arg(long)]
        init: bool,
        /// Use in-memory filesystem instead of disk
        #[arg(long)]
        memory: bool,
    },
    /// Connect to server and sync local project
    Dev {
        /// Host specifier (e.g., ws://localhost:2812/). If not provided, uses in-memory server.
        host: Option<String>,
        /// Project directory (defaults to current directory)
        dir: Option<std::path::PathBuf>,
        /// Push local project to server (default: true)
        #[arg(long, default_value = "true")]
        push: bool,
    },
    /// Create a new project
    Create {
        /// Project directory
        dir: std::path::PathBuf,
        /// Project name (defaults to directory name)
        #[arg(long)]
        name: Option<String>,
        /// Project UID (auto-generated if not provided)
        #[arg(long)]
        uid: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli {
        Cli::Serve { dir, init, memory } => {
            serve::handle_serve(serve::ServeArgs { dir, init, memory })
        }
        Cli::Dev { host, dir, push } => dev::handle_dev(dev::DevArgs { host, dir, push }),
        Cli::Create { dir, name, uid } => {
            create::handle_create(create::CreateArgs { dir, name, uid })
        }
    }
}
