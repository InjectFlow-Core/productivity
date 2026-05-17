use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    #[serde(default = "default_morning")]
    pub morning_time: String,
    #[serde(default = "default_evening")]
    pub evening_time: String,
    #[serde(default)]
    pub claude_api_key: Option<String>,
}

fn default_morning() -> String { "05:00".to_string() }
fn default_evening() -> String { "18:00".to_string() }

impl Default for Config {
    fn default() -> Self {
        Self {
            morning_time: default_morning(),
            evening_time: default_evening(),
            claude_api_key: None,
        }
    }
}

fn config_path() -> PathBuf {
    let base = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(base)
        .join(".local").join("share")
        .join("daily-planner")
        .join("config.json")
}

fn systemd_dir() -> PathBuf {
    let base = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(base).join(".config").join("systemd").join("user")
}

pub fn read_config() -> Config {
    fs::read_to_string(config_path())
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or_default()
}

pub fn save_config(config: &Config) -> Result<(), String> {
    let path = config_path();
    fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn timers_installed() -> bool {
    let dir = systemd_dir();
    dir.join("daily-planner.timer").exists() && dir.join("daily-planner-evening.timer").exists()
}

pub fn apply_schedule(morning_time: &str, evening_time: &str) -> Result<(), String> {
    let dir = systemd_dir();
    let morning_timer = dir.join("daily-planner.timer");
    let evening_timer = dir.join("daily-planner-evening.timer");

    let mut any_patched = false;
    if morning_timer.exists() {
        patch_oncalendar(&morning_timer, morning_time)?;
        any_patched = true;
    }
    if evening_timer.exists() {
        patch_oncalendar(&evening_timer, evening_time)?;
        any_patched = true;
    }

    if any_patched {
        std::process::Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .status()
            .map_err(|e| e.to_string())?;
        for name in &["daily-planner.timer", "daily-planner-evening.timer"] {
            std::process::Command::new("systemctl")
                .args(["--user", "restart", name])
                .status()
                .ok();
        }
    }

    Ok(())
}

fn patch_oncalendar(path: &PathBuf, time: &str) -> Result<(), String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let patched = content
        .lines()
        .map(|line| {
            if line.starts_with("OnCalendar=") {
                format!("OnCalendar=*-*-* {}:00", time)
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(path, patched + "\n").map_err(|e| e.to_string())?;
    Ok(())
}
