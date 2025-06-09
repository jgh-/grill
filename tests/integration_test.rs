use anyhow::{Result, anyhow};
use tokio::sync::{mpsc, broadcast};
use std::time::Duration;
use std::sync::{Arc, Mutex};
use tokio::time::timeout;

use grill::environment::Environment;
use grill::io::Command;

#[tokio::test]
async fn test_command_processing() -> Result<()> {
    // Create a temporary directory for testing
    let temp_dir = tempfile::tempdir()?;
    let env = Environment::new(temp_dir.path().to_path_buf());
    
    // Initialize the environment
    env.init()?;
    
    // Create channels for testing
    let (output_tx, mut output_rx) = mpsc::channel::<String>(100);
    let (command_tx, _) = broadcast::channel::<Command>(100);
    let mut command_rx = command_tx.subscribe();
    
    // Set up a task to process commands
    let running = Arc::new(Mutex::new(true));
    let env_clone = env.clone();
    
    tokio::spawn(async move {
        let current_task = "default".to_string();
        
        while let Ok(command) = command_rx.recv().await {
            match command {
                Command::Help => {
                    let help_text = "\nGrill Commands:\n  /task                 Show the current task\n  /task list            List all available tasks\n  /task <name>          Switch to the specified task\n  /task init <name>     Create a new task\n  /task delete <name>   Delete a task\n  /help                 Show this help message\n  /quit                 Exit grill\n\n";
                    let _ = output_tx.send(help_text.to_string()).await;
                },
                Command::CurrentTask => {
                    let _ = output_tx.send(format!("Current task: {}\n", current_task)).await;
                },
                Command::ListTasks => {
                    match env_clone.list_tasks() {
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
                            let _ = output_tx.send(output).await;
                        },
                        Err(e) => {
                            let _ = output_tx.send(format!("Error listing tasks: {}\n", e)).await;
                        }
                    }
                },
                Command::CreateTask(name) => {
                    match env_clone.create_task(&name) {
                        Ok(_) => {
                            let _ = output_tx.send(format!("Created task: {}\n", name)).await;
                        },
                        Err(e) => {
                            let _ = output_tx.send(format!("Error creating task '{}': {}\n", name, e)).await;
                        }
                    }
                },
                Command::SwitchTask(name) => {
                    match env_clone.set_current_task(&name) {
                        Ok(_) => {
                            let _ = output_tx.send(format!("Switched to task: {}\n", name)).await;
                            let _ = output_tx.send("Please restart grill to apply the change.\n".to_string()).await;
                        },
                        Err(e) => {
                            let _ = output_tx.send(format!("Error switching to task '{}': {}\n", name, e)).await;
                        }
                    }
                },
                Command::DeleteTask(name) => {
                    match env_clone.delete_task(&name) {
                        Ok(_) => {
                            let _ = output_tx.send(format!("Deleted task: {}\n", name)).await;
                        },
                        Err(e) => {
                            let _ = output_tx.send(format!("Error deleting task '{}': {}\n", name, e)).await;
                        }
                    }
                },
                Command::Quit => {
                    let _ = output_tx.send("Exiting grill...\n".to_string()).await;
                    let mut r = running.lock().unwrap();
                    *r = false;
                    break;
                },
            }
        }
    });
    
    // Send test commands
    command_tx.send(Command::Help)?;
    
    // Wait for and verify the response
    let response = timeout(Duration::from_secs(1), output_rx.recv()).await?
        .ok_or_else(|| anyhow!("No output received"))?;
    assert!(response.contains("/task"));
    assert!(response.contains("/help"));
    assert!(response.contains("/quit"));
    
    // Test current task command
    command_tx.send(Command::CurrentTask)?;
    let response = timeout(Duration::from_secs(1), output_rx.recv()).await?
        .ok_or_else(|| anyhow!("No output received"))?;
    assert!(response.contains("Current task: default"));
    
    // Test list tasks command
    command_tx.send(Command::ListTasks)?;
    let response = timeout(Duration::from_secs(1), output_rx.recv()).await?
        .ok_or_else(|| anyhow!("No output received"))?;
    assert!(response.contains("Available tasks:"));
    assert!(response.contains("* default (current)"));
    
    // Test create task command
    command_tx.send(Command::CreateTask("test-task".to_string()))?;
    let response = timeout(Duration::from_secs(1), output_rx.recv()).await?
        .ok_or_else(|| anyhow!("No output received"))?;
    assert_eq!(response, "Created task: test-task\n");
    
    // Test switch task command
    command_tx.send(Command::SwitchTask("test-task".to_string()))?;
    let response = timeout(Duration::from_secs(1), output_rx.recv()).await?
        .ok_or_else(|| anyhow!("No output received"))?;
    assert_eq!(response, "Switched to task: test-task\n");
    
    // Check for the second part of the switch task response
    let response = timeout(Duration::from_secs(1), output_rx.recv()).await?
        .ok_or_else(|| anyhow!("No output received"))?;
    assert_eq!(response, "Please restart grill to apply the change.\n");
    
    // Test quit command
    command_tx.send(Command::Quit)?;
    let response = timeout(Duration::from_secs(1), output_rx.recv()).await?
        .ok_or_else(|| anyhow!("No output received"))?;
    assert_eq!(response, "Exiting grill...\n");
    
    Ok(())
}
