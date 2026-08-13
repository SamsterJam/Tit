//! End-to-end tests driving the real binary against a temp `TIT_DATA_DIR`.

use std::process::Command;

use assert_cmd::prelude::*;
use predicates::prelude::*;
use tempfile::TempDir;

/// Strip ANSI SGR escape sequences (`\x1b[...m`) from text.
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            // Skip until the terminating 'm' of the SGR sequence.
            for c in chars.by_ref() {
                if c == 'm' {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// A `tit` invocation pinned to an isolated data dir.
struct Tit {
    _dir: TempDir,
    data: std::path::PathBuf,
}

impl Tit {
    fn new() -> Tit {
        let dir = TempDir::new().unwrap();
        let data = dir.path().to_path_buf();
        Tit { _dir: dir, data }
    }

    fn cmd(&self) -> Command {
        let mut c = Command::cargo_bin("tit").unwrap();
        c.env("TIT_DATA_DIR", &self.data);
        c
    }

    fn run(&self, args: &[&str]) -> assert_cmd::assert::Assert {
        self.cmd().args(args).assert()
    }

    fn project_file(&self, project: &str, file: &str) -> std::path::PathBuf {
        self.data.join("projects").join(project).join(file)
    }
}

#[test]
fn errors_without_a_project() {
    let tit = Tit::new();
    tit.run(&["status"])
        .failure()
        .stderr(predicate::str::contains("No project selected"));
}

#[test]
fn full_lifecycle() {
    let tit = Tit::new();

    tit.run(&["init", "demo"])
        .success()
        .stdout(predicate::str::contains(
            "Initialized empty time tracking project 'demo'",
        ));

    tit.run(&["start"]).success();
    // A second start while running is rejected.
    tit.run(&["start"])
        .failure()
        .stderr(predicate::str::contains("already in progress"));

    tit.run(&["status"])
        .success()
        .stdout(predicate::str::contains("Session in progress"));

    tit.run(&["end"]).success();

    // Commit requires a message.
    tit.run(&["commit"])
        .failure()
        .stderr(predicate::str::contains("message is required"));

    tit.run(&["c", "-m", "did the thing"])
        .success()
        .stdout(predicate::str::contains("did the thing"));

    tit.run(&["log"])
        .success()
        .stdout(predicate::str::contains("did the thing"));

    tit.run(&["time"])
        .success()
        .stdout(predicate::str::contains("Total:"));

    // Committed JSON exists and is valid; uncommitted was cleared to `[]`.
    let committed = tit.project_file("demo", "committed_sessions.json");
    assert!(committed.exists());
    let parsed: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&committed).unwrap()).unwrap();
    assert_eq!(parsed.as_array().unwrap().len(), 1);

    let uncommitted =
        std::fs::read_to_string(tit.project_file("demo", "uncommitted_sessions.json")).unwrap();
    assert_eq!(uncommitted.trim(), "[]");
}

#[test]
fn rm_then_log_all_shows_deleted() {
    let tit = Tit::new();
    tit.run(&["init", "p"]).success();
    tit.run(&["start"]).success();
    tit.run(&["end"]).success();
    tit.run(&["c", "-m", "to be removed"]).success();

    // Grab the short hash from `log` (stripping ANSI color codes first).
    let out = tit.run(&["log"]).success().get_output().stdout.clone();
    let text = strip_ansi(&String::from_utf8(out).unwrap());
    let short = text
        .split_once('[')
        .and_then(|(_, rest)| rest.split_once(']'))
        .map(|(h, _)| h.trim().to_string())
        .expect("hash in log output");

    tit.run(&["rm", &short])
        .success()
        .stdout(predicate::str::contains("has been removed"));

    // Removed commit no longer counts toward total time.
    tit.run(&["log"])
        .success()
        .stdout(predicate::str::contains("to be removed").not());

    // ...but shows up under `log --all`.
    tit.run(&["log", "--all"])
        .success()
        .stdout(predicate::str::contains("Deleted Sessions:"));
}

