use anyhow::{Result, Context};
use serde::{Serialize, Deserialize};
use std::path::Path;
use std::fs;
use std::collections::HashMap;

/// Global configuration for grill
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// Default CLI to use
    #[serde(default = "default_cli")]
    pub default_cli: String,
    
    /// Available CLIs
    #[serde(default)]
    pub clis: HashMap<String, String>,
    
    /// Hooks to run on task switch
    #[serde(default)]
    pub hooks: HashMap<String, String>,
}

fn default_cli() -> String {
    "q chat".to_string()
}

impl Default for Config {
    fn default() -> Self {
        let mut clis = HashMap::new();
        clis.insert("q".to_string(), "q chat".to_string());
        
        Self {
            default_cli: default_cli(),
            clis,
            hooks: HashMap::new(),
        }
    }
}

impl Config {
    /// Load configuration from a file
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        
        let content = fs::read_to_string(path)
            .context("Failed to read config file")?;
        
        let config: Config = toml::from_str(&content)
            .context("Failed to parse config file")?;
        
        Ok(config)
    }
    
    /// Get the default CLI command
    pub fn get_default_cli(&self) -> &str {
        &self.default_cli
    }
}

/// Task-specific configuration
#[derive(Debug, Serialize, Deserialize)]
pub struct TaskConfig {
    /// CLI to use for this task
    #[serde(default)]
    pub cli: Option<String>,
    
    /// Task-specific hooks
    #[serde(default)]
    pub hooks: HashMap<String, String>,
}

impl Default for TaskConfig {
    fn default() -> Self {
        Self {
            cli: None,
            hooks: HashMap::new(),
        }
    }
}

impl TaskConfig {
    /// Load task configuration from a file
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        
        let content = fs::read_to_string(path)
            .context("Failed to read task config file")?;
        
        let config: TaskConfig = toml::from_str(&content)
            .context("Failed to parse task config file")?;
        
        Ok(config)
    }
    
    /// Get the CLI command for this task
    pub fn get_cli(&self) -> Option<&str> {
        self.cli.as_deref()
    }
}
