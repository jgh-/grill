use anyhow::Result;
use std::sync::{Arc, Mutex};

use crate::environment::Environment;
use crate::process::ProcessManager;
use crate::io::{IoHandler, Command};
use crate::config::Config;
use crate::cli_handler::{CliHandler, CliHandlerFactory};

/// Manages a grill session
pub struct Session {
    environment: Environment,
    process_manager: Option<ProcessManager>,
    current_task: Option<String>,
    running: Arc<Mutex<bool>>,
    cli_handler: Option<CliHandler>,
}

impl Session {
    /// Create a new session
    pub fn new(environment: Environment) -> Self {
        Self {
            environment,
            process_manager: None,
            current_task: None,
            running: Arc::new(Mutex::new(false)),
            cli_handler: None,
        }
    }
    
    /// Start the session
    pub async fn start(&mut self, task_name: Option<String>) -> Result<()> {
        // Set running state
        let mut running = self.running.lock().unwrap();
        *running = true;
        drop(running);
        
        // Get the current task
        let task_name = match task_name {
            Some(name) => name,
            None => self.environment.get_current_task()?,
        };
        
        self.current_task = Some(task_name.clone());
        
        // Get the CLI command for the task
        let cli_command = self.get_cli_command(&task_name)?;
        
        // Create the appropriate CLI handler
        let cli_handler = CliHandlerFactory::create_handler(cli_command.clone());
        
        // Create IO handler and channels
        let (mut io_handler, input_tx, output_tx, command_tx) = IoHandler::new();
        
        // Subscribe to commands
        let mut command_rx = command_tx.subscribe();
        
        // Create process manager
        let mut process_manager = ProcessManager::new(cli_handler.get_command());
        
        // Clone the handler for the process manager
        let cli_handler_clone = cli_handler.clone();
        
        // Start the process
        let process_input_tx = process_manager.start(output_tx.clone(), cli_handler_clone)?;
        
        // Clone the process input sender for the command processing task
        let process_input_tx_for_commands = process_input_tx.clone();
        
        // Store the process manager and CLI handler
        self.process_manager = Some(process_manager);
        self.cli_handler = Some(cli_handler.clone());
        
        // Send welcome message using the CLI handler
        cli_handler.on_start(&task_name, &output_tx)?;
        
        // Create a direct connection between IoHandler and ProcessManager
        let input_tx_clone = input_tx.clone();
        
        // Clone the handler for the input processing task
        let cli_handler_for_input = cli_handler.clone();
        
        // Forward input from IoHandler to ProcessManager
        tokio::spawn(async move {
            let mut input_rx = input_tx_clone.subscribe();
            
            while let Ok(input) = input_rx.recv().await {
                // Intercept input using CLI handler
                match cli_handler_for_input.intercept_input(input.clone()) {
                    Ok(Some(modified_input)) => {
                        // Send the processed input to the child process
                        if let Err(e) = process_input_tx.send(modified_input).await {
                            eprintln!("Failed to forward input to process: {}", e);
                        }
                    },
                    Ok(None) => {
                        // Drop this input
                        continue;
                    },
                    Err(e) => {
                        eprintln!("Error intercepting input: {}", e);
                        // Send the original input as fallback
                        if let Err(e) = process_input_tx.send(input).await {
                            eprintln!("Failed to forward input to process: {}", e);
                        }
                    }
                }
            }
        });
        
        // Set up command processing
        let environment = self.environment.clone();
        let current_task = task_name.clone();
        let output_tx_clone = output_tx.clone();
        let running_clone = Arc::clone(&self.running);
        let process_input_tx_clone = process_input_tx_for_commands;
        
        // Clone the handler for the command processing task
        let cli_handler_for_commands = cli_handler.clone();
        
        // Process commands
        tokio::spawn(async move {
            // Helper function to send carriage return to restore CLI prompt
            async fn send_prompt_restore(input_tx: &tokio::sync::mpsc::Sender<String>) {
                let _ = input_tx.send("\r".to_string()).await;
            }
            
            // Process commands
            while let Ok(command) = command_rx.recv().await {
                eprintln!("Processing command: {:?}", command);
                
                // First, try to handle the command with the CLI-specific handler
                let mut handled = false;
                match cli_handler_for_commands.process_command(command.clone(), &output_tx_clone, &current_task) {
                    Ok(true) => {
                        handled = true;
                    },
                    Ok(false) => {
                        // Command not handled by CLI handler, continue with default handling
                    },
                    Err(e) => {
                        let _ = output_tx_clone.send(format!("\nError processing command: {}\n\n", e)).await;
                        handled = true;
                    }
                }
                
                // If not handled by CLI handler, use default handling
                if !handled {
                    match command {
                        Command::Quit => {
                            let _ = output_tx_clone.send("\nExiting grill...\n".to_string()).await;
                            // Set running to false
                            let mut running = running_clone.lock().unwrap();
                            *running = false;
                            break;
                        },
                        Command::ListTasks => {
                            // List all tasks
                            match environment.list_tasks() {
                                Ok(tasks) => {
                                    let mut output = String::from("\nAvailable tasks:\n");
                                    for task in tasks {
                                        if task == current_task {
                                            output.push_str(&format!("* {} (current)\n", task));
                                        } else {
                                            output.push_str(&format!("  {}\n", task));
                                        }
                                    }
                                    output.push('\n');
                                    let _ = output_tx_clone.send(output).await;
                                },
                                Err(e) => {
                                    let _ = output_tx_clone.send(format!("\nError listing tasks: {}\n", e)).await;
                                }
                            }
                            
                            // Send a carriage return to the CLI to get the prompt back
                            send_prompt_restore(&process_input_tx_clone).await;
                        },
                        Command::CurrentTask => {
                            // Show current task
                            let _ = output_tx_clone.send(format!("\nCurrent task: {}\n\n", current_task)).await;
                            
                            // Send a carriage return to the CLI to get the prompt back
                            send_prompt_restore(&process_input_tx_clone).await;
                        },
                        Command::SwitchTask(task_name) => {
                            // Check if the task exists first
                            match environment.get_task_dir(&task_name) {
                                Ok(task_dir) => {
                                    // Get the CLI command for the new task
                                    let new_cli_command = match Self::get_cli_command_for_task(&environment, &task_name) {
                                        Ok(cmd) => cmd,
                                        Err(e) => {
                                            let _ = output_tx_clone.send(format!("\nError getting CLI command for task '{}': {}\n\n", task_name, e)).await;
                                            send_prompt_restore(&process_input_tx_clone).await;
                                            continue;
                                        }
                                    };
                                    
                                    // Check if the new task uses the same CLI as the current task
                                    if cli_handler_for_commands.can_handle_command(&new_cli_command) {
                                        // Same CLI - we can switch seamlessly
                                        let _ = output_tx_clone.send(format!("\nSwitching to task: {} (seamless switch)\n", task_name)).await;
                                        
                                        // Clear context and switch task
                                        match cli_handler_for_commands.clear_context_and_switch_task(
                                            &task_name,
                                            &task_dir,
                                            &process_input_tx_clone,
                                            &output_tx_clone,
                                        ).await {
                                            Ok(_) => {
                                                // Update the current task in the environment
                                                if let Err(e) = environment.set_current_task(&task_name) {
                                                    let _ = output_tx_clone.send(format!("Warning: Failed to update current task file: {}\n", e)).await;
                                                }
                                                // Note: We don't update current_task variable here since it's used for display only
                                                // The actual task switching is handled by the CLI context clearing
                                            },
                                            Err(e) => {
                                                let _ = output_tx_clone.send(format!("Error switching task context: {}\n\n", e)).await;
                                            }
                                        }
                                    } else {
                                        // Different CLI - requires restart
                                        match environment.set_current_task(&task_name) {
                                            Ok(_) => {
                                                let _ = output_tx_clone.send(format!("\nSwitched to task: {}\n", task_name)).await;
                                                let _ = output_tx_clone.send("Task uses a different CLI. Please restart grill to apply the change.\n\n".to_string()).await;
                                            },
                                            Err(e) => {
                                                let _ = output_tx_clone.send(format!("\nError switching to task '{}': {}\n\n", task_name, e)).await;
                                            }
                                        }
                                    }
                                },
                                Err(e) => {
                                    let _ = output_tx_clone.send(format!("\nError switching to task '{}': {}\n\n", task_name, e)).await;
                                }
                            }
                            
                            // Send a carriage return to the CLI to get the prompt back
                            send_prompt_restore(&process_input_tx_clone).await;
                        },
                        Command::CreateTask(task_name) => {
                            // Create a new task
                            match environment.create_task(&task_name) {
                                Ok(_) => {
                                    let _ = output_tx_clone.send(format!("\nCreated task: {}\n\n", task_name)).await;
                                },
                                Err(e) => {
                                    let _ = output_tx_clone.send(format!("\nError creating task '{}': {}\n\n", task_name, e)).await;
                                }
                            }
                            
                            // Send a carriage return to the CLI to get the prompt back
                            send_prompt_restore(&process_input_tx_clone).await;
                        },
                        Command::DeleteTask(task_name) => {
                            // Delete a task
                            match environment.delete_task(&task_name) {
                                Ok(_) => {
                                    let _ = output_tx_clone.send(format!("\nDeleted task: {}\n\n", task_name)).await;
                                },
                                Err(e) => {
                                    let _ = output_tx_clone.send(format!("\nError deleting task '{}': {}\n\n", task_name, e)).await;
                                }
                            }
                            
                            // Send a carriage return to the CLI to get the prompt back
                            send_prompt_restore(&process_input_tx_clone).await;
                        },
                        Command::Help => {
                            // Show grill help first
                            let mut help_text = get_help_text();
                            
                            // Add CLI-specific help placeholder
                            help_text.push_str(&cli_handler_for_commands.get_help_text());
                            
                            let _ = output_tx_clone.send(help_text).await;
                            
                            // Now send /help to the Q CLI to show its native help
                            let _ = process_input_tx_clone.send("/help\r".to_string()).await;
                        },
                    }
                }
            }
        });
        
        // Start IO handler
        tokio::spawn(async move {
            if let Err(e) = io_handler.start().await {
                eprintln!("Error in IO handler: {}", e);
            }
        });
        
        Ok(())
    }
    
