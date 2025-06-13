use anyhow::{Result, Context};
use tokio::sync::mpsc;
use crate::io::Command;

/// Concrete CLI handler type
#[derive(Clone)]
pub enum CliHandler {
    Q(QCliHandler),
    // Add more variants here for other CLI types
}

impl CliHandler {
    pub fn get_command(&self) -> &str {
        match self {
            CliHandler::Q(handler) => handler.get_command(),
        }
    }
    
    pub fn process_command(
        &self, 
        command: Command, 
        output_tx: &mpsc::Sender<String>,
        current_task: &str,
    ) -> Result<bool> {
        match self {
            CliHandler::Q(handler) => handler.process_command(command, output_tx, current_task),
        }
    }
    
    pub fn get_help_text(&self) -> String {
        match self {
            CliHandler::Q(handler) => handler.get_help_text(),
        }
    }
    
    pub fn on_start(
        &self,
        task_name: &str,
        output_tx: &mpsc::Sender<String>,
    ) -> Result<()> {
        match self {
            CliHandler::Q(handler) => handler.on_start(task_name, output_tx),
        }
    }
    
    pub fn intercept_input(&self, input: String) -> Result<Option<String>> {
        match self {
            CliHandler::Q(handler) => handler.intercept_input(input),
        }
    }
    
    pub fn intercept_output(&self, output: String) -> Result<Option<String>> {
        match self {
            CliHandler::Q(handler) => handler.intercept_output(output),
        }
    }
    
    /// Clear the CLI's context and prepare for new task
    pub async fn clear_context_and_switch_task(
        &self,
        new_task_name: &str,
        task_dir: &std::path::Path,
        process_input_tx: &mpsc::Sender<String>,
        output_tx: &mpsc::Sender<String>,
    ) -> Result<()> {
        match self {
            CliHandler::Q(handler) => {
                handler.clear_context_and_switch_task(new_task_name, task_dir, process_input_tx, output_tx).await
            },
        }
    }
    
    /// Check if this CLI handler can handle the given command
    pub fn can_handle_command(&self, command: &str) -> bool {
        match self {
            CliHandler::Q(handler) => handler.can_handle_command(command),
        }
    }
}

/// Handler for Amazon Q CLI
#[derive(Clone)]
pub struct QCliHandler {
    command: String,
}

impl QCliHandler {
    /// Create a new Amazon Q CLI handler
    pub fn new(command: String) -> Self {
        Self { command }
    }
    
    fn get_command(&self) -> &str {
        &self.command
    }
    
    fn process_command(
        &self, 
        _command: Command, 
        _output_tx: &mpsc::Sender<String>,
        _current_task: &str,
    ) -> Result<bool> {
        // Q-specific command handling could go here
        // For now, we don't have any Q-specific commands
        Ok(false) // Not handled, let the default handler take care of it
    }
    
    fn get_help_text(&self) -> String {
        // Q-specific help text
        String::from("\nQ CLI Help (below):\n")
    }
    
    fn on_start(
        &self,
        task_name: &str,
        output_tx: &mpsc::Sender<String>,
    ) -> Result<()> {
        // Send welcome messages without blocking
        let _ = output_tx.try_send(format!("\nStarting grill with task: {}\n", task_name));
        let _ = output_tx.try_send("Type /help for available commands\n\n".to_string());
        Ok(())
    }
    
    fn intercept_input(&self, input: String) -> Result<Option<String>> {
        // For character-by-character input, just pass through
        // No need for complex echo filtering
        Ok(Some(input))
    }
    
    fn intercept_output(&self, output: String) -> Result<Option<String>> {
        // For character-by-character input, just pass through all output
        // The PTY will handle echo naturally
        Ok(Some(output))
    }
    
    /// Clear the CLI's context and switch to a new task
    async fn clear_context_and_switch_task(
        &self,
        new_task_name: &str,
        task_dir: &std::path::Path,
        process_input_tx: &mpsc::Sender<String>,
        output_tx: &mpsc::Sender<String>,
    ) -> Result<()> {
        // Send clear command to Q CLI
        let _ = output_tx.try_send(format!("\nSwitching to task: {}\n", new_task_name));
        let _ = output_tx.try_send("Clearing current context...\n".to_string());
        
        // Send /clear command to Q CLI to clear the conversation
        process_input_tx.send("/clear\r".to_string()).await
            .context("Failed to send clear command to Q CLI")?;
        
        // Q CLI asks for confirmation, send y.
        process_input_tx.send("y\r".to_string()).await
            .context("Failed to send clear command to Q CLI")?;

        // Give the CLI a moment to process the clear command
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        // Load task context files if they exist
        self.load_task_context(new_task_name, task_dir, process_input_tx, output_tx).await?;
        
        let _ = output_tx.try_send(format!("Successfully switched to task: {}\n\n", new_task_name));
        
        Ok(())
    }
    
    /// Load task context into the CLI
    async fn load_task_context(
        &self,
        task_name: &str,
        task_dir: &std::path::Path,
        process_input_tx: &mpsc::Sender<String>,
        output_tx: &mpsc::Sender<String>,
    ) -> Result<()> {
        // Load instructions.md if it exists
        let instructions_path = task_dir.join("instructions.md");
        if instructions_path.exists() {
            match std::fs::read_to_string(&instructions_path) {
                Ok(instructions) => {
                    let _ = output_tx.try_send("Loading task instructions...\n".to_string());
                    
                    // Send the instructions as a message to Q CLI
                    let context_message = format!("Here are the instructions for task '{}': \n\n{}\n", task_name, instructions);
                    process_input_tx.send(format!("{}\r", context_message)).await
                        .context("Failed to send instructions to Q CLI")?;
                    
                    // Give the CLI time to process
                    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                },
                Err(e) => {
                    eprintln!("Warning: Could not read instructions.md: {}", e);
                }
            }
        }
        
        // Load state.md if it exists and has meaningful content
        let state_path = task_dir.join("state.md");
        if state_path.exists() {
            match std::fs::read_to_string(&state_path) {
                Ok(state) => {
                    // Only load state if it's not just the default template
                    if !state.trim().starts_with("# Task State\n\nTask state will be tracked here.") {
                        let _ = output_tx.try_send("Loading task state...\n".to_string());
                        
                        // Send the state as context to Q CLI
                        let context_message = format!("Here is the current state for task '{}': \n\n{}\n", task_name, state);
                        process_input_tx.send(format!("{}\r", context_message)).await
                            .context("Failed to send state to Q CLI")?;
                        
                        // Give the CLI time to process
                        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                    }
                },
                Err(e) => {
                    eprintln!("Warning: Could not read state.md: {}", e);
                }
            }
        }
        
        Ok(())
    }
    
    /// Check if this handler can handle the given command
    fn can_handle_command(&self, command: &str) -> bool {
        // Q CLI handler can handle any command that starts with "q chat"
        command.contains("q chat") || command.contains("q") && command.contains("chat")
    }
}

/// Factory for creating CLI handlers
pub struct CliHandlerFactory;

impl CliHandlerFactory {
    /// Create a CLI handler based on the command
    pub fn create_handler(command: String) -> CliHandler {
        // Determine which handler to use based on the command
        if command.contains("q chat") {
            CliHandler::Q(QCliHandler::new(command))
        } else {
            // Default to Q handler for now
            // In the future, we can add more handlers here
            CliHandler::Q(QCliHandler::new(command))
        }
    }
}
