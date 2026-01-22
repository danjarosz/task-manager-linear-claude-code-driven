use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Time horizon for task planning
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeHorizon {
    Annual,
    Monthly,
    Weekly,
    Daily,
}

impl fmt::Display for TimeHorizon {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimeHorizon::Annual => write!(f, "Annual"),
            TimeHorizon::Monthly => write!(f, "Monthly"),
            TimeHorizon::Weekly => write!(f, "Weekly"),
            TimeHorizon::Daily => write!(f, "Daily"),
        }
    }
}

impl TimeHorizon {
    pub fn all() -> Vec<TimeHorizon> {
        vec![
            TimeHorizon::Annual,
            TimeHorizon::Monthly,
            TimeHorizon::Weekly,
            TimeHorizon::Daily,
        ]
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            TimeHorizon::Annual => "🎯",
            TimeHorizon::Monthly => "📅",
            TimeHorizon::Weekly => "📆",
            TimeHorizon::Daily => "📌",
        }
    }

    pub fn color(&self) -> &'static str {
        match self {
            TimeHorizon::Annual => "#f2c94c",
            TimeHorizon::Monthly => "#26b5ce",
            TimeHorizon::Weekly => "#4cb782",
            TimeHorizon::Daily => "#f2994a",
        }
    }
}

/// Daytime blocks for organizing tasks within a day/period
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
pub enum DaytimeBlock {
    Morning,
    Work,
    Afternoon,
    Evening,
}

impl fmt::Display for DaytimeBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DaytimeBlock::Morning => write!(f, "Morning"),
            DaytimeBlock::Work => write!(f, "Work"),
            DaytimeBlock::Afternoon => write!(f, "Afternoon"),
            DaytimeBlock::Evening => write!(f, "Evening"),
        }
    }
}

impl DaytimeBlock {
    pub fn all() -> Vec<DaytimeBlock> {
        vec![
            DaytimeBlock::Morning,
            DaytimeBlock::Work,
            DaytimeBlock::Afternoon,
            DaytimeBlock::Evening,
        ]
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            DaytimeBlock::Morning => "🌅",
            DaytimeBlock::Work => "💼",
            DaytimeBlock::Afternoon => "☀️",
            DaytimeBlock::Evening => "🌙",
        }
    }

    pub fn color(&self) -> &'static str {
        match self {
            DaytimeBlock::Morning => "#f5a623",   // Orange sunrise
            DaytimeBlock::Work => "#4a90d9",      // Blue professional
            DaytimeBlock::Afternoon => "#7ed321", // Green sunny
            DaytimeBlock::Evening => "#9013fe",   // Purple twilight
        }
    }

    pub fn time_range(&self) -> &'static str {
        match self {
            DaytimeBlock::Morning => "06:00 - 09:00",
            DaytimeBlock::Work => "09:00 - 17:00",
            DaytimeBlock::Afternoon => "17:00 - 20:00",
            DaytimeBlock::Evening => "20:00 - 23:00",
        }
    }

    pub fn from_str(s: &str) -> Option<DaytimeBlock> {
        match s.to_lowercase().as_str() {
            "morning" | "rano" | "🌅" => Some(DaytimeBlock::Morning),
            "work" | "praca" | "💼" => Some(DaytimeBlock::Work),
            "afternoon" | "popołudnie" | "☀️" => Some(DaytimeBlock::Afternoon),
            "evening" | "wieczór" | "wieczor" | "🌙" => Some(DaytimeBlock::Evening),
            _ => None,
        }
    }
}

/// Task priority levels matching Linear
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Priority {
    NoPriority = 0,
    Urgent = 1,
    High = 2,
    Normal = 3,
    Low = 4,
}

impl From<i32> for Priority {
    fn from(value: i32) -> Self {
        match value {
            1 => Priority::Urgent,
            2 => Priority::High,
            3 => Priority::Normal,
            4 => Priority::Low,
            _ => Priority::NoPriority,
        }
    }
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Priority::NoPriority => write!(f, "—"),
            Priority::Urgent => write!(f, "🔴 Urgent"),
            Priority::High => write!(f, "🟠 High"),
            Priority::Normal => write!(f, "🟡 Normal"),
            Priority::Low => write!(f, "🟢 Low"),
        }
    }
}

/// Task state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskState {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub state_type: String,
    pub color: Option<String>,
}

