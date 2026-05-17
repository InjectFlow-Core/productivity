use chrono::Local;
use serde::{Deserialize, Deserializer, Serialize};
use std::fs;
use std::path::PathBuf;

fn default_category() -> String {
    "personal".to_string()
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Priority {
    pub text: String,
    #[serde(default)]
    pub done: bool,
    #[serde(default = "default_category")]
    pub category: String,
}

// Handles: old string "Buy milk", legacy object {text,done}, new object {text,done,category}
impl<'de> Deserialize<'de> for Priority {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Text(String),
            Object {
                text: String,
                #[serde(default)]
                done: bool,
                #[serde(default = "default_category")]
                category: String,
            },
        }
        match Raw::deserialize(d)? {
            Raw::Text(t) => Ok(Priority { text: t, done: false, category: default_category() }),
            Raw::Object { text, done, category } => Ok(Priority { text, done, category }),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DayPlan {
    pub date: String,
    pub intention: String,
    pub priorities: Vec<Priority>,
    pub notes: String,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_notes: Option<String>,
}

fn plans_dir() -> PathBuf {
    let base = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(base)
        .join(".local")
        .join("share")
        .join("daily-planner")
        .join("plans")
}

fn today_plan_path() -> PathBuf {
    let date = Local::now().format("%Y-%m-%d").to_string();
    plans_dir().join(format!("{}.json", date))
}

fn plan_path(date: &str) -> PathBuf {
    plans_dir().join(format!("{}.json", date))
}

pub fn today_plan_exists() -> bool {
    today_plan_path().exists()
}

pub fn today_reviewed() -> bool {
    fs::read_to_string(today_plan_path())
        .ok()
        .and_then(|c| serde_json::from_str::<DayPlan>(&c).ok())
        .map(|p| p.reviewed_at.is_some())
        .unwrap_or(false)
}

pub fn get_today_plan() -> Option<DayPlan> {
    fs::read_to_string(today_plan_path())
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok())
}

pub fn get_week_plans() -> Vec<DayPlan> {
    use chrono::{Datelike, Duration, Local};
    let today = Local::now().date_naive();
    let monday = today - Duration::days(today.weekday().num_days_from_monday() as i64);
    let mut plans = Vec::new();
    let mut date = monday;
    while date <= today {
        let path = plan_path(&date.format("%Y-%m-%d").to_string());
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(plan) = serde_json::from_str(&content) {
                plans.push(plan);
            }
        }
        date += Duration::days(1);
    }
    plans
}

pub fn get_all_plans() -> Vec<DayPlan> {
    let dir = plans_dir();
    let mut plans = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(plan) = serde_json::from_str::<DayPlan>(&content) {
                        plans.push(plan);
                    }
                }
            }
        }
    }
    plans.sort_by(|a, b| b.date.cmp(&a.date));
    plans
}

