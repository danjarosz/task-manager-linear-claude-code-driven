mod config;
mod linear;
mod models;
mod templates;
mod ui;

use anyhow::{anyhow, Context, Result};
use chrono::{Datelike, Local, NaiveDate, Weekday};
use clap::{Parser, Subcommand};
use colored::Colorize;
use dialoguer::{Confirm, FuzzySelect, Input, MultiSelect, Select};

use crate::linear::LinearClient;
use crate::models::*;
use crate::templates::*;

/// Linear Tasks - A template-based terminal task manager powered by Linear
#[derive(Parser)]
#[command(name = "linear-tasks")]
#[command(author = "Daniel Jarosz")]
#[command(version = "0.2.0")]
#[command(about = "Manage annual, monthly, weekly tasks with templates and Linear integration", long_about = None)]
struct Cli {
    /// Linear API key (can also use LINEAR_API_KEY env var)
    #[arg(long, env = "LINEAR_API_KEY")]
    api_key: Option<String>,

    /// Team name or ID to use
    #[arg(short, long)]
    team: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    // ============ TEMPLATE MANAGEMENT ============
    /// Manage task templates (annual, monthly, weekly)
    Templates {
        #[command(subcommand)]
        action: TemplateAction,
    },

    // ============ GENERATION ============
    /// Generate tasks from templates and push to Linear projects (Monday-Sunday)
    Generate {
        /// Target week: "current", "next", or a date (YYYY-MM-DD)
        #[arg(short, long, default_value = "current")]
        week: String,

        /// Force regeneration even if already generated
        #[arg(short, long)]
        force: bool,

        /// Dry run - show what would be created without creating
        #[arg(long)]
        dry_run: bool,
    },

    // ============ DAILY TASK MANAGEMENT ============
    /// Add a task directly to a specific day's project
    Add {
        /// Day to add to (monday, tuesday, ..., sunday) or "today"
        day: String,

        /// Task title
        title: String,

        /// Daytime block (morning, work, afternoon, evening)
        #[arg(short, long)]
        block: Option<String>,

        /// Priority (1=urgent, 2=high, 3=normal, 4=low)
        #[arg(short, long)]
        priority: Option<i32>,

        /// Due date (YYYY-MM-DD)
        #[arg(long)]
        due: Option<String>,
    },

    /// Change task status (backlog, todo, in progress, done, etc.)
    Status {
        /// Task identifier (e.g., DAY-123)
        id: String,

        /// New status name (e.g., "In Progress", "Done", "Backlog")
        status: Option<String>,
    },

    // ============ VIEWING ============
    /// Show tasks for a specific day or the current week
    Show {
        /// Day to show (monday-sunday, "today", or "week" for all)
        #[arg(default_value = "today")]
        day: String,

        /// Include completed tasks
        #[arg(short, long)]
        all: bool,
    },

    /// Show dashboard summary
    Dashboard,

    // ============ CONFIGURATION ============
    /// Configure the application
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Setup labels in Linear (time horizons + daytime blocks)
    SetupLabels,

    /// Interactive mode
    Interactive,
}

#[derive(Subcommand)]
enum TemplateAction {
    /// List all templates
    List {
        /// Filter by horizon (annual, monthly, weekly)
        #[arg(short = 'H', long)]
        horizon: Option<String>,
    },

    /// Add a new template
    Add {
        /// Task title
        title: String,

        /// Time horizon: annual, monthly, weekly
        #[arg(short = 'H', long)]
        horizon: String,

        /// Days of week (comma-separated): mon,tue,wed,thu,fri,sat,sun
        #[arg(short, long)]
        days: Option<String>,

        /// Daytime block: morning, work, afternoon, evening
        #[arg(short, long)]
        block: Option<String>,

        /// Priority (1-4)
        #[arg(short, long)]
        priority: Option<i32>,

        /// Description
        #[arg(long)]
        description: Option<String>,
    },

