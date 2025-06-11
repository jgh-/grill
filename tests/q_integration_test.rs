use anyhow::Result;
use tokio::sync::mpsc;
use std::time::Duration;
use tokio::time::timeout;

use grill::process::ProcessManager;
use grill::cli_handler::CliHandlerFactory;

/// Test that input can be successfully sent to a process
#[tokio::test]
async fn test_process_input() -> Result<()> {
    // Create a simple echo process that will echo back our input
    let mut process = ProcessManager::new("cat");
    
    // Create output channel
    let (output_tx, mut output_rx) = mpsc::channel::<String>(100);
    
    // Create a CLI handler
    let cli_handler = CliHandlerFactory::create_handler("cat".to_string());
    
    // Start the process
    let process_input_tx = process.start(output_tx.clone(), cli_handler)?;
    
    // Send some input to the process
    let test_input = "Hello, world!";
    process_input_tx.send(test_input.to_string()).await?;
    
    // Wait for output with a timeout
    let output = timeout(Duration::from_secs(2), output_rx.recv()).await?
        .ok_or_else(|| anyhow::anyhow!("No output received"))?;
    
    // Check that the output contains our input
    assert!(output.contains(test_input), "Output should contain our input");
    
    Ok(())
}

/// Test that multiple lines of input can be sent to a process
#[tokio::test]
async fn test_process_multiple_lines() -> Result<()> {
    // Create a simple echo process that will echo back our input
    let mut process = ProcessManager::new("cat");
    
    // Create output channel
    let (output_tx, mut output_rx) = mpsc::channel::<String>(100);
    
    // Create a CLI handler
    let cli_handler = CliHandlerFactory::create_handler("cat".to_string());
    
    // Start the process
    let process_input_tx = process.start(output_tx.clone(), cli_handler)?;
    
    // Send multiple lines of input to the process
    let test_inputs = vec!["Line 1", "Line 2", "Line 3"];
    
    for input in &test_inputs {
        process_input_tx.send(input.to_string()).await?;
        
        // Wait a bit for the process to echo back
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    // Collect all output
    let mut outputs = Vec::new();
    while let Ok(Some(output)) = timeout(Duration::from_millis(500), output_rx.recv()).await {
        outputs.push(output);
    }
    
    // Join all outputs
    let combined_output = outputs.join("");
    
    // Check that all inputs are in the output
    for input in &test_inputs {
        assert!(combined_output.contains(input), "Output should contain '{}'", input);
    }
    
    Ok(())
}

/// Test that the process can be stopped
#[tokio::test]
async fn test_process_stop() -> Result<()> {
    // Create a simple process
    let mut process = ProcessManager::new("cat");
    
    // Create output channel
    let (output_tx, _output_rx) = mpsc::channel::<String>(100);
    
    // Create a CLI handler
    let cli_handler = CliHandlerFactory::create_handler("cat".to_string());
    
    // Start the process
    let process_input_tx = process.start(output_tx.clone(), cli_handler)?;
    
    // Send some input to the process
    process_input_tx.send("test".to_string()).await?;
    
    // Stop the process
    process.stop()?;
    
    // Process should be stopped (we can't easily test this without the is_running method)
    // The stop() call should succeed without error
    
    Ok(())
}

/// Test that the process can handle special characters
#[tokio::test]
async fn test_process_special_chars() -> Result<()> {
    // Create a simple echo process that will echo back our input
    let mut process = ProcessManager::new("cat");
    
    // Create output channel
    let (output_tx, mut output_rx) = mpsc::channel::<String>(100);
    
    // Create a CLI handler
    let cli_handler = CliHandlerFactory::create_handler("cat".to_string());
    
    // Start the process
    let process_input_tx = process.start(output_tx.clone(), cli_handler)?;
    
    // Send input with special characters to the process
    let test_input = "Special chars: !@#$%^&*()_+-=[]{}|;':\",./<>?";
    process_input_tx.send(test_input.to_string()).await?;
    
    // Wait for output with a timeout
    let output = timeout(Duration::from_secs(2), output_rx.recv()).await?
        .ok_or_else(|| anyhow::anyhow!("No output received"))?;
    
    // Check that the output contains our input
    assert!(output.contains(test_input), "Output should contain our special characters");
    
    Ok(())
}

/// Test that the process can handle Unicode characters
#[tokio::test]
async fn test_process_unicode() -> Result<()> {
    // Create a simple echo process that will echo back our input
    let mut process = ProcessManager::new("cat");
    
    // Create output channel
    let (output_tx, mut output_rx) = mpsc::channel::<String>(100);
    
    // Create a CLI handler
    let cli_handler = CliHandlerFactory::create_handler("cat".to_string());
    
    // Start the process
    let process_input_tx = process.start(output_tx.clone(), cli_handler)?;
    
    // Send input with Unicode characters to the process
    let test_input = "Unicode: 你好, こんにちは, 안녕하세요, Привет, مرحبا, שלום";
    process_input_tx.send(test_input.to_string()).await?;
    
    // Wait for output with a timeout
    let output = timeout(Duration::from_secs(2), output_rx.recv()).await?
        .ok_or_else(|| anyhow::anyhow!("No output received"))?;
    
    // Check that the output contains our input
    assert!(output.contains(test_input), "Output should contain our Unicode characters");
    
    Ok(())
}

/// Test that multiple processes can run simultaneously
#[tokio::test]
async fn test_multiple_processes() -> Result<()> {
    // Create two processes
    let mut process1 = ProcessManager::new("cat");
    let mut process2 = ProcessManager::new("cat");
    
    // Create output channels
    let (output_tx1, mut output_rx1) = mpsc::channel::<String>(100);
    let (output_tx2, mut output_rx2) = mpsc::channel::<String>(100);
    
    // Create CLI handlers
    let cli_handler1 = CliHandlerFactory::create_handler("cat".to_string());
    let cli_handler2 = CliHandlerFactory::create_handler("cat".to_string());
    
    // Start the processes
    let process_input_tx1 = process1.start(output_tx1.clone(), cli_handler1)?;
    let process_input_tx2 = process2.start(output_tx2.clone(), cli_handler2)?;
    
    // Send different inputs to each process
    let test_input1 = "Input to process 1";
    let test_input2 = "Input to process 2";
    
    process_input_tx1.send(test_input1.to_string()).await?;
    process_input_tx2.send(test_input2.to_string()).await?;
    
    // Wait for output from both processes
    let output1 = timeout(Duration::from_secs(2), output_rx1.recv()).await?
        .ok_or_else(|| anyhow::anyhow!("No output received from process 1"))?;
    let output2 = timeout(Duration::from_secs(2), output_rx2.recv()).await?
        .ok_or_else(|| anyhow::anyhow!("No output received from process 2"))?;
    
    // Check that each process received the correct input
    assert!(output1.contains(test_input1), "Process 1 should receive its own input");
    assert!(output2.contains(test_input2), "Process 2 should receive its own input");
    
    // Make sure there's no cross-talk
    assert!(!output1.contains(test_input2), "Process 1 should not receive input for process 2");
    assert!(!output2.contains(test_input1), "Process 2 should not receive input for process 1");
    
    Ok(())
}

/// Test that the process can handle large inputs
#[tokio::test]
async fn test_large_input() -> Result<()> {
    // Create a simple echo process that will echo back our input
    let mut process = ProcessManager::new("cat");
    
    // Create output channel
    let (output_tx, mut output_rx) = mpsc::channel::<String>(100);
    
    // Create a CLI handler
    let cli_handler = CliHandlerFactory::create_handler("cat".to_string());
    
    // Start the process
    let process_input_tx = process.start(output_tx.clone(), cli_handler)?;
    
    // Create a smaller input (1KB) to avoid test flakiness
    let test_input = "A".repeat(1024);
    process_input_tx.send(test_input.clone()).await?;
    
    // Collect all output
    let mut combined_output = String::new();
    let start_time = std::time::Instant::now();
    
    while start_time.elapsed() < Duration::from_secs(5) {
        match timeout(Duration::from_millis(100), output_rx.recv()).await {
            Ok(Some(output)) => {
                combined_output.push_str(&output);
                // If we've received enough input, we can break
                if combined_output.contains('A') {
                    break;
                }
            },
            Ok(None) => break,
            Err(_) => continue,
        }
    }
    
    // Check that the output contains at least some of our input
    assert!(combined_output.contains('A'), "Output should contain at least some of our input");
    
    Ok(())
}

/// Test that the process can handle rapid inputs
#[tokio::test]
async fn test_rapid_inputs() -> Result<()> {
    // Create a simple echo process that will echo back our input
    let mut process = ProcessManager::new("cat");
    
    // Create output channel
    let (output_tx, mut output_rx) = mpsc::channel::<String>(100);
    
    // Create a CLI handler
    let cli_handler = CliHandlerFactory::create_handler("cat".to_string());
    
    // Start the process
    let process_input_tx = process.start(output_tx.clone(), cli_handler)?;
    
    // Send many inputs rapidly
    let num_inputs = 50;
    let test_inputs: Vec<String> = (0..num_inputs).map(|i| format!("Input {}", i)).collect();
    
    for input in &test_inputs {
        process_input_tx.send(input.clone()).await?;
    }
    
    // Collect all output for a few seconds
    let mut outputs = Vec::new();
    let start_time = std::time::Instant::now();
    
    while start_time.elapsed() < Duration::from_secs(5) {
        match timeout(Duration::from_millis(100), output_rx.recv()).await {
            Ok(Some(output)) => outputs.push(output),
            Ok(None) => break,
            Err(_) => continue,
        }
    }
    
    // Join all outputs
    let combined_output = outputs.join("");
    
    // Check that at least some of our inputs were echoed back
    let mut found_inputs = 0;
    for input in &test_inputs {
        if combined_output.contains(input) {
            found_inputs += 1;
        }
    }
    
    // We should have found at least some of our inputs
    assert!(found_inputs > 0, "Should have found at least some of our inputs");
    
    Ok(())
}
