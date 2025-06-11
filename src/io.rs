use anyhow::Result;
use std::io::{self, Write};
use tokio::sync::{mpsc, broadcast};
use std::thread;
use std::sync::{Arc, Mutex};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};

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
        // Enable raw mode for character-by-character input
        enable_raw_mode()?;
        
        // Set up stdin reader for character-by-character input
        let input_tx = self.input_tx.clone();
        let command_tx = self.command_tx.clone();
        let running = Arc::clone(&self.running);
        
        thread::spawn(move || -> Result<()> {
            let mut command_buffer = String::new();
            let mut in_command_mode = false;
            
            while *running.lock().unwrap() {
                // Check for keyboard events
                if event::poll(std::time::Duration::from_millis(100))? {
                    if let Event::Key(key_event) = event::read()? {
                        match key_event {
                            // Handle Ctrl+C to quit
                            KeyEvent {
                                code: KeyCode::Char('c'),
                                modifiers: KeyModifiers::CONTROL,
                                ..
                            } => {
                                if let Err(e) = command_tx.send(Command::Quit) {
                                    eprintln!("Failed to send quit command: {}", e);
                                }
                                break;
                            }
                            
                            // Handle Enter key
                            KeyEvent {
                                code: KeyCode::Enter,
                                ..
                            } => {
                                if in_command_mode {
                                    // Process the command and show a newline
                                    println!();
                                    Self::process_command_buffer(&command_buffer, &command_tx);
                                    command_buffer.clear();
                                    in_command_mode = false;
                                } else {
                                    // Send carriage return to the process
                                    if let Err(e) = input_tx.send("\r".to_string()) {
                                        eprintln!("Failed to send input: {}", e);
                                    }
                                }
                            }
                            
                            // Handle regular characters
                            KeyEvent {
                                code: KeyCode::Char(c),
                                modifiers: KeyModifiers::NONE,
                                ..
                            } => {
                                if c == '/' && !in_command_mode && command_buffer.is_empty() {
                                    // Start command mode
                                    in_command_mode = true;
                                    command_buffer.push(c);
                                    // Show the slash character
                                    print!("{}", c);
                                    io::stdout().flush().unwrap();
                                } else if in_command_mode {
                                    // Add to command buffer and show character
                                    command_buffer.push(c);
                                    print!("{}", c);
                                    io::stdout().flush().unwrap();
                                } else {
                                    // Send character to process
                                    if let Err(e) = input_tx.send(c.to_string()) {
                                        eprintln!("Failed to send input: {}", e);
                                    }
                                }
                            }
                            
                            // Handle backspace
                            KeyEvent {
                                code: KeyCode::Backspace,
                                ..
                            } => {
                                if in_command_mode {
                                    if command_buffer.pop().is_some() {
                                        // Show backspace visually
                                        print!("\x08 \x08");
                                        io::stdout().flush().unwrap();
                                    }
                                    if command_buffer.is_empty() {
                                        in_command_mode = false;
                                    }
                                } else {
                                    // Send backspace to process
                                    if let Err(e) = input_tx.send("\x08".to_string()) {
                                        eprintln!("Failed to send backspace: {}", e);
                                    }
                                }
                            }
                            
                            // Handle other special keys
                            KeyEvent {
                                code: KeyCode::Tab,
                                ..
                            } => {
                                if !in_command_mode {
                                    if let Err(e) = input_tx.send("\t".to_string()) {
                                        eprintln!("Failed to send tab: {}", e);
                                    }
                                }
                            }
                            
                            // Ignore other keys for now
                            _ => {}
                        }
                    }
                }
            }
            
            // Disable raw mode when exiting
            let _ = disable_raw_mode();
            Ok(())
        });
        
        // Set up stdout writer
        let mut stdout = io::stdout();
        
        // Process output directly
        while let Some(output) = self.output_rx.recv().await {
            // In raw mode, we need to convert \n to \r\n for proper display
            let formatted_output = output.replace('\n', "\r\n");
            
            // Write to stdout
            stdout.write_all(formatted_output.as_bytes())?;
            stdout.flush()?;
        }
        
        // Ensure raw mode is disabled
        let _ = disable_raw_mode();
        
        Ok(())
    }
    
    /// Process command buffer and send appropriate command
    fn process_command_buffer(buffer: &str, command_tx: &broadcast::Sender<Command>) {
        let parts: Vec<&str> = buffer.trim().split_whitespace().collect();
        
        if parts.is_empty() {
            return;
        }
        
        match parts[0] {
            "/task" => {
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
            },
            "/quit" => {
                if let Err(e) = command_tx.send(Command::Quit) {
                    eprintln!("Failed to send command: {}", e);
                }
            },
            "/help" => {
                if let Err(e) = command_tx.send(Command::Help) {
                    eprintln!("Failed to send command: {}", e);
                }
            },
            _ => {
                // Unknown command, ignore
            }
        }
    }
}

impl Drop for IoHandler {
    fn drop(&mut self) {
        // Ensure raw mode is disabled when the handler is dropped
        let _ = disable_raw_mode();
    }
}
