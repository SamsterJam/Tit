//! `start`, `end`, `status`, `reset`.

use owo_colors::OwoColorize;

use crate::error::TitError;
use crate::model::Session;
use crate::store::Store;
use crate::timefmt;
use crate::ui;

pub fn start(store: &Store) -> Result<(), TitError> {
    let mut project = store.require_project()?;

    if project
        .uncommitted
        .last()
        .is_some_and(Session::is_in_progress)
    {
        return Err(TitError::SessionAlreadyRunning);
    }

    project.uncommitted.push(Session::started_now());
    project.save_uncommitted()?;
    println!("{}", "Session started.".green());
    Ok(())
}

pub fn end(store: &Store) -> Result<(), TitError> {
    let mut project = store.require_project()?;

    let in_progress = project
        .uncommitted
        .last()
        .is_some_and(Session::is_in_progress);
    if !in_progress {
        return Err(TitError::NoSessionInProgress);
    }

    let now = timefmt::now();
    let last = project.uncommitted.last_mut().unwrap();
    last.end = Some(timefmt::to_storage(now));
    let duration = last.duration(now)?;
    let message = format!(
        "Session stopped. Duration: {}",
        timefmt::duration_hms(duration)
    );
    last.message = Some(message.clone());

    project.save_uncommitted()?;
    println!("{}", message.green());
    Ok(())
}

pub fn status(store: &Store) -> Result<(), TitError> {
    let project = store.require_project()?;
    let now = timefmt::now();
    let tag = format!("({})", project.name);
    let tag = tag.blue().bold().to_string();

    let running = project
        .uncommitted
        .last()
        .is_some_and(Session::is_in_progress);

    if !running {
        let count = project.uncommitted.len();
        if count == 0 {
            println!(
                "{} {}",
                tag,
                "No session in progress. 0 uncommitted session(s).".green()
            );
        } else {
            println!("{tag} No session in progress. {count} uncommitted session(s).");
            print_sessions(&project.uncommitted, now)?;
        }
    } else {
        let last = project.uncommitted.last().unwrap();
        let elapsed = last.duration(now)?;
        println!(
            "{}",
            format!(
                "Session in progress. Started at: {} (Elapsed: {})",
                timefmt::display(last.start_dt()?),
                timefmt::duration_hms(elapsed)
            )
            .yellow()
            .bold()
        );
        let others = &project.uncommitted[..project.uncommitted.len() - 1];
        if !others.is_empty() {
            println!("{} uncommitted session(s).", others.len());
        }
        print_sessions(others, now)?;
    }
    Ok(())
}

pub fn reset(store: &Store) -> Result<(), TitError> {
    let mut project = store.require_project()?;

    if project.uncommitted.is_empty() {
        println!("{}", "No uncommitted sessions to reset.".yellow());
        return Ok(());
    }

    let now = timefmt::now();
    println!("{}", "Uncommitted sessions:".yellow());
    print_sessions(&project.uncommitted, now)?;

    if !ui::confirm("Are you sure you want to discard these sessions? [y/N]: ") {
        println!("{}", "Reset aborted.".green());
        return Ok(());
    }

    project.uncommitted.clear();
    project.save_uncommitted()?;
    println!("{}", "Uncommitted sessions have been discarded.".green());
    Ok(())
}

fn print_sessions(sessions: &[Session], now: chrono::NaiveDateTime) -> Result<(), TitError> {
    for session in sessions {
        let line = format!(
            "  - Start: {}, Duration: {}",
            timefmt::display(session.start_dt()?),
            timefmt::duration_hms(session.duration(now)?)
        );
        println!("{}", line.red());
    }
    Ok(())
}
