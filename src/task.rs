// This file contains the Task struct and related functionality.
// It's not currently used in the main application flow but is kept
// for future expansion of the task management capabilities.

use std::path::PathBuf;
use crate::config::TaskConfig;

/// Represents a task in the grill environment
#[allow(dead_code)]
pub struct Task {
    name: String,
    path: PathBuf,
    config: TaskConfig,
}
