//! Core domain + storage + index (skeleton)

use anyhow::{Context, Result};
use directories::ProjectDirs;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use ulid::Ulid;
use walkdir::WalkDir;

// ACTIONS
pub mod actions;
pub use actions::Action;

// Keymap Configs
pub mod keymap;
pub use keymap::{Keymap, default_keymap, load_keymap_from_user};

pub mod ex;
pub use ex::{parse_ex, ExCommand, StatusSet};


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub vault_path: PathBuf,
}

impl Config {
    pub fn load_default() -> Result<Self> {
        let proj = ProjectDirs::from("dev", "example", "tm").expect("project dirs");

        let default_vault = directories::UserDirs::new()
            .map(|u| u.home_dir().to_path_buf().join("TasksVault"))
            .unwrap_or_else(|| PathBuf::from("TasksVault"));

        let cfg_dir = proj.config_dir().to_path_buf();
        fs::create_dir_all(&cfg_dir).ok();

        Ok(Self {
            vault_path: default_vault,
        })
    }
}

#[derive(Clone, Debug)]
pub struct Vault {
    pub cfg: Config,
}

/* ---------- helpers (free functions) ---------- */

fn project_file_path(base: &Path, key: &str) -> PathBuf {
    base.join("projects").join(format!("{key}.md"))
}

fn list_project_files(base: &Path) -> Vec<PathBuf> {
    let dir = base.join("projects");
    if !dir.exists() {
        return vec![];
    }
    WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.path().to_path_buf())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("md"))
        .collect()
}

// locate a task file by frontmatter.id
fn find_task_file_by_id(base: &Path, id: &str) -> Option<PathBuf> {
    let tasks_dir = base.join("tasks");
    if !tasks_dir.exists() {
        return None;
    }
    for entry in WalkDir::new(tasks_dir).into_iter().filter_map(|e| e.ok()) {
        let p = entry.path();
        if p.extension().and_then(|s| s.to_str()) == Some("md") {
            if let Ok(s) = fs::read_to_string(p) {
                if let Ok(fm) = extract_frontmatter(&s) {
                    if fm.id == id {
                        return Some(p.to_path_buf());
                    }
                }
            }
        }
    }
    None
}

/* ---------- Vault impl ---------- */

impl Vault {
    pub fn new(cfg: Config) -> Result<Self> {
        Ok(Self { cfg })
    }

    pub fn init_dirs(&self) -> Result<()> {
        let base = &self.cfg.vault_path;
        fs::create_dir_all(base.join("projects"))?;
        fs::create_dir_all(base.join("tasks"))?;
        Ok(())
    }

    /* ----- Projects API ----- */

    pub fn create_project(&self, p: ProjectNew) -> Result<String> {
        self.init_dirs().ok();
        let key = slug::slugify(&p.title);
        let now = OffsetDateTime::now_utc();
        let fm = ProjectFrontmatter {
            key: key.clone(),
            title: p.title,
            status: "active".into(),
            tags: p.tags,
            created: Some(now.format(&Rfc3339).unwrap()),
            updated: Some(now.format(&Rfc3339).unwrap()),
            description: None,
        };
        let md = format!("---\n{}---\n", serde_yaml::to_string(&fm)?);
        let path = project_file_path(&self.cfg.vault_path, &key);
        fs::create_dir_all(path.parent().unwrap())?;
        fs::write(&path, md)?;
        Ok(key)
    }

    pub fn list_projects(&self) -> Result<Vec<Project>> {
        let mut out = Vec::new();
        for p in list_project_files(&self.cfg.vault_path) {
            if let Ok(pr) = Project::from_md_file(&p) {
                out.push(pr);
            }
        }
        // updated desc, then title asc
        out.sort_by(|a, b| b.updated.cmp(&a.updated).then(a.title.cmp(&b.title)));
        Ok(out)
    }

    pub fn get_project(&self, key: &str) -> Result<Option<Project>> {
        let path = project_file_path(&self.cfg.vault_path, key);
        if !path.exists() {
            return Ok(None);
        }
        Ok(Some(Project::from_md_file(&path)?))
    }

    /* ----- Tasks API ----- */

    pub fn list_tasks(&self, _project: Option<&str>) -> Result<Vec<Task>> {
        let mut out = Vec::new();
        let tasks_dir = self.cfg.vault_path.join("tasks");
        if !tasks_dir.exists() {
            return Ok(out);
        }
        for entry in WalkDir::new(tasks_dir).into_iter().filter_map(|e| e.ok()) {
            if entry.path().extension().and_then(|s| s.to_str()) == Some("md") {
                if let Ok(t) = Task::from_md_file(entry.path()) {
                    out.push(t);
                }
            }
        }
        // naive sort by updated desc
        out.sort_by(|a, b| b.updated.cmp(&a.updated));
        Ok(out)
    }

