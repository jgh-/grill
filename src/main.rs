use clap::{Parser, Subcommand};
use anyhow::Result;
use std::env;

mod environment;
mod task;
mod config;
mod process;
mod io;
mod session;
mod cli_handler;

/// Grill - An interactive CLI tool to augment existing LLM CLIs
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new grill environment in the current directory
    Init,
    
    /// Start a grill session with the specified task (or default/last task)
    #[command(trailing_var_arg = true)]
    Start {
        /// Name of the task to start
        #[arg(short, long)]
        task: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    let cli = Cli::parse();
    let current_dir = env::current_dir()?;
    let env = environment::Environment::new(current_dir);
    
    match cli.command {
        Some(Commands::Init) => {
            println!("Initializing grill environment...");
            env.init()?;
            println!("Grill environment initialized successfully.");
            Ok(())
        },
        Some(Commands::Start { task }) => {
            if !env.exists() {
                eprintln!("Error: No grill environment found. Run 'grill init' first.");
                std::process::exit(1);
            }
            
            println!("Starting grill session...");
            start_session(env, task).await?;
            Ok(())
        },
        None => {
            // Default behavior when no subcommand is provided
            if !env.exists() {
                eprintln!("Error: No grill environment found. Run 'grill init' first.");
                std::process::exit(1);
            }
            
            println!("Starting grill session with default settings...");
            start_session(env, None).await?;
            Ok(())
        }
    }
}

async fn start_session(env: environment::Environment, task_name: Option<String>) -> Result<()> {
    // Create a new session
    let mut session = session::Session::new(env);
    
    // Start the session
    session.start(task_name).await?;
    
    // Wait for the session to complete
    while session.is_running() {
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
    
    println!("Session ended.");
    Ok(())
}
