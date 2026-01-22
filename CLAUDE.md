# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Linear Tasks is a Rust CLI application for managing recurring tasks (annual, monthly, weekly) via local templates that sync to Linear's project management system. Tasks are organized by day-of-week projects (Monday-Sunday) and daytime blocks (Morning, Work, Afternoon, Evening).

## Build Commands

```bash
# Build development version
cd linear-tasks && cargo build

# Build optimized release version
cd linear-tasks && cargo build --release

# Run the CLI (development)
cd linear-tasks && cargo run -- <args>

# Run tests
cd linear-tasks && cargo test

# Run a specific test
cd linear-tasks && cargo test <test_name>

# Check for compilation errors without building
cd linear-tasks && cargo check

# Format code
cd linear-tasks && cargo fmt

# Run linter
cd linear-tasks && cargo clippy
```

## Architecture

The application follows a modular architecture in `linear-tasks/src/`:

- **main.rs** - CLI entry point using `clap` for argument parsing. Defines all commands (`Templates`, `Generate`, `Add`, `Status`, `Show`, `Dashboard`, `Interactive`) and their handlers.

- **models.rs** - Core domain types:
  - `TimeHorizon` (Annual/Monthly/Weekly/Daily)
  - `DaytimeBlock` (Morning/Work/Afternoon/Evening)
  - Data structures for Linear API responses (Team, Project, Issue, Label, WorkflowState)
  - `AppConfig` for configuration management

- **linear.rs** - Linear GraphQL API client (`LinearClient`). Handles all Linear operations: teams, projects, issues, labels, workflow states. Uses `reqwest` for HTTP and manual GraphQL query construction.

- **templates.rs** - Local template storage system:
  - `TaskTemplate` struct with horizon, days, block, priority
  - `TemplateStore` manages `~/.config/linear-tasks/templates.json`
  - Tracks generation history to prevent duplicates

- **config.rs** - Configuration management for `~/.config/linear-tasks/config.toml`. Loads API key from environment variable (`LINEAR_API_KEY`) or config file.

- **ui.rs** - Terminal UI components using `ratatui`, `tabled`, and `colored` for task display, dashboards, and interactive menus.

## Key Data Flow

1. Templates are stored locally in JSON, not in Linear
2. `generate` command reads templates, matches them to days, and creates Linear issues
3. Generation tracking prevents duplicate issue creation for the same template+date
4. Tasks are assigned to Linear projects named after days (Monday, Tuesday, etc.)
5. Labels indicate time horizon and daytime block

## Configuration

- API Key: `LINEAR_API_KEY` env var or `~/.config/linear-tasks/config.toml`
- Templates: `~/.config/linear-tasks/templates.json`
- Requires Linear team with day-of-week projects and specific labels (created via `setup-labels` command)