impl TaskState {
    pub fn emoji(&self) -> &'static str {
        match self.state_type.as_str() {
            "backlog" => "📥",
            "unstarted" => "⚪",
            "started" => "🔵",
            "completed" => "✅",
            "canceled" => "❌",
            _ => "❓",
        }
    }
}

/// Label attached to a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
}

/// Project in Linear
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    #[serde(rename = "startDate")]
    pub start_date: Option<NaiveDate>,
    #[serde(rename = "targetDate")]
    pub target_date: Option<NaiveDate>,
}

/// Team in Linear
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Team {
    pub id: String,
    pub name: String,
    pub key: String,
    pub icon: Option<String>,
}

/// User in Linear
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: Option<String>,
}

/// A task (issue) in Linear
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub identifier: String,
    pub title: String,
    pub description: Option<String>,
    pub priority: i32,
    #[serde(rename = "dueDate")]
    pub due_date: Option<NaiveDate>,
    pub state: TaskState,
    pub labels: Vec<Label>,
    pub project: Option<Project>,
    pub assignee: Option<User>,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,
    pub url: String,
}

impl Task {
    pub fn priority_enum(&self) -> Priority {
        Priority::from(self.priority)
    }

    /// Determine time horizon based on labels
    pub fn time_horizon(&self) -> Option<TimeHorizon> {
        for label in &self.labels {
            match label.name.to_lowercase().as_str() {
                "annual" | "yearly" | "roczne" => return Some(TimeHorizon::Annual),
                "monthly" | "miesięczne" => return Some(TimeHorizon::Monthly),
                "weekly" | "tygodniowe" => return Some(TimeHorizon::Weekly),
                "daily" | "dzienne" => return Some(TimeHorizon::Daily),
                _ => continue,
            }
        }
        None
    }

    /// Determine daytime block based on labels
    pub fn daytime_block(&self) -> Option<DaytimeBlock> {
        for label in &self.labels {
            if let Some(block) = DaytimeBlock::from_str(&label.name) {
                return Some(block);
            }
        }
        None
    }

    pub fn is_completed(&self) -> bool {
        self.state.state_type == "completed"
    }

    pub fn is_canceled(&self) -> bool {
        self.state.state_type == "canceled"
    }

    pub fn is_active(&self) -> bool {
        !self.is_completed() && !self.is_canceled()
    }
}

/// Input for creating a new task
#[derive(Debug, Clone, Serialize)]
pub struct CreateTaskInput {
    pub title: String,
    pub description: Option<String>,
    pub team_id: String,
    pub project_id: Option<String>,
    pub priority: Option<i32>,
    pub due_date: Option<NaiveDate>,
    pub label_ids: Vec<String>,
    pub assignee_id: Option<String>,
}

/// Input for updating a task
#[derive(Debug, Clone, Serialize)]
pub struct UpdateTaskInput {
    pub id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub priority: Option<i32>,
    pub due_date: Option<NaiveDate>,
    pub state_id: Option<String>,
    pub label_ids: Option<Vec<String>>,
    pub project_id: Option<String>,
}

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub linear_api_key: String,
    pub default_team_id: Option<String>,
    pub default_team_name: Option<String>,
    pub time_horizon_labels: TimeHorizonLabels,
    pub daytime_block_labels: DaytimeBlockLabels,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeHorizonLabels {
    pub annual: String,
    pub monthly: String,
    pub weekly: String,
    pub daily: String,
}

impl Default for TimeHorizonLabels {
    fn default() -> Self {
        Self {
            annual: "Annual".to_string(),
            monthly: "Monthly".to_string(),
            weekly: "Weekly".to_string(),
            daily: "Daily".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaytimeBlockLabels {
    pub morning: String,
    pub work: String,
    pub afternoon: String,
    pub evening: String,
}

impl Default for DaytimeBlockLabels {
    fn default() -> Self {
        Self {
            morning: "Morning".to_string(),
            work: "Work".to_string(),
            afternoon: "Afternoon".to_string(),
            evening: "Evening".to_string(),
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            linear_api_key: String::new(),
            default_team_id: None,
            default_team_name: None,
            time_horizon_labels: TimeHorizonLabels::default(),
            daytime_block_labels: DaytimeBlockLabels::default(),
        }
    }
}
