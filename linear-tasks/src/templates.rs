use anyhow::{Context, Result};
use chrono::{Datelike, NaiveDate, Weekday};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

use crate::models::{DaytimeBlock, TimeHorizon};

/// A task template stored locally
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTemplate {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub horizon: TimeHorizon,
    pub block: Option<DaytimeBlock>,
    pub priority: Option<i32>,
    /// For weekly tasks: which day(s) of the week
    /// For monthly/annual: optional preferred days, or None for "any day"
    pub days: Vec<Weekday>,
    /// Whether this template is active
    pub active: bool,
    /// Created timestamp
    pub created_at: String,
}

impl TaskTemplate {
    pub fn new(
        title: String,
        horizon: TimeHorizon,
        block: Option<DaytimeBlock>,
        days: Vec<Weekday>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string()[..8].to_string(),
            title,
            description: None,
            horizon,
            block,
            priority: None,
            days,
            active: true,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn days_display(&self) -> String {
        if self.days.is_empty() {
            return "Any day".to_string();
        }
        self.days
            .iter()
            .map(|d| weekday_short(*d))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn weekday_short(day: Weekday) -> &'static str {
    match day {
        Weekday::Mon => "Mon",
        Weekday::Tue => "Tue",
        Weekday::Wed => "Wed",
        Weekday::Thu => "Thu",
        Weekday::Fri => "Fri",
        Weekday::Sat => "Sat",
        Weekday::Sun => "Sun",
    }
}

/// All templates stored locally
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TemplateStore {
    pub annual: Vec<TaskTemplate>,
    pub monthly: Vec<TaskTemplate>,
    pub weekly: Vec<TaskTemplate>,
    /// Track which templates were generated for which dates
    pub generation_history: HashMap<String, Vec<String>>, // date -> list of template IDs
}

impl TemplateStore {
    /// Get the storage file path
    fn storage_path() -> Result<PathBuf> {
        let dir = dirs::config_dir()
            .context("Could not determine config directory")?
            .join("linear-tasks");

        if !dir.exists() {
            fs::create_dir_all(&dir)?;
        }

        Ok(dir.join("templates.json"))
    }

    /// Load templates from disk
    pub fn load() -> Result<Self> {
        let path = Self::storage_path()?;

        if !path.exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(&path)?;
        serde_json::from_str(&contents).context("Failed to parse templates file")
    }

    /// Save templates to disk
    pub fn save(&self) -> Result<()> {
        let path = Self::storage_path()?;
        let contents = serde_json::to_string_pretty(self)?;
        fs::write(&path, contents)?;
        Ok(())
    }

    /// Add a template
    pub fn add(&mut self, template: TaskTemplate) -> Result<String> {
        let id = template.id.clone();
        match template.horizon {
            TimeHorizon::Annual => self.annual.push(template),
            TimeHorizon::Monthly => self.monthly.push(template),
            TimeHorizon::Weekly | TimeHorizon::Daily => self.weekly.push(template),
        }
        self.save()?;
        Ok(id)
    }

    /// Remove a template by ID
    pub fn remove(&mut self, id: &str) -> Result<bool> {
        let mut found = false;

        self.annual.retain(|t| {
            if t.id == id {
                found = true;
                false
            } else {
                true
            }
        });

        self.monthly.retain(|t| {
            if t.id == id {
                found = true;
                false
            } else {
                true
            }
        });

        self.weekly.retain(|t| {
            if t.id == id {
                found = true;
                false
            } else {
                true
            }
        });

        if found {
            self.save()?;
        }
        Ok(found)
    }

    /// Find a template by ID
    pub fn find(&self, id: &str) -> Option<&TaskTemplate> {
        self.annual
            .iter()
            .chain(self.monthly.iter())
            .chain(self.weekly.iter())
            .find(|t| t.id == id)
    }

    /// Find a template by ID (mutable)
    pub fn find_mut(&mut self, id: &str) -> Option<&mut TaskTemplate> {
        for t in self.annual.iter_mut() {
            if t.id == id {
                return Some(t);
            }
        }
        for t in self.monthly.iter_mut() {
            if t.id == id {
                return Some(t);
            }
        }
        for t in self.weekly.iter_mut() {
            if t.id == id {
                return Some(t);
            }
        }
        None
    }

    /// Get all templates for a specific horizon
    pub fn by_horizon(&self, horizon: TimeHorizon) -> &[TaskTemplate] {
        match horizon {
            TimeHorizon::Annual => &self.annual,
            TimeHorizon::Monthly => &self.monthly,
            TimeHorizon::Weekly | TimeHorizon::Daily => &self.weekly,
        }
    }

    /// Get templates that should be generated for a specific weekday
    pub fn templates_for_day(&self, day: Weekday) -> Vec<&TaskTemplate> {
        let mut result = vec![];

        // Weekly tasks assigned to this day
        for t in &self.weekly {
            if t.active && (t.days.is_empty() || t.days.contains(&day)) {
                result.push(t);
            }
        }

        // Monthly tasks - include if assigned to this day or "any day"
        for t in &self.monthly {
            if t.active && (t.days.is_empty() || t.days.contains(&day)) {
                result.push(t);
            }
        }

        // Annual tasks - include if assigned to this day or "any day"
        for t in &self.annual {
            if t.active && (t.days.is_empty() || t.days.contains(&day)) {
                result.push(t);
            }
        }

        result
    }

    /// Check if a template was already generated for a date
    pub fn was_generated(&self, template_id: &str, date: &NaiveDate) -> bool {
        let date_key = date.to_string();
        self.generation_history
            .get(&date_key)
            .map(|ids| ids.contains(&template_id.to_string()))
            .unwrap_or(false)
    }

    /// Mark a template as generated for a date
    pub fn mark_generated(&mut self, template_id: &str, date: &NaiveDate) {
        let date_key = date.to_string();
        self.generation_history
            .entry(date_key)
            .or_default()
            .push(template_id.to_string());
    }

    /// Get all active templates
    pub fn all_active(&self) -> Vec<&TaskTemplate> {
        self.annual
            .iter()
            .chain(self.monthly.iter())
            .chain(self.weekly.iter())
            .filter(|t| t.active)
            .collect()
    }

    /// Count templates by horizon
    pub fn counts(&self) -> (usize, usize, usize) {
        (
            self.annual.iter().filter(|t| t.active).count(),
            self.monthly.iter().filter(|t| t.active).count(),
            self.weekly.iter().filter(|t| t.active).count(),
        )
    }
}

/// Parse weekday from string
pub fn parse_weekday(s: &str) -> Option<Weekday> {
    match s.to_lowercase().as_str() {
        "mon" | "monday" | "poniedziałek" | "poniedzialek" => Some(Weekday::Mon),
        "tue" | "tuesday" | "wtorek" => Some(Weekday::Tue),
        "wed" | "wednesday" | "środa" | "sroda" => Some(Weekday::Wed),
        "thu" | "thursday" | "czwartek" => Some(Weekday::Thu),
        "fri" | "friday" | "piątek" | "piatek" => Some(Weekday::Fri),
        "sat" | "saturday" | "sobota" => Some(Weekday::Sat),
        "sun" | "sunday" | "niedziela" => Some(Weekday::Sun),
        _ => None,
    }
}

/// Parse multiple weekdays from comma-separated string
pub fn parse_weekdays(s: &str) -> Vec<Weekday> {
    s.split(',')
        .filter_map(|part| parse_weekday(part.trim()))
        .collect()
}

/// Get project name for a weekday (matching Linear project names)
pub fn weekday_to_project_name(day: Weekday) -> &'static str {
    match day {
        Weekday::Mon => "Monday",
        Weekday::Tue => "Tuesday",
        Weekday::Wed => "Wednesday",
        Weekday::Thu => "Thursday",
        Weekday::Fri => "Friday",
        Weekday::Sat => "Saturday",
        Weekday::Sun => "Sunday",
    }
}

/// Get weekday from project name
pub fn project_name_to_weekday(name: &str) -> Option<Weekday> {
    parse_weekday(name)
}
