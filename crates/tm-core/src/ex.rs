use anyhow::{bail, Result};
use std::str::FromStr;

use crate::{Status};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExCommand {
    /// :new "Title here" project:slug +tag1 +tag2 due:2025-09-01
    New {
        title: String,
        project: Option<String>,
        tags: Vec<String>,
        due: Option<String>,
    },
    /// :status <id?> (todo|doing|done|next|prev)
    /// id optional → UI may apply to selected task
    Status { id: Option<String>, set: StatusSet },
    /// :open project:<slug>
    OpenProject { key: String },
    /// :project.new "Title" +tag
    ProjectNew { title: String, tags: Vec<String> },
    /// :config.reload
    ConfigReload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusSet {
    Todo,
    Doing,
    Done,
    Next,
    Prev,
}

impl FromStr for StatusSet {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        Ok(match s {
            "todo" => StatusSet::Todo,
            "doing" | "in-progress" | "in_progress" => StatusSet::Doing,
            "done" => StatusSet::Done,
            "next" => StatusSet::Next,
            "prev" => StatusSet::Prev,
            _ => bail!("unknown status '{s}'"),
        })
    }
}

/// Very small tokenizer that respects double quotes for a single field (title).
fn tokenize(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    for c in input.chars() {
        match c {
            '"' => { in_quotes = !in_quotes; }
            ' ' if !in_quotes => {
                if !cur.is_empty() { out.push(cur.clone()); cur.clear(); }
            }
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() { out.push(cur); }
    out
}

/// Parse ex-line (string without the leading colon)
pub fn parse_ex(line: &str) -> Result<ExCommand> {
    let line = line.trim();
    if line.is_empty() { bail!("empty command"); }

    // config.reload special-case
    if line == "config.reload" {
        return Ok(ExCommand::ConfigReload);
    }

    let mut toks = tokenize(line);
    let cmd = toks.remove(0);

    match cmd.as_str() {
        "new" => {
            // Extract fields
            let mut title = String::new();
            let mut project = None;
            let mut tags = Vec::new();
            let mut due = None;

            // first non-flag token that contains spaces must be quoted → already intact from tokenizer
            if !toks.is_empty() && !toks[0].starts_with("project:") && !toks[0].starts_with('+') && !toks[0].starts_with("due:") {
                title = toks.remove(0);
            }

            for t in toks {
                if let Some(rest) = t.strip_prefix("project:") {
                    project = Some(rest.to_string());
                } else if let Some(rest) = t.strip_prefix("due:") {
                    due = Some(rest.to_string());
                } else if let Some(rest) = t.strip_prefix('+') {
                    if !rest.is_empty() { tags.push(rest.to_string()); }
                } else if title.is_empty() {
                    title = t;
                }
            }

            if title.is_empty() { bail!(":new requires a title (quoted if it has spaces)"); }

            Ok(ExCommand::New { title, project, tags, due })
        }

        "status" => {
            // forms:
            // :status done           (no id → UI uses selected)
            // :status <id> done
            let (id_opt, set_str) = if toks.len() == 1 {
                (None, toks[0].as_str())
            } else if toks.len() >= 2 {
                (Some(toks[0].clone()), toks[1].as_str())
            } else {
                bail!("usage: :status [<id>] (todo|doing|done|next|prev)")
            };
            let set = StatusSet::from_str(set_str)?;
            Ok(ExCommand::Status { id: id_opt, set })
        }

        "open" => {
            // :open project:<slug>
            let mut key = None;
            for t in toks {
                if let Some(rest) = t.strip_prefix("project:") {
                    key = Some(rest.to_string());
                }
            }
            Ok(ExCommand::OpenProject { key: key.unwrap_or_default() })
        }

        "project.new" => {
            // :project.new "Title" +tag ...
            let mut title = String::new();
            let mut tags = Vec::new();

            if !toks.is_empty() && !toks[0].starts_with('+') {
                title = toks.remove(0);
            }
            for t in toks {
                if let Some(rest) = t.strip_prefix('+') {
                    if !rest.is_empty() { tags.push(rest.to_string()); }
                } else if title.is_empty() {
                    title = t;
                }
            }
            if title.is_empty() { bail!(":project.new requires a title"); }
            Ok(ExCommand::ProjectNew { title, tags })
        }

        _ => bail!("unknown command '{cmd}'"),
    }
}

