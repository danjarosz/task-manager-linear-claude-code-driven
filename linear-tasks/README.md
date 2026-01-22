# Linear Tasks 📋

A template-based terminal task manager powered by Linear, designed for managing annual, monthly, and weekly recurring tasks with automatic generation to day-based projects.

## Workflow Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    LOCAL TEMPLATE STORAGE                        │
│  ┌──────────┐   ┌───────────┐   ┌──────────┐                    │
│  │ 🎯 Annual │   │ 📅 Monthly │   │ 📆 Weekly │                    │
│  │ Templates │   │ Templates  │   │ Templates │                    │
│  └──────────┘   └───────────┘   └──────────┘                    │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ generate --week current
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    LINEAR PROJECTS (Days)                        │
│  ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐       │
│  │ Mon │ │ Tue │ │ Wed │ │ Thu │ │ Fri │ │ Sat │ │ Sun │       │
│  └─────┘ └─────┘ └─────┘ └─────┘ └─────┘ └─────┘ └─────┘       │
│     │       │       │       │       │       │       │           │
│     └───────┴───────┴───────┴───────┴───────┴───────┘           │
│                    Tasks with daytime blocks:                    │
│              🌅 Morning  💼 Work  ☀️ Afternoon  🌙 Evening        │
└─────────────────────────────────────────────────────────────────┘
```

## Features

- 📝 **Template System** - Store recurring tasks locally (annual, monthly, weekly)
- 🚀 **One-Command Generation** - Push all templates to Linear week projects
- 📅 **Day-Based Projects** - Tasks organized by Monday-Sunday projects
- 🌅 **Daytime Blocks** - Morning, Work, Afternoon, Evening organization
- 🔄 **Status Management** - Change task states (Backlog, Todo, In Progress, Done)
- ➕ **Direct Task Addition** - Add one-off tasks to specific days
- 📊 **Dashboard** - Overview of tasks and templates

## Installation

### Prerequisites

- Rust 1.70+ (install from [rustup.rs](https://rustup.rs))
- A Linear account with API access
- Linear team with day-of-week projects (Monday, Tuesday, ..., Sunday)

### Build from Source

```bash
cd linear-tasks
cargo build --release

# The binary will be at ./target/release/linear-tasks
```

### Get Your Linear API Key

1. Go to Linear Settings → API → Personal API keys
2. Create a new key
3. Copy the key (starts with `lin_api_`)

## Quick Start

```bash
# 1. Configure
export LINEAR_API_KEY="lin_api_your_key"
linear-tasks config setup

# 2. Setup labels in Linear
linear-tasks --team "Day-to-day operations" setup-labels

# 3. Add templates
linear-tasks templates add "Morning meditation" -H weekly -d mon,tue,wed,thu,fri -b morning
linear-tasks templates add "Weekly review" -H weekly -d fri -b afternoon
linear-tasks templates add "Monthly planning" -H monthly -d mon -b morning

# 4. Generate tasks for the week
linear-tasks --team "Day-to-day operations" generate --week current

# 5. View today's tasks
linear-tasks show today
```

## Template Management

Templates are stored locally in `~/.config/linear-tasks/templates.json`.

### Add Templates

```bash
# Weekly task on specific days
linear-tasks templates add "Daily standup" -H weekly -d mon,tue,wed,thu,fri -b work

# Weekly task on all days (no -d flag)
linear-tasks templates add "Evening reflection" -H weekly -b evening

# Monthly task
linear-tasks templates add "Invoice review" -H monthly -d mon -b work -p 2

# Annual task
linear-tasks templates add "Tax preparation" -H annual -d mon -b work -p 1
```

### Options

| Flag | Description |
|------|-------------|
| `-H, --horizon` | Time horizon: `annual`, `monthly`, `weekly` |
| `-d, --days` | Days of week: `mon,tue,wed,thu,fri,sat,sun` |
| `-b, --block` | Daytime block: `morning`, `work`, `afternoon`, `evening` |
| `-p, --priority` | Priority: 1=Urgent, 2=High, 3=Normal, 4=Low |
| `--description` | Task description |

### List Templates

```bash
# List all templates
linear-tasks templates list

# Filter by horizon
linear-tasks templates list -H weekly
linear-tasks templates list -H monthly
linear-tasks templates list -H annual
```

### Edit Templates

```bash
# Change title
linear-tasks templates edit abc123 --title "New title"

# Change days
linear-tasks templates edit abc123 --days mon,wed,fri

# Change block
linear-tasks templates edit abc123 --block afternoon

# Toggle active/inactive
linear-tasks templates edit abc123 --toggle-active
```

### Remove Templates

```bash
# With confirmation
linear-tasks templates remove abc123

