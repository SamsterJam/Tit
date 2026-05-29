use std::fmt;

/// User-facing errors. `Display` produces the message shown (in red) on stderr,
/// mirroring the wording of the original Python tool.
#[derive(Debug)]
pub enum TitError {
    NoProject,
    SessionAlreadyRunning,
    NoSessionInProgress,
    NoSessionToCommit,
    EmptyMessage,
    ProjectExists(String),
    ProjectNotFound(String),
    CommitNotFound(String),
    AmbiguousHash(String),
    NoEditor,
    EditParse(String),
    Parse(String),
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl fmt::Display for TitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use TitError::*;
        match self {
            NoProject => write!(
                f,
                "Error: No project selected. Use 'tit init <project>' to create a project \
                 or 'tit checkout <project>' to switch to a project."
            ),
            SessionAlreadyRunning => write!(f, "Error: A session is already in progress."),
            NoSessionInProgress => write!(f, "Error: No session in progress."),
            NoSessionToCommit => write!(f, "Error: No session to commit."),
            EmptyMessage => write!(f, "Error: Commit message is required."),
            ProjectExists(p) => write!(f, "Error: Project '{p}' already exists."),
            ProjectNotFound(p) => write!(f, "Error: Project '{p}' does not exist."),
            CommitNotFound(h) => {
                write!(f, "Error: No commit found with hash starting with '{h}'.")
            }
            AmbiguousHash(h) => {
                write!(f, "Error: Ambiguous hash '{h}' matches multiple commits.")
            }
            NoEditor => write!(
                f,
                "Error: No suitable text editor found. Set $EDITOR or install one of: \
                 vim, nano, code, notepad."
            ),
            EditParse(msg) => write!(f, "Error: Failed to parse the edited file: {msg}"),
            Parse(s) => write!(f, "Error: Could not parse datetime '{s}'."),
            Io(e) => write!(f, "Error: {e}"),
            Json(e) => write!(f, "Error: invalid JSON data: {e}"),
        }
    }
}

impl std::error::Error for TitError {}

impl From<std::io::Error> for TitError {
    fn from(e: std::io::Error) -> Self {
        TitError::Io(e)
    }
}

impl From<serde_json::Error> for TitError {
    fn from(e: serde_json::Error) -> Self {
        TitError::Json(e)
    }
}
