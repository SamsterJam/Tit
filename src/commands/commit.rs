//! `commit`, `log`, `rm`, `purge`, `edit`.

use std::fs;
use std::process::Command;

use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};

use crate::commands::report::CommitRange;
use crate::error::TitError;
use crate::model::{self, Commit, Session};
use crate::store::{Project, Store};
use crate::timefmt;
use crate::ui;

pub fn commit(store: &Store, message: &str) -> Result<(), TitError> {
    let mut project = store.require_project()?;

    let ready =
        !project.uncommitted.is_empty() && !project.uncommitted.last().unwrap().is_in_progress();
    if !ready {
        return Err(TitError::NoSessionToCommit);
    }

    let sessions = std::mem::take(&mut project.uncommitted);
    let new_commit = Commit::new(sessions, message.to_string());
    project.committed.push(new_commit);

    project.save_uncommitted()?;
    project.save_committed()?;
    println!(
        "{}",
        format!("Session committed with message: {message}")
            .green()
            .bold()
    );
    Ok(())
}

pub fn log(
    store: &Store,
    all: bool,
    verbose: bool,
    from_commit: Option<&str>,
    to_commit: Option<&str>,
) -> Result<(), TitError> {
    let project = store.require_project()?;
    let now = timefmt::now();

    let pools: Vec<&[Commit]> = if all {
        vec![&project.committed, &project.deleted]
    } else {
        vec![&project.committed]
    };
    let range = CommitRange::resolve(&pools, from_commit, to_commit)?;

    for commit in &project.committed {
        if !range.contains_commit(commit)? {
            continue;
        }
        if verbose {
            print_verbose_commit(commit, now)?;
        } else {
            print_compact_commit(commit, now)?;
        }
    }

    if all {
        let mut uncommitted: Vec<&Session> = Vec::new();
        for session in &project.uncommitted {
            if range.contains(session.start_dt()?) {
                uncommitted.push(session);
            }
        }

        if !uncommitted.is_empty() {
            println!("Uncommitted Sessions:");
            for session in uncommitted {
                let line = format!(
                    "  - Start: {}, Duration: {}",
                    timefmt::display(session.start_dt()?),
                    timefmt::duration_hms(session.duration(now)?)
                );
                println!("{}", line.red());
            }
        }

        let mut deleted: Vec<&Commit> = Vec::new();
        for commit in &project.deleted {
            if range.contains_commit(commit)? {
                deleted.push(commit);
            }
        }

        if !deleted.is_empty() {
            println!("Deleted Sessions:");
            for commit in deleted {
                print_deleted_commit(commit, now)?;
            }
        }
    }
    Ok(())
}

fn print_compact_commit(commit: &Commit, now: chrono::NaiveDateTime) -> Result<(), TitError> {
    let short = &commit.hash[..commit.hash.len().min(7)];
    println!("[{}] {}", short.yellow(), commit.message.green().bold());
    for session in &commit.sessions {
        println!(
            "└─ {} | Start: {}",
            timefmt::duration_hms(session.duration(now)?).blue(),
            timefmt::display(session.start_dt()?)
        );
    }
    println!();
    Ok(())
}

fn print_verbose_commit(commit: &Commit, now: chrono::NaiveDateTime) -> Result<(), TitError> {
    println!("{}", format!("commit {}", commit.hash).yellow().bold());
    if let Some(time) = commit.commit_time()? {
        println!("Date: {}\n", timefmt::commit_date(time));
    }
    println!("{}", format!("    {}", commit.message).green());
    println!("    Sessions ({}):", commit.sessions.len());
    for session in &commit.sessions {
        println!(
            "      - Start: {}, Duration: {}",
            timefmt::display(session.start_dt()?),
            timefmt::duration_hms(session.duration(now)?).bold()
        );
    }
    println!();
    Ok(())
}

fn print_deleted_commit(commit: &Commit, now: chrono::NaiveDateTime) -> Result<(), TitError> {
    println!(
        "{}",
        format!("[Removed] commit {}", commit.hash).red().dimmed()
    );
    if let Some(time) = commit.commit_time()? {
        println!(
            "{}",
            format!("Date: {}\n", timefmt::commit_date(time)).dimmed()
        );
    }
    println!("{}", format!("    {}", commit.message).dimmed());
    println!(
        "{}",
        format!("    Sessions ({}):", commit.sessions.len()).dimmed()
    );
    for session in &commit.sessions {
        let line = format!(
            "      - Start: {}, Duration: {}",
            timefmt::display(session.start_dt()?),
            timefmt::duration_hms(session.duration(now)?)
        );
        println!("{}", line.dimmed());
    }
    println!();
    Ok(())
}