# Force remove
linear-tasks templates remove abc123 --force
```

### Import/Export

```bash
# Export templates to file
linear-tasks templates export ~/my-templates.json

# Import templates from file
linear-tasks templates import ~/my-templates.json
```

## Generate Tasks

The `generate` command creates Linear issues from your templates:

```bash
# Generate for current week
linear-tasks generate --week current

# Generate for next week
linear-tasks generate --week next

# Generate for specific week (by Monday's date)
linear-tasks generate --week 2024-01-29

# Dry run (preview without creating)
linear-tasks generate --week current --dry-run

# Force regeneration (even if already generated)
linear-tasks generate --week current --force
```

### How Generation Works

1. Reads all active templates from local storage
2. For each day (Monday-Sunday):
   - Finds the corresponding Linear project
   - Matches templates to that day based on `days` field
   - Creates Linear issues with appropriate labels
3. Tracks which templates were generated for which dates (prevents duplicates)

## Daily Task Management

### Add One-Off Tasks

```bash
# Add task to today
linear-tasks add today "Unexpected meeting" --block work

# Add task to specific day
linear-tasks add monday "Team sync" --block morning

# Add with priority
linear-tasks add friday "Urgent bugfix" --block work --priority 1

# Add with due date
linear-tasks add wednesday "Report" --due 2024-02-01
```

### Change Task Status

```bash
# Interactive status selection
linear-tasks status DAY-123

# Direct status change
linear-tasks status DAY-123 "In Progress"
linear-tasks status DAY-123 "Done"
linear-tasks status DAY-123 "Backlog"
```

### View Tasks

```bash
# Today's tasks
linear-tasks show today

# Tomorrow's tasks
linear-tasks show tomorrow

# Specific day
linear-tasks show monday
linear-tasks show friday

# Entire week
linear-tasks show week

# Include completed tasks
linear-tasks show today --all
```

## Dashboard

```bash
linear-tasks dashboard
```

Shows:
- Task statistics (total, active, completed, overdue)
- Template summary (annual, monthly, weekly counts)
- Today's tasks

## Interactive Mode

```bash
linear-tasks interactive
```

Menu-driven interface for all operations:
- Show Today's Tasks
- Show Week
- Add Task to Day
- Change Task Status
- Manage Templates (submenu)
- Generate Week from Templates
- Dashboard

## Configuration

### Setup Wizard

```bash
linear-tasks config setup
```

### View Configuration

```bash
linear-tasks config show
```

### Set Values

```bash
linear-tasks config set api_key "lin_api_..."
linear-tasks config set default_team "Day-to-day operations"
```

### Configuration Files

- Config: `~/.config/linear-tasks/config.toml`
- Templates: `~/.config/linear-tasks/templates.json`

## Linear Setup Requirements

### Required Projects

Your Linear team needs projects for each day:
- Monday
- Tuesday
- Wednesday
- Thursday
- Friday
- Saturday
- Sunday

### Required Labels

Run `setup-labels` to create:

**Time Horizons:**
- Annual (🎯)
- Monthly (📅)
- Weekly (📆)
- Daily (📌)

**Daytime Blocks:**
- Morning (🌅) - 06:00-09:00
- Work (💼) - 09:00-17:00
- Afternoon (☀️) - 17:00-20:00
- Evening (🌙) - 20:00-23:00

## Example Weekly Workflow

### Sunday Evening (Plan the Week)

```bash
# Review current templates
linear-tasks templates list

# Add any new recurring tasks
linear-tasks templates add "Project review" -H weekly -d fri -b afternoon

# Generate tasks for the coming week
linear-tasks generate --week next --dry-run
linear-tasks generate --week next
```

### Daily Usage

```bash
# Morning: Check today's tasks
linear-tasks show today

# Start working on a task
linear-tasks status DAY-101 "In Progress"

# Add unexpected task
linear-tasks add today "Urgent client call" --block work --priority 1

# Complete tasks
linear-tasks status DAY-101 "Done"
linear-tasks status DAY-102 "Done"

# Evening: Review
linear-tasks dashboard
```

## Project Structure

```
linear-tasks/
├── Cargo.toml           # Dependencies
├── README.md            # This file
└── src/
    ├── main.rs          # CLI commands and handlers
    ├── models.rs        # Data structures
    ├── linear.rs        # Linear GraphQL API client
    ├── templates.rs     # Local template storage
    ├── ui.rs            # Terminal UI
    └── config.rs        # Configuration management
```

## License

MIT License - Feel free to use and modify as needed.

## Author

Created for Programming Craftsman Daniel Jarosz