#[test]
fn checkout_and_projects_listing() {
    let tit = Tit::new();
    tit.run(&["init", "alpha"]).success();
    tit.run(&["init", "beta"]).success();

    // `init beta` made beta current; switch back to alpha.
    tit.run(&["checkout", "alpha"])
        .success()
        .stdout(predicate::str::contains("Switched to project 'alpha'"));

    tit.run(&["projects"])
        .success()
        .stdout(predicate::str::contains("alpha").and(predicate::str::contains("beta")));

    tit.run(&["checkout", "missing"])
        .failure()
        .stderr(predicate::str::contains("does not exist"));
}

#[test]
fn reads_legacy_in_progress_sentinel() {
    // Older data wrote the literal string "In Progress" for an open session.
    let tit = Tit::new();
    let dir = tit.data.join("projects").join("legacy");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(tit.data.join("HEAD"), "legacy").unwrap();
    std::fs::write(
        dir.join("uncommitted_sessions.json"),
        r#"[{"start": "2024-01-01T10:00:00", "end": "In Progress"}]"#,
    )
    .unwrap();

    tit.run(&["status"])
        .success()
        .stdout(predicate::str::contains("Session in progress"));
}

#[test]
fn log_from_and_to_bound_the_range() {
    let tit = Tit::new();
    let dir = tit.data.join("projects").join("r");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(tit.data.join("HEAD"), "r").unwrap();

    let commit = |day: u32, message: &str, hash: &str| {
        format!(
            r#"{{"sessions": [{{"start": "2024-05-0{day}T10:00:00", "end": "2024-05-0{day}T11:00:00"}}],
                "message": "{message}", "hash": "{hash}"}}"#
        )
    };
    let committed = format!(
        "[{}, {}, {}]",
        commit(1, "first", "aaaaaaa1111111111111111111111111111111111"),
        commit(2, "second", "bbbbbbb2222222222222222222222222222222222"),
        commit(3, "third", "ccccccc3333333333333333333333333333333333"),
    );
    std::fs::write(dir.join("committed_sessions.json"), committed).unwrap();

    tit.run(&["log", "--from", "bbbbbbb"])
        .success()
        .stdout(
            predicate::str::contains("first")
                .not()
                .and(predicate::str::contains("second"))
                .and(predicate::str::contains("third")),
        );

    tit.run(&["log", "--to", "bbbbbbb"]).success().stdout(
        predicate::str::contains("first")
            .and(predicate::str::contains("second"))
            .and(predicate::str::contains("third").not()),
    );

    // Both bounds are inclusive, so a single commit can be isolated.
    tit.run(&["l", "-v", "--from", "bbbbbbb", "--to", "bbbbbbb"])
        .success()
        .stdout(
            predicate::str::contains("second")
                .and(predicate::str::contains("first").not())
                .and(predicate::str::contains("third").not()),
        );

    tit.run(&["log", "--from", "nope"])
        .failure()
        .stderr(predicate::str::contains("No commit found"));
}

/// Sanity: the binary parses a fixture mirroring the real on-disk shape.
#[test]
fn parses_realistic_committed_fixture() {
    let tit = Tit::new();
    let dir = tit.data.join("projects").join("fix");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(tit.data.join("HEAD"), "fix").unwrap();
    let fixture = r#"[
    {
        "sessions": [
            {
                "start": "2024-05-26T12:36:53",
                "end": "2024-05-26T13:05:57",
                "message": "Session stopped. Duration: 00:29:04"
            }
        ],
        "message": "Youtube Shorts",
        "hash": "0c7059fbf63d7376a475eec752d3522f243467dd"
    }
]"#;
    std::fs::write(dir.join("committed_sessions.json"), fixture).unwrap();

    // `time` uses the unpadded form (matching the original tool).
    tit.run(&["time"])
        .success()
        .stdout(predicate::str::contains("0:29:04"));

    // `export` keeps the zero-padded table format.
    tit.run(&["export"])
        .success()
        .stdout(predicate::str::contains("00:29:04"));
}
