use anyhow::Result;
use std::io::{self, Write, BufRead};
use tokio::sync::{mpsc, broadcast};
use std::thread;
use std::sync::{Arc, Mutex};

/// Handles input/output between the user and the child process
pub struct IoHandler {
    input_tx: broadcast::Sender<String>,
    output_rx: mpsc::Receiver<String>,
    command_tx: broadcast::Sender<Command>,
    running: Arc<Mutex<bool>>,
}

/// Commands that can be sent to the IoHandler
#[derive(Debug, Clone)]
pub enum Command {
    /// Switch to a different task
    SwitchTask(String),
    /// List all tasks
    ListTasks,
    /// Show current task
    CurrentTask,
    /// Create a new task
    CreateTask(String),
    /// Delete a task
    DeleteTask(String),
    /// Show help
    Help,
    /// Quit the application
    Quit,
}

impl IoHandler {
    /// Create a new IoHandler
    pub fn new() -> (Self, broadcast::Sender<String>, mpsc::Sender<String>, broadcast::Sender<Command>) {
        let (input_tx, _) = broadcast::channel(100);
        let (output_tx, output_rx) = mpsc::channel(100);
        let (command_tx, _) = broadcast::channel(100);
        let running = Arc::new(Mutex::new(true));
        
        let handler = Self {
            input_tx: input_tx.clone(),
            output_rx,
            command_tx: command_tx.clone(),
            running,
        };
        
        (handler, input_tx.clone(), output_tx, command_tx.clone())
    }
    
    /// Start the IO handler
    pub async fn start(&mut self) -> Result<()> {
        // Set up stdin reader
        let input_tx = self.input_tx.clone();
        let command_tx = self.command_tx.clone();
        let running = Arc::clone(&self.running);
        
        thread::spawn(move || -> io::Result<()> {
            let stdin = io::stdin();
            let mut reader = stdin.lock();
            let mut buffer = String::new();
            
            while *running.lock().unwrap() {
                buffer.clear();
                match reader.read_line(&mut buffer) {
                    Ok(0) => {
                        break; // EOF
                    },
                    Ok(_) => {
                        // Check for special commands
                        if buffer.starts_with("/task") {
                            let parts: Vec<&str> = buffer.trim().split_whitespace().collect();
                            
                            match parts.get(1) {
                                Some(&"init") if parts.len() > 2 => {
                                    let task_name = parts[2];
                                    if let Err(e) = command_tx.send(Command::CreateTask(task_name.to_string())) {
                                        eprintln!("Failed to send command: {}", e);
                                    }
                                },
                                Some(&"delete") if parts.len() > 2 => {
                                    let task_name = parts[2];
                                    if let Err(e) = command_tx.send(Command::DeleteTask(task_name.to_string())) {
                                        eprintln!("Failed to send command: {}", e);
                                    }
                                },
                                Some(&"list") => {
                                    if let Err(e) = command_tx.send(Command::ListTasks) {
                                        eprintln!("Failed to send command: {}", e);
                                    }
                                },
                                Some(task_name) => {
                                    if let Err(e) = command_tx.send(Command::SwitchTask(task_name.to_string())) {
                                        eprintln!("Failed to send command: {}", e);
                                    }
                                },
                                None => {
                                    if let Err(e) = command_tx.send(Command::CurrentTask) {
                                        eprintln!("Failed to send command: {}", e);
                                    }
                                },
                            }
                        } else if buffer.trim() == "/quit" {
                            if let Err(e) = command_tx.send(Command::Quit) {
                                eprintln!("Failed to send command: {}", e);
                            }
                            break;
                        } else if buffer.trim() == "/help" {
                            if let Err(e) = command_tx.send(Command::Help) {
                                eprintln!("Failed to send command: {}", e);
                            }
                        } else {
                            // Forward input to the child process
                            if let Err(e) = input_tx.send(buffer.clone()) {
                                eprintln!("Failed to send input: {}", e);
                            }
                        }
                    },
                    Err(e) => {
                        eprintln!("Error reading from stdin: {}", e);
                        break;
                    }
                }
            }
            
            Ok(())
        });
        
        // Set up stdout writer
        let mut stdout = io::stdout();
        
        // Process output directly
        while let Some(output) = self.output_rx.recv().await {
            // Write to stdout
            stdout.write_all(output.as_bytes())?;
            stdout.flush()?;
        }
        
        Ok(())
    }
}
