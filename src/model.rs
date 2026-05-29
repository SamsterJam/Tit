//! Data model: `Session` and `Commit`, serialized to the exact JSON layout the
//! original Python tool used, plus duration helpers and commit hashing.

use std::io;

use chrono::{Duration, NaiveDateTime};
use serde::{Deserialize, Serialize};
use serde_json::ser::Formatter;
use sha1::{Digest, Sha1};

use crate::error::TitError;
use crate::timefmt;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Session {
    pub start: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl Session {
    pub fn started_now() -> Session {
        Session {
            start: timefmt::to_storage(timefmt::now()),
            end: None,
            message: None,
        }
    }

    /// A session is in progress when `end` is absent, null, or the legacy
    /// sentinel string `"In Progress"`.
    pub fn is_in_progress(&self) -> bool {
        match &self.end {
            None => true,
            Some(e) => e == "In Progress",
        }
    }

    pub fn start_dt(&self) -> Result<NaiveDateTime, TitError> {
        timefmt::parse(&self.start)
    }

    /// The effective end time: `now` if the session is still running.
    pub fn end_dt(&self, now: NaiveDateTime) -> Result<NaiveDateTime, TitError> {
        if self.is_in_progress() {
            Ok(now)
        } else {
            timefmt::parse(self.end.as_ref().unwrap())
        }
    }

    pub fn duration(&self, now: NaiveDateTime) -> Result<Duration, TitError> {
        Ok(self.end_dt(now)? - self.start_dt()?)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Commit {
    pub sessions: Vec<Session>,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub hash: String,
}

impl Commit {
    pub fn new(sessions: Vec<Session>, message: String) -> Commit {
        let hash = commit_hash(&sessions);
        Commit {
            sessions,
            message,
            hash,
        }
    }

    pub fn duration(&self, now: NaiveDateTime) -> Result<Duration, TitError> {
        let mut total = Duration::zero();
        for session in &self.sessions {
            total += session.duration(now)?;
        }
        Ok(total)
    }

    /// The commit's timestamp = the start of its first session.
    pub fn commit_time(&self) -> Result<Option<NaiveDateTime>, TitError> {
        match self.sessions.first() {
            Some(s) => Ok(Some(s.start_dt()?)),
            None => Ok(None),
        }
    }
}

/// Resolve a (possibly partial) commit hash against the given commit pools.
/// Returns the unique full hash, or an error if none / multiple match.
pub fn resolve_hash(prefix: &str, pools: &[&[Commit]]) -> Result<String, TitError> {
    let mut matches: Vec<&str> = Vec::new();
    for pool in pools {
        for commit in *pool {
            if commit.hash.starts_with(prefix) && !matches.contains(&commit.hash.as_str()) {
                matches.push(&commit.hash);
            }
        }
    }
    match matches.len() {
        1 => Ok(matches[0].to_string()),
        0 => Err(TitError::CommitNotFound(prefix.to_string())),
        _ => Err(TitError::AmbiguousHash(prefix.to_string())),
    }
}

/// SHA-1 of the sessions, byte-compatible with the Python tool's
/// `sha1(json.dumps(sessions, sort_keys=True).encode())`.
pub fn commit_hash(sessions: &[Session]) -> String {
    let value = serde_json::to_value(sessions).expect("sessions serialize to JSON");
    let mut buf = Vec::new();
    let mut ser = serde_json::Serializer::with_formatter(&mut buf, PyFormatter);
    value
        .serialize(&mut ser)
        .expect("JSON value serializes to buffer");

    let mut hasher = Sha1::new();
    hasher.update(&buf);
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Replicates Python's `json.dumps` default separators (`", "` and `": "`).
/// Key ordering is already sorted because serde_json's `Map` is a `BTreeMap`
/// (the `preserve_order` feature is off), matching `sort_keys=True`.
struct PyFormatter;

impl Formatter for PyFormatter {
    fn begin_array_value<W: ?Sized + io::Write>(
        &mut self,
        writer: &mut W,
        first: bool,
    ) -> io::Result<()> {
        if first {
            Ok(())
        } else {
            writer.write_all(b", ")
        }
    }

    fn begin_object_key<W: ?Sized + io::Write>(
        &mut self,
        writer: &mut W,
        first: bool,
    ) -> io::Result<()> {
        if first {
            Ok(())
        } else {
            writer.write_all(b", ")
        }
    }

    fn begin_object_value<W: ?Sized + io::Write>(&mut self, writer: &mut W) -> io::Result<()> {
        writer.write_all(b": ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(start: &str, end: Option<&str>, message: Option<&str>) -> Session {
        Session {
            start: start.to_string(),
            end: end.map(str::to_string),
            message: message.map(str::to_string),
        }
    }

    #[test]
    fn in_progress_detection() {
        assert!(s("2024-01-01T00:00:00", None, None).is_in_progress());
        assert!(s("2024-01-01T00:00:00", Some("In Progress"), None).is_in_progress());
        assert!(!s("2024-01-01T00:00:00", Some("2024-01-01T01:00:00"), None).is_in_progress());
    }

    #[test]
    fn closed_session_duration() {
        let sess = s("2024-01-01T00:00:00", Some("2024-01-01T01:30:00"), None);
        let now = timefmt::now();
        assert_eq!(sess.duration(now).unwrap(), Duration::minutes(90));
    }

    #[test]
    fn resolve_partial_hash() {
        let commits = vec![Commit::new(
            vec![s("2024-01-01T00:00:00", Some("2024-01-01T01:00:00"), None)],
            "a".into(),
        )];
        let full = &commits[0].hash.clone();
        let resolved = resolve_hash(&full[..7], &[&commits]).unwrap();
        assert_eq!(&resolved, full);
    }

    #[test]
    fn resolve_missing_hash_errors() {
        let commits: Vec<Commit> = vec![];
        assert!(matches!(
            resolve_hash("deadbeef", &[&commits]),
            Err(TitError::CommitNotFound(_))
        ));
    }

    // Pins our hash to a value computed by the reference Python implementation:
    //   import json, hashlib
    //   s=[{"start":"2024-01-01T00:00:00","end":"2024-01-01T01:00:00"}]
    //   hashlib.sha1(json.dumps(s, sort_keys=True).encode()).hexdigest()
    #[test]
    fn hash_matches_python() {
        let sessions = vec![s("2024-01-01T00:00:00", Some("2024-01-01T01:00:00"), None)];
        assert_eq!(
            commit_hash(&sessions),
            "749ca23c1abe1f9e3985a56714b3e4d6ee964bf8"
        );

        // Real commits carry a `message` on each stopped session; the sorted
        // keys become end, message, start.
        let with_msg = vec![s(
            "2024-01-01T00:00:00",
            Some("2024-01-01T01:00:00"),
            Some("Session stopped. Duration: 01:00:00"),
        )];
        assert_eq!(
            commit_hash(&with_msg),
            "42fb403be07e0e1609391fe6e0854cbea824d722"
        );
    }
}
