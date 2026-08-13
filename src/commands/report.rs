//! `time`, `today`, `export`.

use std::collections::HashSet;

use chrono::{Duration, NaiveDateTime};
use comfy_table::{Attribute, Cell, Color, Table, presets::ASCII_FULL};
use owo_colors::OwoColorize;

use crate::cli::ExportFormat;
use crate::error::TitError;
use crate::model::Commit;
use crate::store::{Project, Store};
use crate::timefmt;

/// Hashes of deleted commits, so they can be excluded from totals.
fn deleted_hashes(project: &Project) -> HashSet<&str> {
    project.deleted.iter().map(|c| c.hash.as_str()).collect()
}

pub fn time(store: &Store) -> Result<(), TitError> {
    let project = store.require_project()?;
    let now = timefmt::now();
    let deleted = deleted_hashes(&project);

    let mut committed = Duration::zero();
    for commit in &project.committed {
        if deleted.contains(commit.hash.as_str()) {
            continue;
        }
        committed += commit.duration(now)?;
    }

    let mut uncommitted = Duration::zero();
    for session in &project.uncommitted {
        uncommitted += session.duration(now)?;
    }

    if uncommitted > Duration::zero() {
        println!(
            "{}",
            format!("Committed: {}", timefmt::duration_hms(committed)).green()
        );
        println!(
            "{}",
            format!("Uncommitted: {}", timefmt::duration_hms(uncommitted)).yellow()
        );
        println!("{}", "---------------------".blue());
        println!(
            "{}",
            format!("Total: {}", timefmt::duration_hms(committed + uncommitted))
                .blue()
                .bold()
        );
    } else {
        println!(
            "{}",
            format!("Total: {}", timefmt::duration_hms(committed))
                .green()
                .bold()
        );
    }
    Ok(())
}

pub fn today(store: &Store) -> Result<(), TitError> {
    let project = store.require_project()?;
    let now = timefmt::now();
    let today = now.date();
    let deleted = deleted_hashes(&project);

    let mut total = Duration::zero();

    for commit in &project.committed {
        if deleted.contains(commit.hash.as_str()) {
            continue;
        }
        for session in &commit.sessions {
            if session.start_dt()?.date() == today {
                total += session.duration(now)?;
            }
        }
    }
    for session in &project.uncommitted {
        if session.start_dt()?.date() == today {
            total += session.duration(now)?;
        }
    }

    println!(
        "{}",
        format!("Today's Total: {}", timefmt::duration_hms(total))
            .blue()
            .bold()
    );
    Ok(())
}

pub fn export(
    store: &Store,
    all: bool,
    format: ExportFormat,
    from_commit: Option<&str>,
    to_commit: Option<&str>,
    verbose: bool,
) -> Result<(), TitError> {
    let project = store.require_project()?;
    let now = timefmt::now();

    let range = CommitRange::resolve(&[&project.committed], from_commit, to_commit)?;

    let mut rows: Vec<(RowStyle, [String; 3])> = Vec::new();
    let mut total = Duration::zero();

    for commit in &project.committed {
        if !range.contains_commit(commit)? {
            continue;
        }
        let duration = commit.duration(now)?;
        total += duration;

        let date = if commit.sessions.len() == 1 {
            timefmt::display(commit.sessions[0].start_dt()?)
        } else {
            match commit.commit_time()? {
                Some(t) => timefmt::display(t),
                None => "Unknown".to_string(),
            }
        };
        rows.push((
            RowStyle::Committed,
            [
                commit.message.clone(),
                timefmt::duration_hms_padded(duration),
                date,
            ],
        ));

        if verbose && commit.sessions.len() > 1 {
            for (i, session) in commit.sessions.iter().enumerate() {
                rows.push((
                    RowStyle::Session,
                    [
                        format!("└── (Session {})", i + 1),
                        timefmt::duration_hms_padded(session.duration(now)?),
                        timefmt::display(session.start_dt()?),
                    ],
                ));
            }
        }
    }

    if all {
        for session in &project.uncommitted {
            let duration = session.duration(now)?;
            total += duration;
            rows.push((
                RowStyle::Uncommitted,
                [
                    "Uncommitted".to_string(),
                    timefmt::duration_hms_padded(duration),
                    timefmt::display(session.start_dt()?),
                ],
            ));
        }
    }

    rows.push((
        RowStyle::Total,
        [
            "Total".to_string(),
            timefmt::duration_hms_padded(total),
            timefmt::display(now),
        ],
    ));

    let headers = ["Message", "Duration", "Date"];
    match format {
        ExportFormat::Ascii => print_table(&headers, &rows),
        ExportFormat::Csv => write_csv(&project.name, &headers, &rows)?,
    }
    Ok(())
}