    pub fn create_task(&self, t: TaskNew) -> Result<String> {
        self.init_dirs().ok();
        let id = Ulid::new().to_string();
        let now = OffsetDateTime::now_utc();
        let slug = slug::slugify(&t.title);
        let date = now.date();
        let y = date.year();
        let m = u8::from(date.month());
        let file = self
            .cfg
            .vault_path
            .join("tasks")
            .join(format!("{y:04}"))
            .join(format!("{m:02}"))
            .join(format!(
                "{:04}-{:02}-{:02}--{}--{}.md",
                y,
                m,
                date.day(),
                slug,
                id
            ));
        fs::create_dir_all(file.parent().unwrap())?;

        let frontmatter = Frontmatter {
            id: id.clone(),
            key: slug,
            title: t.title,
            status: "todo".into(),
            project: t.project,
            tags: t.tags,
            priority: "none".into(),
            due: t.due,
            created: Some(now.format(&Rfc3339).unwrap()),
            updated: Some(now.format(&Rfc3339).unwrap()),
            parent: None,
        };
        let md = frontmatter.to_markdown("---\n")?;
        let mut f = fs::File::create(&file)?;
        f.write_all(md.as_bytes())?;
        Ok(id)
    }

    pub fn set_status(&self, id: &str, status: Status) -> Result<()> {
        let path = find_task_file_by_id(&self.cfg.vault_path, id)
            .with_context(|| format!("task {id} not found"))?;
        let content = fs::read_to_string(&path)?;
        let (mut fm, body) =
            extract_frontmatter_and_body(&content).with_context(|| "invalid frontmatter")?;
        fm.status = status.as_str().to_string();
        fm.updated = Some(OffsetDateTime::now_utc().format(&Rfc3339).unwrap());
        let new = format!("---\n{}---\n{}", serde_yaml::to_string(&fm)?, body);
        fs::write(&path, new)?;
        Ok(())
    }

    pub fn cycle_status(&self, id: &str, direction: i8) -> Result<Status> {
        let path = find_task_file_by_id(&self.cfg.vault_path, id)
            .with_context(|| format!("task {id} not found"))?;
        let content = fs::read_to_string(&path)?;
        let (fm, _) = extract_frontmatter_and_body(&content)?;
        let cur = Status::from_str(&fm.status);
        let next = if direction >= 0 {
            cur.next()
        } else {
            cur.prev()
        };
        self.set_status(id, next.clone())?;
        Ok(next)
    }

    pub fn set_due(&self, id: &str, due: &str) -> Result<()> {
        let path = find_task_file_by_id(&self.cfg.vault_path, id)
            .with_context(|| format!("task {id} not found"))?;
        let content = fs::read_to_string(&path)?;
        let (mut fm, body) = extract_frontmatter_and_body(&content)?;
        fm.due = Some(due.to_string());
        fm.updated = Some(OffsetDateTime::now_utc().format(&Rfc3339).unwrap());
        let new = format!("---\n{}---\n{}", serde_yaml::to_string(&fm)?, body);
        fs::write(&path, new)?;
        Ok(())
    }

    pub fn set_tags_csv(&self, id: &str, csv: &str) -> Result<()> {
        let path = find_task_file_by_id(&self.cfg.vault_path, id)
            .with_context(|| format!("task {id} not found"))?;
        let content = fs::read_to_string(&path)?;
        let (mut fm, body) = extract_frontmatter_and_body(&content)?;
        let tags: Vec<String> = csv
            .split([',', ' '])
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.trim_start_matches('+').to_string())
            .collect();
        fm.tags = tags;
        fm.updated = Some(OffsetDateTime::now_utc().format(&Rfc3339).unwrap());
        let new = format!("---\n{}---\n{}", serde_yaml::to_string(&fm)?, body);
        fs::write(&path, new)?;
        Ok(())
    }

    pub fn rename_title(&self, id: &str, new_title: &str) -> Result<()> {
        let path = find_task_file_by_id(&self.cfg.vault_path, id)
            .with_context(|| format!("task {id} not found"))?;
        let content = fs::read_to_string(&path)?;
        let (mut fm, body) = extract_frontmatter_and_body(&content)?;
        let new_slug = slug::slugify(new_title);

        // update frontmatter
        fm.title = new_title.to_string();
        fm.key = new_slug.clone();
        fm.updated = Some(OffsetDateTime::now_utc().format(&Rfc3339).unwrap());

        // write updated frontmatter/body first
        let updated = format!("---\n{}---\n{}", serde_yaml::to_string(&fm)?, body);
        fs::write(&path, updated)?;

        // rename file to: YYYY-MM-DD--slug--ID.md
        if let (Some(parent), Some(stem)) = (path.parent(), path.file_stem().and_then(|s| s.to_str()))
        {
            let parts: Vec<&str> = stem.split("--").collect();
            if parts.len() >= 3 {
                let date_part = parts[0];
                let new_name = format!("{date}--{slug}--{id}.md", date = date_part, slug = new_slug, id = id);
                let new_path = parent.join(new_name);
                if new_path != path {
                    let _ = fs::rename(&path, &new_path);
                }
            }
        }
        Ok(())
    }
}

