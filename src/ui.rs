//! Small terminal helpers: confirmation prompts and editor discovery.

use std::env;
use std::io::{self, Write};
use std::process::Command;

use owo_colors::OwoColorize;

use crate::error::TitError;

/// Print a yellow `[y/N]` prompt and return whether the user confirmed.
pub fn confirm(prompt: &str) -> bool {
    print!("{}", prompt.yellow());
    let _ = io::stdout().flush();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return false;
    }
    matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
}

/// Find a usable editor: honour `$VISUAL`/`$EDITOR` first, then fall back to a
/// list of common editors found on `PATH`.
pub fn find_editor() -> Result<String, TitError> {
    if let Some(ed) = env::var_os("VISUAL").or_else(|| env::var_os("EDITOR"))
        && let Ok(ed) = ed.into_string()
        && !ed.trim().is_empty()
    {
        return Ok(ed);
    }

    for candidate in ["vim", "nano", "code", "notepad"] {
        if is_on_path(candidate) {
            return Ok(candidate.to_string());
        }
    }
    Err(TitError::NoEditor)
}

fn is_on_path(program: &str) -> bool {
    Command::new("which")
        .arg(program)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
