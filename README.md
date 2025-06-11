# Grill 🔥

An interactive CLI tool that augments existing LLM CLIs like Amazon Q with task management and enhanced functionality.

## What is Grill?

Grill wraps around your favorite LLM CLI (like Amazon Q) and adds powerful task management capabilities while preserving the full native experience. Think of it as a transparent layer that enhances your CLI workflow without getting in the way.

## Features

- **Task Management**: Organize your conversations into separate tasks
- **Transparent Integration**: Use all native CLI commands seamlessly
- **Enhanced Help**: Get both grill and native CLI help in one command
- **Character-by-Character Input**: Natural, responsive typing experience
- **Cross-Platform**: Works on macOS, Linux, and Windows

## Installation

### Prerequisites

- Rust (latest stable version)
- Amazon Q CLI or other supported CLI tool

### Build from Source

```bash
git clone <repository-url>
cd grill
cargo build --release
```

The binary will be available at `target/release/grill`.

## Quick Start

### Basic Usage

Start grill with the default Amazon Q CLI:

```bash
grill
```

Start grill with a custom command:

```bash
grill --command "q chat"
```

### Your First Session

1. **Start grill**: Run `grill` in your terminal
2. **See the welcome**: Grill shows startup info and the Q CLI interface
3. **Try the help**: Type `/help` to see all available commands
4. **Chat normally**: Use Q CLI exactly as you normally would
5. **Manage tasks**: Use grill's task commands to organize your work

## Task Management

Grill's main feature is organizing your conversations into tasks. Each task maintains its own context and history.

### Task Commands

| Command | Description |
|---------|-------------|
| `/task` | Show the current task |
| `/task list` | List all available tasks |
| `/task <name>` | Switch to a specific task |
| `/task init <name>` | Create a new task |
| `/task delete <name>` | Delete a task |

### Task Workflow Example

```bash
# Create a new task for a specific project
/task init web-redesign

# Work on your project, chat with Q CLI
> How do I implement responsive design?

# Switch to another task
/task init bug-fixes

# Work on different context
> Help me debug this JavaScript error

# List all your tasks
/task list

# Switch back to previous task
/task web-redesign
```

## Command Reference

### Grill Commands

These commands are handled by grill itself:

- `/help` - Show complete help (grill + native CLI)
- `/task` - Task management commands
- `/quit` - Exit grill

### Native CLI Commands

All other slash commands are passed through to the underlying CLI:

- `/model` - Q CLI model selection
- `/clear` - Q CLI clear conversation  
- `/settings` - Q CLI settings
- Any other native command works as expected

## Configuration

Grill creates a `.grill` directory in your project folder to store:

- Task configurations
- Environment settings
- Session history

### Directory Structure

```
your-project/
├── .grill/
│   ├── config.toml          # Main configuration
│   ├── tasks/               # Task-specific configs
│   │   ├── default.toml
│   │   ├── web-redesign.toml
│   │   └── bug-fixes.toml
│   └── state.md            # Current state info
```

## Advanced Usage

### Custom CLI Commands

Grill can wrap any interactive CLI tool:

```bash
# Use with different tools
grill --command "python -i"
grill --command "node --interactive"
grill --command "mysql -u user -p"
```

### Environment Variables

Set default behavior with environment variables:

```bash
export GRILL_DEFAULT_COMMAND="q chat --model claude-3.7-sonnet"
grill
```

## Tips and Best Practices

### Task Organization

- **Use descriptive task names**: `user-auth-feature` instead of `task1`
- **Create tasks per project/feature**: Keep contexts separate
- **Clean up old tasks**: Use `/task delete` for completed work

### Workflow Integration

- **Start each work session**: Create or switch to relevant task
- **Use native commands freely**: All Q CLI features work normally
- **Leverage task switching**: Jump between different contexts quickly

### Keyboard Shortcuts

- **Ctrl+C**: Quit grill safely
- **Tab**: Tab completion (passed to underlying CLI)
- **Arrow keys**: Command history (passed to underlying CLI)

## Troubleshooting

### Common Issues

**Grill won't start**
- Check that the underlying CLI (like `q chat`) works independently
- Verify Rust installation and build process

**Commands not working**
- Grill commands start with `/` (like `/help`, `/task`)
- Native CLI commands are passed through automatically
- Use `/help` to see all available commands

**Task switching issues**
- Restart grill after switching tasks for full context change
- Check `.grill/` directory permissions

### Getting Help

1. **In-app help**: Type `/help` for complete command reference
2. **Check logs**: Grill outputs debug info to stderr
3. **Verify setup**: Test the underlying CLI independently

## Examples

### Web Development Workflow

```bash
# Start working on frontend
/task init frontend-redesign
> Help me create a responsive navigation bar

# Switch to backend work  
/task init api-development
> How do I implement JWT authentication?

# Back to frontend
/task frontend-redesign
> Continue with the navigation - how do I add mobile menu?
```

### Debugging Session

```bash
# Create debug task
/task init bug-investigation
> I'm getting a 500 error, here's my code...

# Use Q CLI's native commands
/clear
/model claude-3.7-sonnet
> Let's approach this systematically...
```

## Contributing

Grill is designed to be extensible. Future enhancements could include:

- Support for more CLI tools
- Enhanced task metadata
- Session recording and replay
- Team collaboration features

## License

[Add your license information here]

---

**Happy grilling!** 🔥 Enhance your CLI workflow with organized, task-based conversations.
