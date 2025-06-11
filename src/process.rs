use anyhow::{Result, Context};
use portable_pty::{native_pty_system, CommandBuilder, PtyPair, PtySize, Child};
use std::io::{Read, Write, ErrorKind};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tokio::sync::mpsc;
use crate::cli_handler::CliHandler;

/// Manages the child process for the CLI
pub struct ProcessManager {
    pty_pair: Option<PtyPair>,
    child: Option<Box<dyn Child + Send + Sync>>,
    #[allow(dead_code)]
    command: String,
    #[allow(dead_code)]
    args: Vec<String>,
    input_tx: Option<mpsc::Sender<String>>,
    output_tx: Option<mpsc::Sender<String>>,
    running: Arc<Mutex<bool>>,
    writer_running: Arc<Mutex<bool>>,
}

impl ProcessManager {
    /// Create a new process manager
    pub fn new(command: &str) -> Self {
        let parts: Vec<&str> = command.split_whitespace().collect();
        let cmd = parts.first().unwrap_or(&"").to_string();
        let args = parts.iter().skip(1).map(|s| s.to_string()).collect();
        
        Self {
            pty_pair: None,
            child: None,
            command: cmd,
            args,
            input_tx: None,
            output_tx: None,
            running: Arc::new(Mutex::new(false)),
            writer_running: Arc::new(Mutex::new(false)),
        }
    }
    
    /// Start the child process
    pub fn start(&mut self, output_tx: mpsc::Sender<String>, cli_handler: CliHandler) -> Result<mpsc::Sender<String>> {
        let pty_system = native_pty_system();
        
        // Create a new pty
        let pair = pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        }).context("Failed to open pty")?;
        
        // Build the command
        let mut cmd = CommandBuilder::new(&self.command);
        cmd.args(&self.args);
        
        // Spawn the command in the pty
        let child = pair.slave.spawn_command(cmd)
            .context("Failed to spawn command")?;
        
        // Create channels for input/output
        let (input_tx, mut input_rx) = mpsc::channel::<String>(100);
        
        // Store the pty pair and channels
        self.pty_pair = Some(pair);
        self.child = Some(child);
        self.input_tx = Some(input_tx.clone());
        self.output_tx = Some(output_tx.clone());
        
        // Set running state
        let mut running = self.running.lock().unwrap();
        *running = true;
        drop(running);
        
        // Set writer running state
        let mut writer_running = self.writer_running.lock().unwrap();
        *writer_running = true;
        drop(writer_running);
        
        // Clone for thread
        let running = Arc::clone(&self.running);
        let writer_running = Arc::clone(&self.writer_running);
        
        // Set up reader thread with its own buffer
        let mut reader = self.pty_pair.as_ref().unwrap().master.try_clone_reader()
            .context("Failed to clone reader")?;
        
        // Create a separate thread for reading output
        let cli_handler_for_output = cli_handler.clone();
        
        thread::spawn(move || {
            let mut buffer = [0u8; 1024];
            
            // Give the process a moment to start up
            thread::sleep(Duration::from_millis(500));
            
            while *running.lock().unwrap() {
                match reader.read(&mut buffer) {
                    Ok(0) => {
                        // End of file
                        break;
                    },
                    Ok(n) => {
                        // Convert to string and send to output channel
                        let output_str = String::from_utf8_lossy(&buffer[0..n]).to_string();
                        
                        // Intercept output using CLI handler
                        match cli_handler_for_output.intercept_output(output_str) {
                            Ok(Some(modified_output)) => {
                                if let Err(e) = output_tx.blocking_send(modified_output) {
                                    eprintln!("Failed to send output: {}", e);
                                    break;
                                }
                            },
                            Ok(None) => {
                                // Drop this output
                                continue;
                            },
                            Err(e) => {
                                eprintln!("Error intercepting output: {}", e);
                                continue;
                            }
                        }
                    },
                    Err(e) if e.kind() == ErrorKind::WouldBlock => {
                        // No data available, sleep a bit
                        thread::sleep(Duration::from_millis(10));
                    },
                    Err(e) => {
                        eprintln!("Error reading from pty: {}", e);
                        break;
                    }
                }
            }
            
            // Set running to false when the thread exits
            let mut running_lock = running.lock().unwrap();
            *running_lock = false;
        });
        
        // Set up writer thread with its own writer
        let writer = self.pty_pair.as_ref().unwrap().master.take_writer()
            .context("Failed to take writer")?;
        
        // Create a mutex-protected writer
        let writer_mutex = Arc::new(Mutex::new(writer));
        
        // Process input in a separate thread
        thread::spawn(move || {
            while *writer_running.lock().unwrap() {
                // Try to receive input
                match input_rx.blocking_recv() {
                    Some(input) => {
                        // Get a lock on the writer
                        if let Ok(mut writer) = writer_mutex.lock() {
                            // Write the input character/string directly to the process
                            // For character-by-character input, don't modify the input
                            if let Err(e) = writer.write_all(input.as_bytes()) {
                                eprintln!("Failed to write to pty: {}", e);
                                continue;
                            }
                            
                            // Flush the writer to ensure the input is sent immediately
                            if let Err(e) = writer.flush() {
                                eprintln!("Failed to flush pty writer: {}", e);
                                continue;
                            }
                        }
                    },
                    None => {
                        // Channel closed
                        break;
                    },
                }
            }
        });
        
        Ok(input_tx)
    }
    
    /// Stop the child process
    pub fn stop(&mut self) -> Result<()> {
        // Set writer running to false
        let mut writer_running = self.writer_running.lock().unwrap();
        *writer_running = false;
        drop(writer_running);
        
        // Set running to false
        let mut running = self.running.lock().unwrap();
        *running = false;
        drop(running);
        
        // Kill the child process if it's still running
        if let Some(mut child) = self.child.take() {
            if child.try_wait()?.is_none() {
                child.kill()?;
            }
        }
        
        // Drop the pty pair to close the process
        self.pty_pair = None;
        self.input_tx = None;
        self.output_tx = None;
        
        Ok(())
    }
    
}

impl Drop for ProcessManager {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
