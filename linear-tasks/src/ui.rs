use colored::*;
use console::Term;
use tabled::{
    builder::Builder,
    settings::{
        object::{Columns, Rows},
        themes::Colorization,
        Alignment, Color, Modify, Panel, Style, Width,
    },
    Table,
};

use crate::models::*;

/// Terminal width detection
fn term_width() -> usize {
    Term::stdout().size().1 as usize
}

/// Truncate string to fit width with ellipsis
fn truncate(s: &str, max_width: usize) -> String {
    if s.len() <= max_width {
        s.to_string()
    } else if max_width <= 3 {
        ".".repeat(max_width)
    } else {
        format!("{}...", &s[..max_width - 3])
    }
}

/// Format a priority for display
fn format_priority(priority: i32) -> String {
    match priority {
        1 => "🔴 Urgent".to_string(),
        2 => "🟠 High".to_string(),
        3 => "🟡 Normal".to_string(),
        4 => "🟢 Low".to_string(),
        _ => "—".to_string(),
    }
}

/// Format state with emoji
fn format_state(state: &TaskState) -> String {
    let emoji = match state.state_type.as_str() {
        "backlog" => "📥",
        "unstarted" => "⚪",
        "started" => "🔵",
        "completed" => "✅",
        "canceled" => "❌",
        _ => "❓",
    };
    format!("{} {}", emoji, state.name)
}

/// Format due date with urgency coloring
fn format_due_date(due: &Option<chrono::NaiveDate>) -> String {
    match due {
        None => "—".to_string(),
        Some(date) => {
            let today = chrono::Local::now().date_naive();
            let days_until = (*date - today).num_days();

            let formatted = date.format("%Y-%m-%d").to_string();

            if days_until < 0 {
                format!("⚠️  {} (overdue)", formatted)
            } else if days_until == 0 {
                format!("🔥 {} (today)", formatted)
            } else if days_until <= 3 {
                format!("⏰ {} ({}d)", formatted, days_until)
            } else {
                formatted
            }
        }
    }
}

