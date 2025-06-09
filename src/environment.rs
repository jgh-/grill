use anyhow::{Result, Context, anyhow};
use std::path::PathBuf;
use std::fs;

/// Represents the grill environment
#[derive(Clone)]
pub struct Environment {
    grill_dir: PathBuf,
    tasks_dir: PathBuf,
    config_file: PathBuf,
    current_task_file: PathBuf,
}

impl Environment {
    /// Create a new environment instance
    pub fn new(root_dir: PathBuf) -> Self {
        let grill_dir = root_dir.join(".grill");
        let tasks_dir = grill_dir.join("tasks");
        let config_file = grill_dir.join("config.toml");
        let current_task_file = grill_dir.join("current_task");
        
        Self {
            grill_dir,
            tasks_dir,
            config_file,
            current_task_file,
        }
    }
    
    /// Initialize a new grill environment
    pub fn init(&self) -> Result<()> {
        // Create .grill directory
        fs::create_dir_all(&self.tasks_dir)
            .context("Failed to create tasks directory")?;
        
        // Create default config file if it doesn't exist
        if !self.config_file.exists() {
            let default_config = r#"# Grill Configuration
default_cli = "q chat"

[clis]
q = "q chat"
"#;
            fs::write(&self.config_file, default_config)
                .context("Failed to write default config file")?;
        }
        
        // Create current_task file if it doesn't exist
        if !self.current_task_file.exists() {
            fs::write(&self.current_task_file, "default")
                .context("Failed to write current task file")?;
            
            // Create default task
            self.create_task("default")?;
        }
        
        Ok(())
    }
    
    /// Check if the environment exists
    pub fn exists(&self) -> bool {
        self.grill_dir.exists() && self.config_file.exists()
    }
    
    /// Create a new task
    pub fn create_task(&self, name: &str) -> Result<()> {
        let task_dir = self.tasks_dir.join(name);
        
        if task_dir.exists() {
            return Err(anyhow!("Task '{}' already exists", name));
        }
        
        fs::create_dir_all(&task_dir)
            .context(format!("Failed to create task directory for '{}'", name))?;
        
        // Create task-specific files
        let instructions_file = task_dir.join("instructions.md");
        let state_file = task_dir.join("state.md");
        let config_file = task_dir.join("config.toml");
        
        fs::write(&instructions_file, "# Task Instructions\n\nAdd your instructions here.\n")
            .context(format!("Failed to create instructions file for task '{}'", name))?;
        
        fs::write(&state_file, "# Task State\n\nTask state will be tracked here.\n")
            .context(format!("Failed to create state file for task '{}'", name))?;
        
        fs::write(&config_file, "# Task Configuration\ncli = \"q chat\"\n")
            .context(format!("Failed to create config file for task '{}'", name))?;
        
        Ok(())
    }
    
    /// Get the current task name
    pub fn get_current_task(&self) -> Result<String> {
        if !self.current_task_file.exists() {
            return Err(anyhow!("No current task set"));
        }
        
        let task = fs::read_to_string(&self.current_task_file)
            .context("Failed to read current task file")?;
        
        Ok(task.trim().to_string())
    }
    
    /// Get the path to a task directory
    pub fn get_task_dir(&self, name: &str) -> Result<PathBuf> {
        let task_dir = self.tasks_dir.join(name);
        
        if !task_dir.exists() {
            return Err(anyhow!("Task '{}' does not exist", name));
        }
        
        Ok(task_dir)
    }
    
    /// Get the path to the config file
    pub fn get_config_path(&self) -> PathBuf {
        self.config_file.clone()
    }
    
    // The following methods are kept for future use but marked as allow(dead_code)
    
    /// Set the current task
    #[allow(dead_code)]
    pub fn set_current_task(&self, name: &str) -> Result<()> {
        let task_dir = self.tasks_dir.join(name);
        
        if !task_dir.exists() {
            return Err(anyhow!("Task '{}' does not exist", name));
        }
        
        fs::write(&self.current_task_file, name)
            .context(format!("Failed to set current task to '{}'", name))?;
        
        Ok(())
    }
    
    /// List all tasks
    #[allow(dead_code)]
    pub fn list_tasks(&self) -> Result<Vec<String>> {
        let mut tasks = Vec::new();
        
        if !self.tasks_dir.exists() {
            return Ok(tasks);
        }
        
        for entry in fs::read_dir(&self.tasks_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    tasks.push(name.to_string());
                }
            }
        }
        
        Ok(tasks)
    }
    
    /// Delete a task
    #[allow(dead_code)]
    pub fn delete_task(&self, name: &str) -> Result<()> {
        let task_dir = self.tasks_dir.join(name);
        
        if !task_dir.exists() {
            return Err(anyhow!("Task '{}' does not exist", name));
        }
        
        // Check if this is the current task
        let current_task = self.get_current_task()?;
        if current_task == name {
            return Err(anyhow!("Cannot delete the current task"));
        }
        
        fs::remove_dir_all(&task_dir)
            .context(format!("Failed to delete task '{}'", name))?;
        
        Ok(())
    }
}
