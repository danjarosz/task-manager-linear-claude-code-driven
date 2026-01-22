use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

use crate::models::AppConfig;

/// Get the configuration directory path
pub fn config_dir() -> Result<PathBuf> {
    let dir = dirs::config_dir()
        .context("Could not determine config directory")?
        .join("linear-tasks");

    if !dir.exists() {
        fs::create_dir_all(&dir).context("Failed to create config directory")?;
    }

    Ok(dir)
}

/// Get the configuration file path
pub fn config_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("config.toml"))
}

/// Load configuration from file
pub fn load_config() -> Result<AppConfig> {
    let path = config_path()?;

    if !path.exists() {
        return Ok(AppConfig::default());
    }

    let contents = fs::read_to_string(&path).context("Failed to read config file")?;

    toml::from_str(&contents).context("Failed to parse config file")
}

/// Save configuration to file
pub fn save_config(config: &AppConfig) -> Result<()> {
    let path = config_path()?;

    let contents = toml::to_string_pretty(config).context("Failed to serialize config")?;

    fs::write(&path, contents).context("Failed to write config file")?;

    Ok(())
}

/// Get API key from config or environment
pub fn get_api_key(config: &AppConfig) -> Option<String> {
    // Priority: environment variable > config file
    std::env::var("LINEAR_API_KEY")
        .ok()
        .or_else(|| {
            if config.linear_api_key.is_empty() {
                None
            } else {
                Some(config.linear_api_key.clone())
            }
        })
}

/// Interactive setup wizard
pub fn setup_wizard() -> Result<AppConfig> {
    use dialoguer::{Input, Password};

    println!("\n🔧 Linear Tasks Configuration Wizard\n");

    let api_key: String = Password::new()
        .with_prompt("Enter your Linear API key")
        .interact()
        .context("Failed to read API key")?;

    let default_team: String = Input::new()
        .with_prompt("Default team name (optional, press Enter to skip)")
        .allow_empty(true)
        .interact_text()
        .context("Failed to read team name")?;

    let config = AppConfig {
        linear_api_key: api_key,
        default_team_id: None,
        default_team_name: if default_team.is_empty() {
            None
        } else {
            Some(default_team)
        },
        time_horizon_labels: Default::default(),
        daytime_block_labels: Default::default(),
    };

    save_config(&config)?;

    println!("\n✅ Configuration saved to {:?}", config_path()?);

    Ok(config)
}

/// Initialize time horizon labels in Linear
pub async fn ensure_time_horizon_labels(
    client: &crate::linear::LinearClient,
    team_id: Option<&str>,
) -> Result<()> {
    use crate::models::TimeHorizon;

    let existing_labels = client.get_labels(team_id).await?;
    let existing_names: Vec<_> = existing_labels.iter().map(|l| l.name.to_lowercase()).collect();

    for horizon in TimeHorizon::all() {
        let label_name = horizon.to_string();
        if !existing_names.contains(&label_name.to_lowercase()) {
            println!("Creating time horizon label: {}", label_name);
            client
                .create_label(&label_name, horizon.color(), team_id)
                .await?;
        }
    }

    Ok(())
}

/// Initialize daytime block labels in Linear (at team level)
pub async fn ensure_daytime_block_labels(
    client: &crate::linear::LinearClient,
    team_id: &str,
) -> Result<()> {
    use crate::models::DaytimeBlock;

    let existing_labels = client.get_labels(Some(team_id)).await?;
    let existing_names: Vec<_> = existing_labels.iter().map(|l| l.name.to_lowercase()).collect();

    for block in DaytimeBlock::all() {
        let label_name = block.to_string();
        if !existing_names.contains(&label_name.to_lowercase()) {
            println!(
                "Creating daytime block label: {} {} ({})",
                block.emoji(),
                label_name,
                block.time_range()
            );
            client
                .create_label(&label_name, block.color(), Some(team_id))
                .await?;
        }
    }

    Ok(())
}

/// Initialize all labels (time horizons + daytime blocks)
pub async fn ensure_all_labels(
    client: &crate::linear::LinearClient,
    team_id: &str,
) -> Result<()> {
    ensure_time_horizon_labels(client, Some(team_id)).await?;
    ensure_daytime_block_labels(client, team_id).await?;
    Ok(())
}