pub fn save_today_plan(plan: &DayPlan) -> Result<(), String> {
    let dir = plans_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(plan).map_err(|e| e.to_string())?;
    fs::write(today_plan_path(), json).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn toggle_priority(date: &str, index: usize) -> Result<DayPlan, String> {
    let path = plan_path(date);
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut plan: DayPlan = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    if let Some(p) = plan.priorities.get_mut(index) {
        p.done = !p.done;
    }
    let json = serde_json::to_string_pretty(&plan).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(plan)
}

pub fn save_review(review_notes: String) -> Result<(), String> {
    let path = today_plan_path();
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut plan: DayPlan = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    plan.reviewed_at = Some(Local::now().to_rfc3339());
    plan.review_notes = if review_notes.trim().is_empty() {
        None
    } else {
        Some(review_notes.trim().to_string())
    };
    let json = serde_json::to_string_pretty(&plan).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;
    use std::sync::Mutex;
    use std::time::{SystemTime, UNIX_EPOCH};

    static HOME_LOCK: Mutex<()> = Mutex::new(());

    struct TestHome {
        original_home: Option<String>,
        path: PathBuf,
    }

    impl TestHome {
        fn install() -> Self {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "daily-planner-test-{}-{}",
                std::process::id(),
                timestamp
            ));
            fs::create_dir_all(&path).expect("test home should be created");
            let original_home = std::env::var("HOME").ok();
            std::env::set_var("HOME", &path);
            Self { original_home, path }
        }
    }

    impl Drop for TestHome {
        fn drop(&mut self) {
            match &self.original_home {
                Some(home) => std::env::set_var("HOME", home),
                None => std::env::remove_var("HOME"),
            }
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn plan_for_date(date: &str, intention: &str) -> DayPlan {
        DayPlan {
            date: date.to_string(),
            intention: intention.to_string(),
            priorities: vec![Priority {
                text: "First".to_string(),
                done: false,
                category: "personal".to_string(),
            }],
            notes: String::new(),
            created_at: "2026-05-16T08:00:00+01:00".to_string(),
            reviewed_at: None,
            review_notes: None,
        }
    }

    fn write_plan_file(file_name: &str, plan: &DayPlan) {
        fs::create_dir_all(plans_dir()).expect("plans directory should be created");
        fs::write(
            plans_dir().join(file_name),
            serde_json::to_string(plan).expect("plan should serialize"),
        )
        .expect("plan file should be written");
    }

    #[test]
    fn save_today_plan_writes_plan_for_current_date() {
        let _guard = HOME_LOCK.lock().expect("home lock should not be poisoned");
        let _home = TestHome::install();

        let plan = DayPlan {
            date: Local::now().format("%Y-%m-%d").to_string(),
            intention: "Focus on the important work".to_string(),
            priorities: vec![
                Priority { text: "Plan the day".to_string(), done: false, category: "personal".to_string() },
                Priority { text: "Ship the fix".to_string(), done: false, category: "work".to_string() },
            ],
            notes: "Keep it simple".to_string(),
            created_at: "2026-05-16T08:30:00+01:00".to_string(),
            reviewed_at: None,
            review_notes: None,
        };

        assert!(!today_plan_exists());
        save_today_plan(&plan).expect("plan should be saved");
        assert!(today_plan_exists());

        let saved = fs::read_to_string(today_plan_path()).expect("saved plan should be readable");
        let saved: DayPlan = serde_json::from_str(&saved).expect("saved plan should be valid JSON");

        assert_eq!(saved.date, plan.date);
        assert_eq!(saved.intention, plan.intention);
        assert_eq!(saved.priorities, plan.priorities);
        assert_eq!(saved.notes, plan.notes);
        assert_eq!(saved.created_at, plan.created_at);
        assert!(saved.reviewed_at.is_none());
    }

    #[test]
    fn priority_deserializes_legacy_strings_and_new_objects() {
        let legacy: Priority =
            serde_json::from_str(r#""Plan the day""#).expect("legacy priority should parse");
        let current: Priority = serde_json::from_str(r#"{"text":"Ship","done":true}"#)
            .expect("current priority should parse");

        assert_eq!(
            legacy,
            Priority {
                text: "Plan the day".to_string(),
                done: false,
                category: "personal".to_string(),
            }
        );
        assert_eq!(
            current,
            Priority {
                text: "Ship".to_string(),
                done: true,
                category: "personal".to_string(),
            }
        );
    }

    #[test]
    fn get_today_plan_reads_saved_plan() {
        let _guard = HOME_LOCK.lock().expect("home lock should not be poisoned");
        let _home = TestHome::install();

        let today = Local::now().format("%Y-%m-%d").to_string();
        let plan = plan_for_date(&today, "Read today's plan");
        save_today_plan(&plan).expect("plan should be saved");

        let saved = get_today_plan().expect("today's plan should be returned");

        assert_eq!(saved.date, today);
        assert_eq!(saved.intention, "Read today's plan");
    }

    #[test]
    fn get_all_plans_returns_valid_json_plans_sorted_by_newest_date() {
        let _guard = HOME_LOCK.lock().expect("home lock should not be poisoned");
        let _home = TestHome::install();

        write_plan_file("older.json", &plan_for_date("2026-05-14", "Older plan"));
        write_plan_file("newer.json", &plan_for_date("2026-05-16", "Newer plan"));
        write_plan_file("middle.json", &plan_for_date("2026-05-15", "Middle plan"));
        fs::write(plans_dir().join("invalid.json"), "{not json")
            .expect("invalid file should be written");
        fs::write(plans_dir().join("readme.txt"), "not a plan")
            .expect("text file should be written");

        let plans = get_all_plans();
        let dates: Vec<&str> = plans.iter().map(|plan| plan.date.as_str()).collect();
        let intentions: Vec<&str> = plans.iter().map(|plan| plan.intention.as_str()).collect();

        assert_eq!(dates, vec!["2026-05-16", "2026-05-15", "2026-05-14"]);
        assert_eq!(intentions, vec!["Newer plan", "Middle plan", "Older plan"]);
    }

    #[test]
    fn save_review_marks_today_reviewed_and_trims_notes() {
        let _guard = HOME_LOCK.lock().expect("home lock should not be poisoned");
        let _home = TestHome::install();

        let today = Local::now().format("%Y-%m-%d").to_string();
        save_today_plan(&plan_for_date(&today, "Review today's plan")).expect("plan should save");

        assert!(!today_reviewed());
        save_review("  Finished the essentials  ".to_string()).expect("review should save");

        let reviewed = get_today_plan().expect("reviewed plan should be readable");
        assert!(today_reviewed());
        assert!(reviewed.reviewed_at.is_some());
        assert_eq!(
            reviewed.review_notes,
            Some("Finished the essentials".to_string())
        );
    }

    #[test]
    fn toggle_priority_flips_done_state() {
        let _guard = HOME_LOCK.lock().expect("home lock should not be poisoned");
        let _home = TestHome::install();

        let date = Local::now().format("%Y-%m-%d").to_string();
        let plan = DayPlan {
            date: date.clone(),
            intention: "Test".to_string(),
            priorities: vec![Priority { text: "First".to_string(), done: false, category: "personal".to_string() }],
            notes: String::new(),
            created_at: "2026-05-16T08:00:00+01:00".to_string(),
            reviewed_at: None,
            review_notes: None,
        };
        save_today_plan(&plan).expect("plan should be saved");

        let updated = toggle_priority(&date, 0).expect("toggle should succeed");
        assert!(updated.priorities[0].done);

        let updated = toggle_priority(&date, 0).expect("toggle should succeed");
        assert!(!updated.priorities[0].done);
    }
}