    /// Edit a template
    Edit {
        /// Template ID
        id: String,

        /// New title
        #[arg(short, long)]
        title: Option<String>,

        /// New days
        #[arg(short, long)]
        days: Option<String>,

        /// New block
        #[arg(short, long)]
        block: Option<String>,

        /// New priority
        #[arg(short, long)]
        priority: Option<i32>,

        /// Toggle active status
        #[arg(long)]
        toggle_active: bool,
    },

    /// Remove a template
    Remove {
        /// Template ID
        id: String,

        /// Skip confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// Import templates from a file
    Import {
        /// File path (JSON)
        path: String,
    },

    /// Export templates to a file
    Export {
        /// File path (JSON)
        path: String,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Run configuration wizard
    Setup,
    /// Show current configuration
    Show,
    /// Set a configuration value
    Set { key: String, value: String },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Load configuration
    let mut app_config = config::load_config()?;

    // Get API key
    let api_key = cli
        .api_key
        .or_else(|| config::get_api_key(&app_config))
        .ok_or_else(|| {
            anyhow!(
                "No API key provided. Use --api-key, LINEAR_API_KEY env var, or run 'linear-tasks config setup'"
            )
        })?;

    // Create client
    let client = LinearClient::new(&api_key)?;

    // Determine team
    let team_id = if let Some(team_arg) = &cli.team {
        Some(resolve_team_id(&client, team_arg).await?)
    } else if let Some(default_id) = &app_config.default_team_id {
        Some(default_id.clone())
    } else if let Some(default_name) = &app_config.default_team_name {
        Some(resolve_team_id(&client, default_name).await?)
    } else {
        None
    };

    match cli.command {
        Commands::Templates { action } => handle_templates(action).await?,
        Commands::Generate { week, force, dry_run } => {
            let tid = team_id.ok_or_else(|| anyhow!("Team required for generating tasks"))?;
            handle_generate(&client, &tid, &week, force, dry_run).await?
        }
        Commands::Add {
            day,
            title,
            block,
            priority,
            due,
        } => {
            let tid = team_id.ok_or_else(|| anyhow!("Team required for adding tasks"))?;
            handle_add(&client, &tid, &day, &title, block, priority, due).await?
        }
        Commands::Status { id, status } => {
            let tid = team_id.ok_or_else(|| anyhow!("Team required"))?;
            handle_status(&client, &tid, &id, status).await?
        }
        Commands::Show { day, all } => {
            let tid = team_id.ok_or_else(|| anyhow!("Team required"))?;
            handle_show(&client, &tid, &day, all).await?
        }
        Commands::Dashboard => {
            let tid = team_id.ok_or_else(|| anyhow!("Team required"))?;
            handle_dashboard(&client, &tid).await?
        }
        Commands::Config { action } => handle_config(action, &mut app_config).await?,
        Commands::SetupLabels => {
            let tid = team_id.ok_or_else(|| anyhow!("Team required for setting up labels"))?;
            config::ensure_all_labels(&client, &tid).await?;
            ui::display_success("Time horizon and daytime block labels created/verified!");
        }
        Commands::Interactive => {
            let tid = team_id.ok_or_else(|| anyhow!("Team required for interactive mode"))?;
            handle_interactive(&client, &tid).await?
        }
    }

    Ok(())
}

async fn resolve_team_id(client: &LinearClient, name_or_id: &str) -> Result<String> {
    let teams = client.get_teams().await?;

    if let Some(team) = teams.iter().find(|t| t.id == name_or_id) {
        return Ok(team.id.clone());
    }

    if let Some(team) = teams
        .iter()
        .find(|t| t.name.eq_ignore_ascii_case(name_or_id) || t.key.eq_ignore_ascii_case(name_or_id))
    {
        return Ok(team.id.clone());
    }

    Err(anyhow!("Team '{}' not found", name_or_id))
}

// ============ TEMPLATE HANDLERS ============

async fn handle_templates(action: TemplateAction) -> Result<()> {
    let mut store = TemplateStore::load()?;

    match action {
        TemplateAction::List { horizon } => {
            let horizon_filter = horizon.as_ref().and_then(|h| parse_horizon(h));
            ui::display_templates(&store, horizon_filter);
        }

        TemplateAction::Add {
            title,
            horizon,
            days,
            block,
            priority,
            description,
        } => {
            let horizon_enum = parse_horizon(&horizon)
                .ok_or_else(|| anyhow!("Invalid horizon. Use: annual, monthly, weekly"))?;

            let days_vec = days.map(|d| parse_weekdays(&d)).unwrap_or_default();
            let block_enum = block.as_ref().and_then(|b| DaytimeBlock::from_str(b));

            let mut template = TaskTemplate::new(title, horizon_enum, block_enum, days_vec);
            template.priority = priority;
            template.description = description;

            let id = store.add(template)?;
            ui::display_success(&format!("Created template: {}", id));
        }

        TemplateAction::Edit {
            id,
            title,
            days,
            block,
            priority,
            toggle_active,
        } => {
            let template = store
                .find_mut(&id)
                .ok_or_else(|| anyhow!("Template '{}' not found", id))?;

            if let Some(t) = title {
                template.title = t;
            }
            if let Some(d) = days {
                template.days = parse_weekdays(&d);
            }
            if let Some(b) = block {
                template.block = DaytimeBlock::from_str(&b);
            }
            if let Some(p) = priority {
                template.priority = Some(p);
            }
            if toggle_active {
                template.active = !template.active;
            }

            store.save()?;
            ui::display_success(&format!("Updated template: {}", id));
        }

        TemplateAction::Remove { id, force } => {
            if !force {
                let template = store
                    .find(&id)
                    .ok_or_else(|| anyhow!("Template '{}' not found", id))?;

                println!("\nTemplate to remove:");
                println!("  {} - {}", template.id, template.title);

                let confirm = Confirm::new()
                    .with_prompt("Are you sure?")
                    .default(false)
                    .interact()?;

                if !confirm {
                    ui::display_info("Cancelled");
                    return Ok(());
                }
            }

            if store.remove(&id)? {
                ui::display_success(&format!("Removed template: {}", id));
            } else {
                ui::display_error(&format!("Template '{}' not found", id));
            }
        }

        TemplateAction::Import { path } => {
            let contents = std::fs::read_to_string(&path)?;
            let imported: TemplateStore = serde_json::from_str(&contents)?;

            let count = imported.annual.len() + imported.monthly.len() + imported.weekly.len();
            store.annual.extend(imported.annual);
            store.monthly.extend(imported.monthly);
            store.weekly.extend(imported.weekly);
            store.save()?;

            ui::display_success(&format!("Imported {} templates from {}", count, path));
        }

        TemplateAction::Export { path } => {
            let contents = serde_json::to_string_pretty(&store)?;
            std::fs::write(&path, contents)?;
            ui::display_success(&format!("Exported templates to {}", path));
        }
    }

    Ok(())
}

fn parse_horizon(s: &str) -> Option<TimeHorizon> {
    match s.to_lowercase().as_str() {
        "annual" | "yearly" | "roczne" => Some(TimeHorizon::Annual),
        "monthly" | "miesięczne" | "miesieczne" => Some(TimeHorizon::Monthly),
        "weekly" | "tygodniowe" => Some(TimeHorizon::Weekly),
        "daily" | "dzienne" => Some(TimeHorizon::Daily),
        _ => None,
    }
}

// ============ GENERATION HANDLER ============

async fn handle_generate(
    client: &LinearClient,
    team_id: &str,
    week: &str,
    force: bool,
    dry_run: bool,
) -> Result<()> {
    let mut store = TemplateStore::load()?;

    // Determine the week start date (Monday)
    let week_start = parse_week_target(week)?;
    let week_end = week_start + chrono::Duration::days(6);

    println!(
        "\n📅 Generating tasks for week: {} to {}",
        week_start.format("%Y-%m-%d"),
        week_end.format("%Y-%m-%d")
    );

    if dry_run {
        println!("{}", "  (DRY RUN - no tasks will be created)".dimmed());
    }

    // Get projects (Monday-Sunday)
    let projects = client.get_projects(Some(team_id)).await?;
    let labels = client.get_labels(Some(team_id)).await?;

    // Map project names to IDs
    let project_map: std::collections::HashMap<_, _> = projects
        .iter()
        .filter_map(|p| project_name_to_weekday(&p.name).map(|day| (day, p.id.clone())))
        .collect();

    let mut total_created = 0;
    let mut total_skipped = 0;

    // Iterate through each day of the week
    for day_offset in 0..7 {
        let current_date = week_start + chrono::Duration::days(day_offset);
        let weekday = current_date.weekday();
        let project_name = weekday_to_project_name(weekday);

        let project_id = match project_map.get(&weekday) {
            Some(id) => id,
            None => {
                ui::display_warning(&format!("Project '{}' not found, skipping", project_name));
                continue;
            }
        };

        println!(
            "\n  {} {} ({}):",
            weekday_emoji(weekday),
            project_name,
            current_date
        );

        let templates = store.templates_for_day(weekday);

        if templates.is_empty() {
            println!("    No templates for this day");
            continue;
        }

        for template in templates {
            // Check if already generated (unless force)
            if !force && store.was_generated(&template.id, &current_date) {
                println!(
                    "    {} {} {}",
                    "⏭️",
                    template.title.dimmed(),
                    "(already generated)".dimmed()
                );
                total_skipped += 1;
                continue;
            }

            if dry_run {
                println!(
                    "    {} {} [{}] {}",
                    "➕",
                    template.title,
                    template.horizon,
                    template
                        .block
                        .map(|b| format!("({})", b))
                        .unwrap_or_default()
                );
                total_created += 1;
            } else {
                // Build label IDs
                let mut label_ids = vec![];

                // Add horizon label
                let horizon_name = template.horizon.to_string();
                if let Some(label) = labels
                    .iter()
                    .find(|l| l.name.eq_ignore_ascii_case(&horizon_name))
                {
                    label_ids.push(label.id.clone());
                }

                // Add block label
                if let Some(block) = template.block {
                    let block_name = block.to_string();
                    if let Some(label) = labels
                        .iter()
                        .find(|l| l.name.eq_ignore_ascii_case(&block_name))
                    {
                        label_ids.push(label.id.clone());
                    }
                }

                // Create the task
                let input = CreateTaskInput {
                    title: template.title.clone(),
                    description: template.description.clone(),
                    team_id: team_id.to_string(),
                    project_id: Some(project_id.clone()),
                    priority: template.priority,
                    due_date: Some(current_date),
                    label_ids,
                    assignee_id: None,
                };

                match client.create_task(input).await {
                    Ok(task) => {
                        println!(
                            "    {} {} → {}",
                            "✅",
                            template.title,
                            task.identifier.cyan()
                        );
                        store.mark_generated(&template.id, &current_date);
                        total_created += 1;
                    }
                    Err(e) => {
                        println!("    {} {} ({})", "❌", template.title, e);
                    }
                }
            }
        }
    }

    if !dry_run {
        store.save()?;
    }

    println!("\n{}", "─".repeat(50));
    println!(
        "📊 Summary: {} created, {} skipped",
        total_created.to_string().green(),
        total_skipped.to_string().yellow()
    );

    Ok(())
}

fn parse_week_target(s: &str) -> Result<NaiveDate> {
    let today = Local::now().date_naive();

    match s.to_lowercase().as_str() {
        "current" | "this" => {
            // Get Monday of current week
            let days_since_monday = today.weekday().num_days_from_monday();
            Ok(today - chrono::Duration::days(days_since_monday as i64))
        }
        "next" => {
            let days_since_monday = today.weekday().num_days_from_monday();
            let this_monday = today - chrono::Duration::days(days_since_monday as i64);
            Ok(this_monday + chrono::Duration::days(7))
        }
        _ => {
            // Try to parse as date
            NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .context("Invalid date format. Use YYYY-MM-DD, 'current', or 'next'")
        }
    }
}

fn weekday_emoji(day: Weekday) -> &'static str {
    match day {
        Weekday::Mon => "📅",
        Weekday::Tue => "📅",
        Weekday::Wed => "📅",
        Weekday::Thu => "📅",
        Weekday::Fri => "📅",
        Weekday::Sat => "🌅",
        Weekday::Sun => "🌅",
    }
}

// ============ ADD TASK HANDLER ============

async fn handle_add(
    client: &LinearClient,
    team_id: &str,
    day: &str,
    title: &str,
    block: Option<String>,
    priority: Option<i32>,
    due: Option<String>,
) -> Result<()> {
    // Resolve day to weekday
    let (weekday, date) = resolve_day(day)?;
    let project_name = weekday_to_project_name(weekday);

    // Find project
    let projects = client.get_projects(Some(team_id)).await?;
    let project = projects
        .iter()
        .find(|p| p.name.eq_ignore_ascii_case(project_name))
        .ok_or_else(|| anyhow!("Project '{}' not found", project_name))?;

    // Get labels
    let labels = client.get_labels(Some(team_id)).await?;
    let mut label_ids = vec![];

    // Add Daily horizon label
    if let Some(label) = labels.iter().find(|l| l.name.eq_ignore_ascii_case("Daily")) {
        label_ids.push(label.id.clone());
    }

    // Add block label
    if let Some(b) = &block {
        if let Some(block_enum) = DaytimeBlock::from_str(b) {
            let block_name = block_enum.to_string();
            if let Some(label) = labels
                .iter()
                .find(|l| l.name.eq_ignore_ascii_case(&block_name))
            {
                label_ids.push(label.id.clone());
            }
        }
    }

    // Parse due date
    let due_date = if let Some(d) = &due {
        Some(NaiveDate::parse_from_str(d, "%Y-%m-%d")?)
    } else {
        Some(date)
    };

    let spinner = ui::create_spinner("Creating task...");

    let input = CreateTaskInput {
        title: title.to_string(),
        description: None,
        team_id: team_id.to_string(),
        project_id: Some(project.id.clone()),
        priority,
        due_date,
        label_ids,
        assignee_id: None,
    };

    let task = client.create_task(input).await?;

    spinner.finish_and_clear();

    ui::display_success(&format!(
        "Created {} in {} ({})",
        task.identifier, project_name, date
    ));

    Ok(())
}

fn resolve_day(s: &str) -> Result<(Weekday, NaiveDate)> {
    let today = Local::now().date_naive();

    if s.eq_ignore_ascii_case("today") {
        return Ok((today.weekday(), today));
    }

    if s.eq_ignore_ascii_case("tomorrow") {
        let tomorrow = today + chrono::Duration::days(1);
        return Ok((tomorrow.weekday(), tomorrow));
    }

    // Try to parse as weekday
    if let Some(weekday) = parse_weekday(s) {
        // Find the next occurrence of this weekday
        let today_weekday = today.weekday();
        let days_ahead = (weekday.num_days_from_monday() as i64
            - today_weekday.num_days_from_monday() as i64
            + 7)
            % 7;
        let target_date = if days_ahead == 0 {
            today
        } else {
            today + chrono::Duration::days(days_ahead)
        };
        return Ok((weekday, target_date));
    }

    // Try to parse as date
    let date = NaiveDate::parse_from_str(s, "%Y-%m-%d")?;
    Ok((date.weekday(), date))
}

// ============ STATUS HANDLER ============

async fn handle_status(
    client: &LinearClient,
    team_id: &str,
    id: &str,
    status: Option<String>,
) -> Result<()> {
    // Get states
    let states = client.get_states(team_id).await?;

    // Find the task
    let tasks = client.get_tasks(Some(team_id), None, None, true).await?;
    let task = tasks
        .iter()
        .find(|t| t.identifier.eq_ignore_ascii_case(id) || t.id == id)
        .ok_or_else(|| anyhow!("Task '{}' not found", id))?;

    let new_state_id = if let Some(status_name) = status {
        // Find state by name
        states
            .iter()
            .find(|s| s.name.eq_ignore_ascii_case(&status_name))
            .map(|s| s.id.clone())
            .ok_or_else(|| {
                let available: Vec<_> = states.iter().map(|s| s.name.as_str()).collect();
                anyhow!(
                    "State '{}' not found. Available: {}",
                    status_name,
                    available.join(", ")
                )
            })?
    } else {
        // Interactive selection
        let state_names: Vec<_> = states
            .iter()
            .map(|s| format!("{} {}", s.emoji(), s.name))
            .collect();

        let current_idx = states
            .iter()
            .position(|s| s.id == task.state.id)
            .unwrap_or(0);

        let selection = Select::new()
            .with_prompt(&format!("Select new status for {}", task.identifier))
            .items(&state_names)
            .default(current_idx)
            .interact()?;

        states[selection].id.clone()
    };

    let spinner = ui::create_spinner("Updating status...");

    let input = UpdateTaskInput {
        id: task.id.clone(),
        title: None,
        description: None,
        priority: None,
        due_date: None,
        state_id: Some(new_state_id),
        label_ids: None,
        project_id: None,
    };

    let updated = client.update_task(input).await?;

    spinner.finish_and_clear();

    ui::display_success(&format!(
        "{} → {} {}",
        updated.identifier,
        updated.state.emoji(),
        updated.state.name
    ));

    Ok(())
}

// ============ SHOW HANDLER ============

async fn handle_show(
    client: &LinearClient,
    team_id: &str,
    day: &str,
    include_completed: bool,
) -> Result<()> {
    let spinner = ui::create_spinner("Fetching tasks...");

    if day.eq_ignore_ascii_case("week") || day.eq_ignore_ascii_case("all") {
        // Show entire week
        let tasks = client
            .get_tasks(Some(team_id), None, None, include_completed)
            .await?;
        spinner.finish_and_clear();
        ui::display_tasks_by_horizon(&tasks);
    } else {
        // Show specific day
        let (weekday, date) = resolve_day(day)?;
        let project_name = weekday_to_project_name(weekday);

        let projects = client.get_projects(Some(team_id)).await?;
        let project = projects
            .iter()
            .find(|p| p.name.eq_ignore_ascii_case(project_name));

        let tasks = if let Some(proj) = project {
            client
                .get_tasks(Some(team_id), Some(&proj.id), None, include_completed)
                .await?
        } else {
            vec![]
        };

        spinner.finish_and_clear();

        let title = format!(
            "{} {} - {} {}",
            weekday_emoji(weekday),
            project_name,
            date.format("%Y-%m-%d"),
            if date == Local::now().date_naive() {
                "(Today)"
            } else {
                ""
            }
        );

        ui::display_tasks_by_daytime_block(&tasks, &title);
    }

    Ok(())
}

// ============ DASHBOARD HANDLER ============

async fn handle_dashboard(client: &LinearClient, team_id: &str) -> Result<()> {
    let spinner = ui::create_spinner("Loading dashboard...");

    let teams = client.get_teams().await?;
    let team_name = teams
        .iter()
        .find(|t| t.id == team_id)
        .map(|t| t.name.as_str())
        .unwrap_or("Unknown");

    let tasks = client.get_tasks(Some(team_id), None, None, true).await?;
    let store = TemplateStore::load()?;

    spinner.finish_and_clear();

    ui::display_dashboard(&tasks, team_name);

    // Show template summary
    let (annual, monthly, weekly) = store.counts();
    println!("\n{}", "📋 Template Summary:".bold());
    println!("  🎯 Annual:  {} templates", annual);
    println!("  📅 Monthly: {} templates", monthly);
    println!("  📆 Weekly:  {} templates", weekly);

    // Show today's tasks
    let today = Local::now().date_naive();
    let today_weekday = today.weekday();
    let project_name = weekday_to_project_name(today_weekday);

    println!("\n{} Today ({}):", "📌".bold(), project_name);
    let today_tasks: Vec<_> = tasks
        .iter()
        .filter(|t| {
            t.project
                .as_ref()
                .map(|p| p.name.eq_ignore_ascii_case(project_name))
                .unwrap_or(false)
                && t.is_active()
        })
        .take(5)
        .collect();

    if today_tasks.is_empty() {
        println!("  No tasks for today");
    } else {
        for task in today_tasks {
            ui::display_task_compact(task);
        }
    }

    Ok(())
}

// ============ CONFIG HANDLER ============

async fn handle_config(action: ConfigAction, config: &mut AppConfig) -> Result<()> {
    match action {
        ConfigAction::Setup => {
            *config = config::setup_wizard()?;
        }
        ConfigAction::Show => {
            println!("\n📋 Current Configuration:");
            println!("  Config file: {:?}", config::config_path()?);
            println!(
                "  API key: {}",
                if config.linear_api_key.is_empty() {
                    "Not set"
                } else {
                    "********"
                }
            );
            println!(
                "  Default team: {}",
                config.default_team_name.as_deref().unwrap_or("Not set")
            );

            // Show template file location
            let store = TemplateStore::load()?;
            let (a, m, w) = store.counts();
            println!("\n📋 Templates: {} annual, {} monthly, {} weekly", a, m, w);
        }
        ConfigAction::Set { key, value } => {
            match key.as_str() {
                "api_key" => config.linear_api_key = value,
                "default_team" => config.default_team_name = Some(value),
                _ => return Err(anyhow!("Unknown config key: {}", key)),
            }
            config::save_config(config)?;
            ui::display_success(&format!("Set {} successfully", key));
        }
    }
    Ok(())
}

// ============ INTERACTIVE HANDLER ============

async fn handle_interactive(client: &LinearClient, team_id: &str) -> Result<()> {
    loop {
        println!("\n");
        let options = vec![
            "📋 Show Today's Tasks",
            "📆 Show Week",
            "➕ Add Task to Day",
            "🔄 Change Task Status",
            "📝 Manage Templates",
            "🚀 Generate Week from Templates",
            "📊 Dashboard",
            "❌ Exit",
        ];

        let selection = FuzzySelect::new()
            .with_prompt("What would you like to do?")
            .items(&options)
            .default(0)
            .interact()?;

        match selection {
            0 => handle_show(client, team_id, "today", false).await?,
            1 => handle_show(client, team_id, "week", false).await?,
            2 => {
                // Add task interactively
                let days = vec![
                    "Today",
                    "Tomorrow",
                    "Monday",
                    "Tuesday",
                    "Wednesday",
                    "Thursday",
                    "Friday",
                    "Saturday",
                    "Sunday",
                ];
                let day_sel = Select::new()
                    .with_prompt("Which day?")
                    .items(&days)
                    .default(0)
                    .interact()?;

                let day = days[day_sel].to_lowercase();

                let title: String = Input::new().with_prompt("Task title").interact_text()?;

                let blocks = vec![
                    "None",
                    "🌅 Morning",
                    "💼 Work",
                    "☀️ Afternoon",
                    "🌙 Evening",
                ];
                let b_sel = Select::new()
                    .with_prompt("Daytime block")
                    .items(&blocks)
                    .default(0)
                    .interact()?;

                let block = match b_sel {
                    1 => Some("Morning".to_string()),
                    2 => Some("Work".to_string()),
                    3 => Some("Afternoon".to_string()),
                    4 => Some("Evening".to_string()),
                    _ => None,
                };

                handle_add(client, team_id, &day, &title, block, None, None).await?;
            }
            3 => {
                // Change status
                let id: String = Input::new()
                    .with_prompt("Task ID (e.g., DAY-123)")
                    .interact_text()?;

                handle_status(client, team_id, &id, None).await?;
            }
            4 => {
                // Template management submenu
                let template_options = vec![
                    "📋 List Templates",
                    "➕ Add Template",
                    "✏️  Edit Template",
                    "🗑️  Remove Template",
                    "⬅️  Back",
                ];

                let t_sel = Select::new()
                    .with_prompt("Template Management")
                    .items(&template_options)
                    .default(0)
                    .interact()?;

                match t_sel {
                    0 => handle_templates(TemplateAction::List { horizon: None }).await?,
                    1 => {
                        let title: String =
                            Input::new().with_prompt("Template title").interact_text()?;

                        let horizons = vec!["Annual", "Monthly", "Weekly"];
                        let h_sel = Select::new()
                            .with_prompt("Time horizon")
                            .items(&horizons)
                            .default(2)
                            .interact()?;

                        let horizon = horizons[h_sel].to_lowercase();

                        let day_options = vec!["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
                        let day_selections = MultiSelect::new()
                            .with_prompt("Select days (Space to toggle, Enter to confirm)")
                            .items(&day_options)
                            .interact()?;

                        let days = if day_selections.is_empty() {
                            None
                        } else {
                            Some(
                                day_selections
                                    .iter()
                                    .map(|&i| day_options[i])
                                    .collect::<Vec<_>>()
                                    .join(","),
                            )
                        };

                        let blocks = vec!["None", "Morning", "Work", "Afternoon", "Evening"];
                        let b_sel = Select::new()
                            .with_prompt("Daytime block")
                            .items(&blocks)
                            .default(0)
                            .interact()?;

                        let block = if b_sel == 0 {
                            None
                        } else {
                            Some(blocks[b_sel].to_string())
                        };

                        handle_templates(TemplateAction::Add {
                            title,
                            horizon,
                            days,
                            block,
                            priority: None,
                            description: None,
                        })
                        .await?;
                    }
                    2 => {
                        let id: String = Input::new()
                            .with_prompt("Template ID to edit")
                            .interact_text()?;

                        let title: String = Input::new()
                            .with_prompt("New title (leave empty to skip)")
                            .allow_empty(true)
                            .interact_text()?;

                        handle_templates(TemplateAction::Edit {
                            id,
                            title: if title.is_empty() { None } else { Some(title) },
                            days: None,
                            block: None,
                            priority: None,
                            toggle_active: false,
                        })
                        .await?;
                    }
                    3 => {
                        let id: String = Input::new()
                            .with_prompt("Template ID to remove")
                            .interact_text()?;

                        handle_templates(TemplateAction::Remove { id, force: false }).await?;
                    }
                    _ => {}
                }
            }
            5 => {
                // Generate week
                let weeks = vec!["Current Week", "Next Week"];
                let w_sel = Select::new()
                    .with_prompt("Which week?")
                    .items(&weeks)
                    .default(0)
                    .interact()?;

                let week = if w_sel == 0 { "current" } else { "next" };

                let dry_run = Confirm::new()
                    .with_prompt("Dry run first?")
                    .default(true)
                    .interact()?;

                handle_generate(client, team_id, week, false, dry_run).await?;

                if dry_run {
                    let proceed = Confirm::new()
                        .with_prompt("Proceed with actual generation?")
                        .default(true)
                        .interact()?;

                    if proceed {
                        handle_generate(client, team_id, week, false, false).await?;
                    }
                }
            }
            6 => handle_dashboard(client, team_id).await?,
            7 => {
                ui::display_info("Goodbye!");
                break;
            }
            _ => {}
        }
    }

    Ok(())
}
