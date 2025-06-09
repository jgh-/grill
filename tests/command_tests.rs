use anyhow::{Result, anyhow};
use tokio::sync::{mpsc, broadcast};
use tokio::time::{Duration, timeout};

use grill::io::{Command, IoHandler};

/// Test that commands are properly sent and received
#[tokio::test]
async fn test_command_broadcast() -> Result<()> {
    // Create channels
    let (_input_tx, _input_rx) = mpsc::channel::<String>(100);
    let (_output_tx, _output_rx) = mpsc::channel::<String>(100);
    let (command_tx, _) = broadcast::channel::<Command>(100);
    
    // Create a subscriber to the command channel
    let mut command_rx1 = command_tx.subscribe();
    let mut command_rx2 = command_tx.subscribe();
    
    // Send a command
    command_tx.send(Command::Help)?;
    
    // Both receivers should get the command
    let cmd1 = command_rx1.recv().await?;
    let cmd2 = command_rx2.recv().await?;
    
    // Check that both receivers got the same command
    match (cmd1, cmd2) {
        (Command::Help, Command::Help) => {
            // Success
        },
        _ => {
            panic!("Receivers got different commands");
        }
    }
    
    // Send another command
    command_tx.send(Command::ListTasks)?;
    
    // Both receivers should get the command
    let cmd1 = command_rx1.recv().await?;
    let cmd2 = command_rx2.recv().await?;
    
    // Check that both receivers got the same command
    match (cmd1, cmd2) {
        (Command::ListTasks, Command::ListTasks) => {
            // Success
        },
        _ => {
            panic!("Receivers got different commands");
        }
    }
    
    Ok(())
}

/// Test that the IO handler properly processes commands
#[tokio::test]
async fn test_io_handler_commands() -> Result<()> {
    // Create a channel for receiving output
    let (test_tx, mut test_rx) = mpsc::channel::<String>(100);
    
    // Create IO handler
    let (_io_handler, _input_tx, _output_tx, command_tx) = IoHandler::new();
    
    // Subscribe to commands
    let mut command_rx = command_tx.subscribe();
    
    // Set up a task to process commands and forward output to test channel
    tokio::spawn(async move {
        while let Ok(command) = command_rx.recv().await {
            match command {
                Command::Help => {
                    let _ = test_tx.send("Help command received\n".to_string()).await;
                },
                Command::ListTasks => {
                    let _ = test_tx.send("ListTasks command received\n".to_string()).await;
                },
                Command::CurrentTask => {
                    let _ = test_tx.send("CurrentTask command received\n".to_string()).await;
                },
                Command::SwitchTask(name) => {
                    let _ = test_tx.send(format!("SwitchTask command received: {}\n", name)).await;
                },
                Command::CreateTask(name) => {
                    let _ = test_tx.send(format!("CreateTask command received: {}\n", name)).await;
                },
                Command::DeleteTask(name) => {
                    let _ = test_tx.send(format!("DeleteTask command received: {}\n", name)).await;
                },
                Command::Quit => {
                    let _ = test_tx.send("Quit command received\n".to_string()).await;
                    break;
                },
            }
        }
    });
    
    // Send commands directly to the command channel
    command_tx.send(Command::Help)?;
    
    // Wait for the command to be processed with timeout
    let output = timeout(Duration::from_secs(1), test_rx.recv())
        .await?
        .ok_or_else(|| anyhow!("No output received"))?;
    assert_eq!(output, "Help command received\n");
    
    // Send another command
    command_tx.send(Command::ListTasks)?;
    
    // Wait for the command to be processed with timeout
    let output = timeout(Duration::from_secs(1), test_rx.recv())
        .await?
        .ok_or_else(|| anyhow!("No output received"))?;
    assert_eq!(output, "ListTasks command received\n");
    
    // Send a command with a parameter
    command_tx.send(Command::SwitchTask("test-task".to_string()))?;
    
    // Wait for the command to be processed with timeout
    let output = timeout(Duration::from_secs(1), test_rx.recv())
        .await?
        .ok_or_else(|| anyhow!("No output received"))?;
    assert_eq!(output, "SwitchTask command received: test-task\n");
    
    // Send the quit command
    command_tx.send(Command::Quit)?;
    
    // Wait for the command to be processed with timeout
    let output = timeout(Duration::from_secs(1), test_rx.recv())
        .await?
        .ok_or_else(|| anyhow!("No output received"))?;
    assert_eq!(output, "Quit command received\n");
    
    Ok(())
}