/// A `--from` / `--to` window, resolved to commit times.
pub(super) struct CommitRange {
    from: Option<NaiveDateTime>,
    to: Option<NaiveDateTime>,
}

impl CommitRange {
    pub(super) fn resolve(
        pools: &[&[Commit]],
        from_commit: Option<&str>,
        to_commit: Option<&str>,
    ) -> Result<Self, TitError> {
        let bound = |hash: Option<&str>| -> Result<Option<NaiveDateTime>, TitError> {
            let Some(h) = hash else {
                return Ok(None);
            };
            let full = crate::model::resolve_hash(h, pools)?;
            let commit = pools
                .iter()
                .flat_map(|pool| pool.iter())
                .find(|c| c.hash == full)
                .ok_or_else(|| TitError::CommitNotFound(full.clone()))?;
            commit.commit_time()
        };

        Ok(CommitRange {
            from: bound(from_commit)?,
            to: bound(to_commit)?,
        })
    }

    fn is_unbounded(&self) -> bool {
        self.from.is_none() && self.to.is_none()
    }

    pub(super) fn contains(&self, time: NaiveDateTime) -> bool {
        self.from.is_none_or(|f| time >= f) && self.to.is_none_or(|t| time <= t)
    }

    /// A commit with no sessions has no time, so it survives only an unbounded range.
    pub(super) fn contains_commit(&self, commit: &Commit) -> Result<bool, TitError> {
        if self.is_unbounded() {
            return Ok(true);
        }
        Ok(commit.commit_time()?.is_some_and(|t| self.contains(t)))
    }
}

/// How a table row is styled, mirroring the original tool.
#[derive(Copy, Clone)]
enum RowStyle {
    /// Committed commit: bold, default colour.
    Committed,
    /// Verbose per-session sub-row: plain.
    Session,
    /// Uncommitted session: yellow.
    Uncommitted,
    /// Grand total: green, bold.
    Total,
}

fn print_table(headers: &[&str; 3], rows: &[(RowStyle, [String; 3])]) {
    let mut table = Table::new();
    table.load_preset(ASCII_FULL);
    table.set_header(headers.to_vec());

    for (style, cells) in rows {
        let row = cells.iter().map(|text| style.cell(text));
        table.add_row(row);
    }
    println!("{table}");
}

impl RowStyle {
    fn cell(self, text: &str) -> Cell {
        let cell = Cell::new(text);
        match self {
            RowStyle::Committed => cell.add_attribute(Attribute::Bold),
            RowStyle::Session => cell,
            RowStyle::Uncommitted => cell.fg(Color::Yellow),
            RowStyle::Total => cell.fg(Color::Green).add_attribute(Attribute::Bold),
        }
    }
}

fn write_csv(
    project: &str,
    headers: &[&str; 3],
    rows: &[(RowStyle, [String; 3])],
) -> Result<(), TitError> {
    let file = format!("{project}_sessions.csv");
    let mut writer = csv::Writer::from_path(&file).map_err(csv_io_err)?;
    writer.write_record(headers).map_err(csv_io_err)?;
    for (_, cells) in rows {
        writer.write_record(cells).map_err(csv_io_err)?;
    }
    writer.flush()?;
    println!("{}", format!("Sessions exported to {file}").green());
    Ok(())
}

fn csv_io_err(e: csv::Error) -> TitError {
    TitError::Io(std::io::Error::other(e.to_string()))
}