/* ---------- Task types ---------- */

#[derive(Clone, Debug)]
pub struct TaskNew {
    pub title: String,
    pub project: String,
    pub due: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Frontmatter {
    pub id: String,
    pub key: String,
    pub title: String,
    pub status: String,
    pub project: String,
    pub tags: Vec<String>,
    pub priority: String,
    pub due: Option<String>,
    pub created: Option<String>,
    pub updated: Option<String>,
    pub parent: Option<String>,
}

impl Frontmatter {
    pub fn to_markdown(&self, sep: &str) -> Result<String> {
        let yml = serde_yaml::to_string(self)?;
        Ok(format!("{sep}{yml}{sep}\n"))
    }
}

#[derive(Clone, Debug)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub status: String,
    pub project: String,
    pub updated: String,
}

impl Task {
    pub fn from_md_file(path: &Path) -> Result<Self> {
        let s = fs::read_to_string(path)?;
        let re = Regex::new(r"(?s)^---\n(.*?)\n---").unwrap();
        let caps = re.captures(&s).context("no frontmatter")?;
        let fm: Frontmatter = serde_yaml::from_str(&caps[1])?;
        Ok(Task {
            id: fm.id,
            title: fm.title,
            status: fm.status,
            project: fm.project,
            updated: fm.updated.unwrap_or_default(),
        })
    }
}

fn extract_frontmatter(s: &str) -> Result<Frontmatter> {
    let re = Regex::new(r"(?s)^---\n(.*?)\n---")?;
    let caps = re.captures(s).context("no frontmatter")?;
    let fm: Frontmatter = serde_yaml::from_str(&caps[1])?;
    Ok(fm)
}

fn extract_frontmatter_and_body(s: &str) -> Result<(Frontmatter, String)> {
    let re = Regex::new(r"(?s)^---\n(.*?)\n---\n?(.*)$")?;
    let caps = re.captures(s).context("no frontmatter")?;
    let fm: Frontmatter = serde_yaml::from_str(&caps[1])?;
    let body = caps.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();
    Ok((fm, body))
}

/* ---------- Status ---------- */

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    Todo,
    Doing,
    Done,
}

impl Status {
    pub fn as_str(&self) -> &'static str {
        match self {
            Status::Todo => "todo",
            Status::Doing => "doing",
            Status::Done => "done",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "doing" | "in-progress" | "in_progress" => Status::Doing,
            "done" => Status::Done,
            _ => Status::Todo,
        }
    }
    pub fn next(&self) -> Self {
        match self {
            Status::Todo => Status::Doing,
            Status::Doing => Status::Done,
            Status::Done => Status::Todo,
        }
    }
    pub fn prev(&self) -> Self {
        match self {
            Status::Todo => Status::Done,
            Status::Doing => Status::Todo,
            Status::Done => Status::Doing,
        }
    }
}

/* ---------- Project types ---------- */

#[derive(Clone, Debug)]
pub struct ProjectNew {
    pub title: String,
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectFrontmatter {
    pub key: String, // slug
    pub title: String,
    pub status: String, // "active" | "archived"
    pub tags: Vec<String>,
    pub created: Option<String>,
    pub updated: Option<String>,
    pub description: Option<String>, // optional markdown body below fm
}

#[derive(Clone, Debug)]
pub struct Project {
    pub key: String,
    pub title: String,
    pub status: String,
    pub tags: Vec<String>,
    pub updated: String,
}

impl Project {
    fn from_md_file(path: &Path) -> Result<Self> {
        let s = fs::read_to_string(path)?;
        let re = Regex::new(r"(?s)^---\n(.*?)\n---")?;
        let caps = re.captures(&s).context("no project frontmatter")?;
        let fm: ProjectFrontmatter = serde_yaml::from_str(&caps[1])?;
        Ok(Self {
            key: fm.key,
            title: fm.title,
            status: fm.status,
            tags: fm.tags,
            updated: fm.updated.unwrap_or_default(),
        })
    }
}