pub fn rm(store: &Store, hash: &str) -> Result<(), TitError> {
    let mut project = store.require_project()?;
    let full = model::resolve_hash(hash, &[&project.committed, &project.deleted])?;

    if let Some(pos) = project.committed.iter().position(|c| c.hash == full) {
        let commit = project.committed.remove(pos);
        project.deleted.push(commit);
        project.save_committed()?;
        project.save_deleted()?;
        println!("{}", format!("Commit '{full}' has been removed.").green());
        return Ok(());
    }

    // Already in the deleted pool: a second `rm` purges it (with confirmation).
    purge_resolved(&mut project, &full)
}

pub fn purge(store: &Store, hash: &str) -> Result<(), TitError> {
    let mut project = store.require_project()?;
    let full = model::resolve_hash(hash, &[&project.committed, &project.deleted])?;
    purge_resolved(&mut project, &full)
}

fn purge_resolved(project: &mut Project, full: &str) -> Result<(), TitError> {
    let prompt = format!(
        "Are you sure you want to permanently delete commit '{full}'? \
         This action cannot be undone. [y/N]: "
    );
    if !ui::confirm(&prompt) {
        println!("{}", "Purge aborted.".green());
        return Ok(());
    }

    if let Some(pos) = project.committed.iter().position(|c| c.hash == full) {
        project.committed.remove(pos);
        project.save_committed()?;
    } else if let Some(pos) = project.deleted.iter().position(|c| c.hash == full) {
        project.deleted.remove(pos);
        project.save_deleted()?;
    } else {
        return Err(TitError::CommitNotFound(full.to_string()));
    }

    println!(
        "{}",
        format!("Commit '{full}' has been permanently deleted.").green()
    );
    Ok(())
}

// ---- edit ---------------------------------------------------------------

#[derive(Serialize, Deserialize)]
struct EditDoc {
    message: String,
    #[serde(default, rename = "session")]
    sessions: Vec<EditSession>,
}

#[derive(Serialize, Deserialize)]
struct EditSession {
    start: String,
    end: String,
}

const EDIT_HEADER: &str = "\
# Edit the commit below, then save and close to apply.
# Times use the format YYYY-MM-DDTHH:MM:SS (24-hour).
# Delete all [[session]] blocks to delete the commit entirely.

";

pub fn edit(store: &Store, hash: &str) -> Result<(), TitError> {
    let mut project = store.require_project()?;
    let full = model::resolve_hash(hash, &[&project.committed])?;
    let pos = project
        .committed
        .iter()
        .position(|c| c.hash == full)
        .ok_or_else(|| TitError::CommitNotFound(full.clone()))?;

    // Build the editable TOML document from the current commit.
    let doc = EditDoc {
        message: project.committed[pos].message.clone(),
        sessions: project.committed[pos]
            .sessions
            .iter()
            .map(|s| {
                Ok(EditSession {
                    start: timefmt::to_storage(s.start_dt()?),
                    end: timefmt::to_storage(s.end_dt(timefmt::now())?),
                })
            })
            .collect::<Result<_, TitError>>()?,
    };
    let body = toml::to_string_pretty(&doc).map_err(|e| TitError::EditParse(e.to_string()))?;
    let original = format!("{EDIT_HEADER}{body}");

    // Write to a temp file, open the editor, read it back.
    let path = std::env::temp_dir().join(format!("tit-edit-{}.toml", &full[..full.len().min(7)]));
    fs::write(&path, &original)?;

    let editor = ui::find_editor()?;
    Command::new(&editor).arg(&path).status()?;

    let edited = fs::read_to_string(&path)?;
    let _ = fs::remove_file(&path);

    if edited == original {
        println!("{}", "No changes made to the commit.".yellow());
        return Ok(());
    }

    let parsed: EditDoc =
        toml::from_str(&edited).map_err(|e| TitError::EditParse(e.message().to_string()))?;

    // Validate and convert sessions.
    let mut sessions = Vec::with_capacity(parsed.sessions.len());
    for s in &parsed.sessions {
        let start = timefmt::parse(&s.start)?;
        let end = timefmt::parse(&s.end)?;
        if end < start {
            return Err(TitError::EditParse(format!(
                "session end '{}' is before start '{}'",
                s.end, s.start
            )));
        }
        sessions.push(Session {
            start: timefmt::to_storage(start),
            end: Some(timefmt::to_storage(end)),
            message: None,
        });
    }

    if sessions.is_empty() {
        project.committed.remove(pos);
        println!(
            "{}",
            format!("Commit '{full}' has been deleted because no sessions were left.").yellow()
        );
    } else {
        let commit = &mut project.committed[pos];
        commit.hash = model::commit_hash(&sessions);
        commit.sessions = sessions;
        commit.message = parsed.message;
        println!("{}", format!("Commit '{full}' has been edited.").green());
    }

    project.save_committed()?;
    Ok(())
}
