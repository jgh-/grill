use anyhow::Result;
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
        String::from("\nAmazon Q Commands:\n  (No Q-specific commands available yet)\n")
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