/// Format labels as comma-separated colored badges
fn format_labels(labels: &[Label]) -> String {
    if labels.is_empty() {
        return "—".to_string();
    }

    labels
        .iter()
        .map(|l| format!("[{}]", l.name))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Display tasks in a beautiful table
pub fn display_tasks_table(tasks: &[Task], title: &str) {
    if tasks.is_empty() {
        println!("\n{}", "No tasks found.".dimmed());
        return;
    }

    let width = term_width().min(180);

    let mut builder = Builder::default();

    // Header
    builder.push_record(["ID", "Title", "Priority", "State", "Due Date", "Labels", "Project"]);

    // Rows
    for task in tasks {
        let title_width = (width as f32 * 0.25) as usize;
        let project_name = task
            .project
            .as_ref()
            .map(|p| p.name.as_str())
            .unwrap_or("—");

        builder.push_record([
            &task.identifier,
            &truncate(&task.title, title_width),
            &format_priority(task.priority),
            &format_state(&task.state),
            &format_due_date(&task.due_date),
            &truncate(&format_labels(&task.labels), 20),
            &truncate(project_name, 15),
        ]);
    }

    let mut table = builder.build();
    table
        .with(Style::rounded())
        .with(Panel::header(title))
        .with(Modify::new(Rows::first()).with(Alignment::center()))
        .with(Modify::new(Columns::single(0)).with(Alignment::left()))
        .with(Modify::new(Columns::single(2)).with(Alignment::center()))
        .with(Modify::new(Columns::single(3)).with(Alignment::center()));

    println!("\n{}", table);
    println!(
        "{}",
        format!("Total: {} task(s)", tasks.len()).dimmed()
    );
}

/// Display tasks grouped by time horizon
pub fn display_tasks_by_horizon(tasks: &[Task]) {
    let mut annual: Vec<&Task> = vec![];
    let mut monthly: Vec<&Task> = vec![];
    let mut weekly: Vec<&Task> = vec![];
    let mut daily: Vec<&Task> = vec![];
    let mut other: Vec<&Task> = vec![];

    for task in tasks {
        match task.time_horizon() {
            Some(TimeHorizon::Annual) => annual.push(task),
            Some(TimeHorizon::Monthly) => monthly.push(task),
            Some(TimeHorizon::Weekly) => weekly.push(task),
            Some(TimeHorizon::Daily) => daily.push(task),
            None => other.push(task),
        }
    }

    if !annual.is_empty() {
        display_horizon_section_with_blocks(&annual, TimeHorizon::Annual);
    }
    if !monthly.is_empty() {
        display_horizon_section_with_blocks(&monthly, TimeHorizon::Monthly);
    }
    if !weekly.is_empty() {
        display_horizon_section_with_blocks(&weekly, TimeHorizon::Weekly);
    }
    if !daily.is_empty() {
        display_horizon_section_with_blocks(&daily, TimeHorizon::Daily);
    }
    if !other.is_empty() {
        println!("\n{}", "━".repeat(60).dimmed());
        println!(
            "{}",
            "📋 Uncategorized Tasks"
                .bold()
                .white()
        );
        println!("{}", "━".repeat(60).dimmed());
        for task in other {
            display_task_compact(task);
        }
    }
}

fn display_horizon_section(tasks: &[&Task], horizon: TimeHorizon) {
    let header = format!(
        "{} {} Tasks ({})",
        horizon.emoji(),
        horizon,
        tasks.len()
    );

    println!("\n{}", "━".repeat(60).dimmed());
    println!("{}", header.bold().cyan());
    println!("{}", "━".repeat(60).dimmed());

    for task in tasks {
        display_task_compact(task);
    }
}

/// Display tasks within a horizon, grouped by daytime blocks
fn display_horizon_section_with_blocks(tasks: &[&Task], horizon: TimeHorizon) {
    use crate::models::DaytimeBlock;

    let header = format!(
        "{} {} Tasks ({})",
        horizon.emoji(),
        horizon,
        tasks.len()
    );

    println!("\n{}", "═".repeat(70).cyan());
    println!("{}", header.bold().cyan());
    println!("{}", "═".repeat(70).cyan());

    // Group by daytime block
    let mut morning: Vec<&Task> = vec![];
    let mut work: Vec<&Task> = vec![];
    let mut afternoon: Vec<&Task> = vec![];
    let mut evening: Vec<&Task> = vec![];
    let mut no_block: Vec<&Task> = vec![];

    for task in tasks {
        match task.daytime_block() {
            Some(DaytimeBlock::Morning) => morning.push(task),
            Some(DaytimeBlock::Work) => work.push(task),
            Some(DaytimeBlock::Afternoon) => afternoon.push(task),
            Some(DaytimeBlock::Evening) => evening.push(task),
            None => no_block.push(task),
        }
    }

    if !morning.is_empty() {
        display_daytime_block_section(&morning, DaytimeBlock::Morning);
    }
    if !work.is_empty() {
        display_daytime_block_section(&work, DaytimeBlock::Work);
    }
    if !afternoon.is_empty() {
        display_daytime_block_section(&afternoon, DaytimeBlock::Afternoon);
    }
    if !evening.is_empty() {
        display_daytime_block_section(&evening, DaytimeBlock::Evening);
    }
    if !no_block.is_empty() {
        println!("\n  {} {}", "📋".dimmed(), "Unscheduled".dimmed().italic());
        for task in no_block {
            display_task_compact_indented(task, 4);
        }
    }
}

/// Display a daytime block section
fn display_daytime_block_section(tasks: &[&Task], block: crate::models::DaytimeBlock) {
    let header = format!(
        "{} {} ({}) - {}",
        block.emoji(),
        block,
        tasks.len(),
        block.time_range()
    );

    println!("\n  {}", "─".repeat(55).dimmed());
    println!("  {}", header.bold());

    for task in tasks {
        display_task_compact_indented(task, 4);
    }
}

/// Display tasks grouped by daytime block only (for single horizon view)
pub fn display_tasks_by_daytime_block(tasks: &[Task], title: &str) {
    use crate::models::DaytimeBlock;

    if tasks.is_empty() {
        println!("\n{}", "No tasks found.".dimmed());
        return;
    }

    println!("\n{}", "═".repeat(70).cyan());
    println!("{}", title.bold().cyan());
    println!("{}", "═".repeat(70).cyan());

    let mut morning: Vec<&Task> = vec![];
    let mut work: Vec<&Task> = vec![];
    let mut afternoon: Vec<&Task> = vec![];
    let mut evening: Vec<&Task> = vec![];
    let mut no_block: Vec<&Task> = vec![];

    for task in tasks {
        match task.daytime_block() {
            Some(DaytimeBlock::Morning) => morning.push(task),
            Some(DaytimeBlock::Work) => work.push(task),
            Some(DaytimeBlock::Afternoon) => afternoon.push(task),
            Some(DaytimeBlock::Evening) => evening.push(task),
            None => no_block.push(task),
        }
    }

    if !morning.is_empty() {
        display_daytime_block_section(&morning, DaytimeBlock::Morning);
    }
    if !work.is_empty() {
        display_daytime_block_section(&work, DaytimeBlock::Work);
    }
    if !afternoon.is_empty() {
        display_daytime_block_section(&afternoon, DaytimeBlock::Afternoon);
    }
    if !evening.is_empty() {
        display_daytime_block_section(&evening, DaytimeBlock::Evening);
    }
    if !no_block.is_empty() {
        println!("\n  {} {}", "📋".dimmed(), "Unscheduled".dimmed().italic());
        for task in no_block {
            display_task_compact_indented(task, 4);
        }
    }

    println!(
        "\n{}",
        format!("Total: {} task(s)", tasks.len()).dimmed()
    );
}

/// Display a single task in compact format
pub fn display_task_compact(task: &Task) {
    display_task_compact_indented(task, 2);
}

/// Display a single task in compact format with custom indentation
pub fn display_task_compact_indented(task: &Task, indent: usize) {
    let state_indicator = match task.state.state_type.as_str() {
        "completed" => "✅".to_string(),
        "canceled" => "❌".to_string(),
        "started" => "🔵".to_string(),
        _ => "⚪".to_string(),
    };

    let priority_indicator = match task.priority {
        1 => "!!!".red().to_string(),
        2 => "!!".yellow().to_string(),
        3 => "!".green().to_string(),
        _ => " ".to_string(),
    };

    let due_str = task
        .due_date
        .map(|d| {
            let today = chrono::Local::now().date_naive();
            let days = (d - today).num_days();
            if days < 0 {
                format!("⚠️  overdue").red().to_string()
            } else if days == 0 {
                "🔥 today".red().to_string()
            } else if days <= 3 {
                format!("⏰ {}d", days).yellow().to_string()
            } else {
                d.format("%m/%d").to_string().dimmed().to_string()
            }
        })
        .unwrap_or_default();

    let project_str = task
        .project
        .as_ref()
        .map(|p| format!("[{}]", p.name).dimmed().to_string())
        .unwrap_or_default();

    let indent_str = " ".repeat(indent);
    println!(
        "{}{} {} {} {} {} {}",
        indent_str,
        state_indicator,
        task.identifier.dimmed(),
        priority_indicator,
        truncate(&task.title, 45),
        due_str,
        project_str
    );
}

/// Display detailed task view
pub fn display_task_detail(task: &Task) {
    let width = 60;
    let border = "═".repeat(width);

    println!("\n╔{}╗", border);
    println!("║ {} {}", "📋".to_string(), task.identifier.bold().cyan());
    println!("╠{}╣", "═".repeat(width));

    // Title
    println!("║ {}: {}", "Title".bold(), task.title);

    // State & Priority
    println!(
        "║ {}: {}  |  {}: {}",
        "State".bold(),
        format_state(&task.state),
        "Priority".bold(),
        format_priority(task.priority)
    );

    // Due Date
    if let Some(due) = &task.due_date {
        println!("║ {}: {}", "Due Date".bold(), format_due_date(&Some(*due)));
    }

    // Project
    if let Some(project) = &task.project {
        println!("║ {}: {}", "Project".bold(), project.name);
    }

    // Labels
    if !task.labels.is_empty() {
        println!("║ {}: {}", "Labels".bold(), format_labels(&task.labels));
    }

    // Assignee
    if let Some(assignee) = &task.assignee {
        println!("║ {}: {}", "Assignee".bold(), assignee.name);
    }

    // Description
    if let Some(desc) = &task.description {
        println!("╠{}╣", "═".repeat(width));
        println!("║ {}:", "Description".bold());
        for line in desc.lines().take(10) {
            println!("║   {}", truncate(line, width - 4));
        }
    }

    // Timestamps
    println!("╠{}╣", "═".repeat(width));
    println!(
        "║ {}: {}",
        "Created".dimmed(),
        task.created_at.format("%Y-%m-%d %H:%M")
    );
    println!(
        "║ {}: {}",
        "Updated".dimmed(),
        task.updated_at.format("%Y-%m-%d %H:%M")
    );

    // URL
    println!("╠{}╣", "═".repeat(width));
    println!("║ {}: {}", "URL".dimmed(), task.url);

    println!("╚{}╝", border);
}

/// Display teams in a table
pub fn display_teams_table(teams: &[Team]) {
    if teams.is_empty() {
        println!("\n{}", "No teams found.".dimmed());
        return;
    }

    let mut builder = Builder::default();
    builder.push_record(["Key", "Name", "ID"]);

    for team in teams {
        builder.push_record([
            &team.key,
            &team.name,
            &team.id,
        ]);
    }

    let mut table = builder.build();
    table
        .with(Style::rounded())
        .with(Panel::header("📁 Teams"));

    println!("\n{}", table);
}

/// Display projects in a table
pub fn display_projects_table(projects: &[Project]) {
    if projects.is_empty() {
        println!("\n{}", "No projects found.".dimmed());
        return;
    }

    let mut builder = Builder::default();
    builder.push_record(["Name", "Start Date", "Target Date", "ID"]);

    for project in projects {
        builder.push_record([
            &format!(
                "{} {}",
                project.icon.as_deref().unwrap_or("📁"),
                project.name
            ),
            &project
                .start_date
                .map(|d| d.to_string())
                .unwrap_or_else(|| "—".to_string()),
            &project
                .target_date
                .map(|d| d.to_string())
                .unwrap_or_else(|| "—".to_string()),
            &project.id,
        ]);
    }

    let mut table = builder.build();
    table
        .with(Style::rounded())
        .with(Panel::header("📂 Projects"));

    println!("\n{}", table);
}

/// Display labels
pub fn display_labels_table(labels: &[Label]) {
    if labels.is_empty() {
        println!("\n{}", "No labels found.".dimmed());
        return;
    }

    let mut builder = Builder::default();
    builder.push_record(["Name", "Color", "ID"]);

    for label in labels {
        builder.push_record([
            &label.name,
            label.color.as_deref().unwrap_or("—"),
            &label.id,
        ]);
    }

    let mut table = builder.build();
    table
        .with(Style::rounded())
        .with(Panel::header("🏷️  Labels"));

    println!("\n{}", table);
}

/// Display workflow states
pub fn display_states_table(states: &[TaskState]) {
    if states.is_empty() {
        println!("\n{}", "No states found.".dimmed());
        return;
    }

    let mut builder = Builder::default();
    builder.push_record(["Type", "Name", "ID"]);

    for state in states {
        builder.push_record([
            &format!("{} {}", state.emoji(), state.state_type),
            &state.name,
            &state.id,
        ]);
    }

    let mut table = builder.build();
    table
        .with(Style::rounded())
        .with(Panel::header("📊 Workflow States"));

    println!("\n{}", table);
}

/// Display task templates
pub fn display_templates(store: &crate::templates::TemplateStore, horizon_filter: Option<crate::models::TimeHorizon>) {
    use crate::models::TimeHorizon;

    let show_annual = horizon_filter.is_none() || horizon_filter == Some(TimeHorizon::Annual);
    let show_monthly = horizon_filter.is_none() || horizon_filter == Some(TimeHorizon::Monthly);
    let show_weekly = horizon_filter.is_none() || horizon_filter == Some(TimeHorizon::Weekly);

    let mut has_content = false;

    if show_annual && !store.annual.is_empty() {
        has_content = true;
        display_template_section(&store.annual, "🎯 Annual Templates");
    }

    if show_monthly && !store.monthly.is_empty() {
        has_content = true;
        display_template_section(&store.monthly, "📅 Monthly Templates");
    }

    if show_weekly && !store.weekly.is_empty() {
        has_content = true;
        display_template_section(&store.weekly, "📆 Weekly Templates");
    }

    if !has_content {
        println!("\n{}", "No templates found.".dimmed());
        println!("Use 'linear-tasks templates add' to create templates.");
    }
}

fn display_template_section(templates: &[crate::templates::TaskTemplate], title: &str) {
    let mut builder = Builder::default();
    builder.push_record(["ID", "Title", "Days", "Block", "Priority", "Active"]);

    for template in templates {
        let priority_str = template
            .priority
            .map(|p| match p {
                1 => "🔴 Urgent",
                2 => "🟠 High",
                3 => "🟡 Normal",
                4 => "🟢 Low",
                _ => "—",
            })
            .unwrap_or("—");

        let block_str = template
            .block
            .map(|b| format!("{} {}", b.emoji(), b))
            .unwrap_or_else(|| "—".to_string());

        let active_str = if template.active { "✅" } else { "❌" };

        builder.push_record([
            &template.id,
            &truncate(&template.title, 35),
            &template.days_display(),
            &block_str,
            priority_str,
            active_str,
        ]);
    }

    let mut table = builder.build();
    table
        .with(Style::rounded())
        .with(Panel::header(title));

    println!("\n{}", table);
}

/// Display success message
pub fn display_success(message: &str) {
    println!("\n{} {}", "✅".green(), message.green());
}

/// Display error message
pub fn display_error(message: &str) {
    eprintln!("\n{} {}", "❌".red(), message.red());
}

/// Display warning message
pub fn display_warning(message: &str) {
    println!("\n{} {}", "⚠️".yellow(), message.yellow());
}

/// Display info message
pub fn display_info(message: &str) {
    println!("\n{} {}", "ℹ️".blue(), message.blue());
}

/// Display a spinner while waiting
pub fn create_spinner(message: &str) -> indicatif::ProgressBar {
    let pb = indicatif::ProgressBar::new_spinner();
    pb.set_style(
        indicatif::ProgressStyle::default_spinner()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(std::time::Duration::from_millis(80));
    pb
}

/// Display dashboard summary
pub fn display_dashboard(tasks: &[Task], team_name: &str) {
    let total = tasks.len();
    let completed = tasks.iter().filter(|t| t.is_completed()).count();
    let active = tasks.iter().filter(|t| t.is_active()).count();
    let overdue = tasks
        .iter()
        .filter(|t| {
            t.is_active()
                && t.due_date
                    .map(|d| d < chrono::Local::now().date_naive())
                    .unwrap_or(false)
        })
        .count();
    let due_today = tasks
        .iter()
        .filter(|t| {
            t.is_active()
                && t.due_date
                    .map(|d| d == chrono::Local::now().date_naive())
                    .unwrap_or(false)
        })
        .count();

    let urgent = tasks
        .iter()
        .filter(|t| t.is_active() && t.priority == 1)
        .count();

    println!("\n{}", "╔════════════════════════════════════════════════════════════╗".cyan());
    println!(
        "{}  {} Dashboard - {}",
        "║".cyan(),
        "📊",
        team_name.bold()
    );
    println!("{}", "╠════════════════════════════════════════════════════════════╣".cyan());
    println!(
        "{}  {} Total Tasks: {}",
        "║".cyan(),
        "📋",
        total.to_string().bold()
    );
    println!(
        "{}  {} Active: {}  |  {} Completed: {}",
        "║".cyan(),
        "🔵",
        active.to_string().blue(),
        "✅",
        completed.to_string().green()
    );
    
    if overdue > 0 {
        println!(
            "{}  {} Overdue: {}",
            "║".cyan(),
            "⚠️",
            overdue.to_string().red().bold()
        );
    }
    
    if due_today > 0 {
        println!(
            "{}  {} Due Today: {}",
            "║".cyan(),
            "🔥",
            due_today.to_string().yellow().bold()
        );
    }
    
    if urgent > 0 {
        println!(
            "{}  {} Urgent: {}",
            "║".cyan(),
            "🔴",
            urgent.to_string().red()
        );
    }

    println!("{}", "╚════════════════════════════════════════════════════════════╝".cyan());

    // Show time horizon breakdown
    let annual = tasks.iter().filter(|t| t.time_horizon() == Some(TimeHorizon::Annual) && t.is_active()).count();
    let monthly = tasks.iter().filter(|t| t.time_horizon() == Some(TimeHorizon::Monthly) && t.is_active()).count();
    let weekly = tasks.iter().filter(|t| t.time_horizon() == Some(TimeHorizon::Weekly) && t.is_active()).count();
    let daily = tasks.iter().filter(|t| t.time_horizon() == Some(TimeHorizon::Daily) && t.is_active()).count();

    if annual + monthly + weekly + daily > 0 {
        println!("\n{}", "Time Horizon Breakdown:".bold());
        if annual > 0 {
            println!("  {} Annual:  {}", "🎯", annual);
        }
        if monthly > 0 {
            println!("  {} Monthly: {}", "📅", monthly);
        }
        if weekly > 0 {
            println!("  {} Weekly:  {}", "📆", weekly);
        }
        if daily > 0 {
            println!("  {} Daily:   {}", "📌", daily);
        }
    }
}