    /// Get the CLI command for a task
    fn get_cli_command(&self, task_name: &str) -> Result<String> {
        // Try to load task-specific config
        let task_dir = self.environment.get_task_dir(task_name)?;
        let config_path = task_dir.join("config.toml");
        
        if config_path.exists() {
            let task_config = crate::config::TaskConfig::load(&config_path)?;
            if let Some(cli) = task_config.get_cli() {
                return Ok(cli.to_string());
            }
        }
        
        // Fall back to global config
        let config_path = self.environment.get_config_path();
        let config = Config::load(&config_path)?;
        Ok(config.get_default_cli().to_string())
    }
    
    /// Get the CLI command for a task (static version for use in async contexts)
    fn get_cli_command_for_task(environment: &Environment, task_name: &str) -> Result<String> {
        // Try to load task-specific config
        let task_dir = environment.get_task_dir(task_name)?;
        let config_path = task_dir.join("config.toml");
        
        if config_path.exists() {
            let task_config = crate::config::TaskConfig::load(&config_path)?;
            if let Some(cli) = task_config.get_cli() {
                return Ok(cli.to_string());
            }
        }
        
        // Fall back to global config
        let config_path = environment.get_config_path();
        let config = Config::load(&config_path)?;
        Ok(config.get_default_cli().to_string())
    }
    
    /// Check if the session is running
    pub fn is_running(&self) -> bool {
        *self.running.lock().unwrap()
    }
}

/// Get help text
fn get_help_text() -> String {
    let mut help = String::from("\nGrill Commands:\n");
    help.push_str("  /task                 Show the current task\n");
    help.push_str("  /task list            List all available tasks\n");
    help.push_str("  /task <n>          Switch to the specified task\n");
    help.push_str("  /task init <n>     Create a new task\n");
    help.push_str("  /task delete <n>   Delete a task\n");
    help.push_str("  /help                 Show this help message\n");
    help.push_str("  /quit                 Exit grill\n\n");
    help
}
