use clap::{Parser, Subcommand, ValueEnum};

/// Time Tracker CLI - a git-inspired tool to track and manage project time sessions.
#[derive(Parser)]
#[command(name = "tit", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Start a new session
    #[command(visible_alias = "s")]
    Start,

    /// End the current session
    #[command(visible_alias = "e")]
    End,

    /// Commit the current session(s) with a message
    #[command(visible_alias = "c")]
    Commit {
        /// Commit message
        #[arg(short, long)]
        message: Option<String>,
        /// Commit message (positional)
        #[arg(trailing_var_arg = true)]
        words: Vec<String>,
    },

    /// Show a log of committed sessions
    #[command(visible_alias = "l")]
    Log {
        /// Show all sessions including uncommitted and deleted ones
        #[arg(short, long)]
        all: bool,
        /// Show verbose, git-style output
        #[arg(short, long)]
        verbose: bool,
        /// Show commits from this commit hash onward
        #[arg(long = "from")]
        from_commit: Option<String>,
        /// Show commits up to this commit hash
        #[arg(long = "to")]
        to_commit: Option<String>,
    },

    /// Show total time from all committed, non-deleted commits
    Time,

    /// Show the current status of the project
    Status,

    /// Discard uncommitted sessions
    Reset,

    /// Initialize a new project
    Init {
        /// Name of the project to initialize
        project: String,
    },

    /// List all available projects
    Projects,

    /// Switch to a different project
    Checkout {
        /// Name of the project to switch to
        project: String,
    },

    /// Delete a project
    Delete {
        /// Name of the project to delete
        project: String,
    },

    /// Export sessions to an ASCII table or CSV
    Export {
        /// Export all sessions including uncommitted ones
        #[arg(short, long)]
        all: bool,
        /// Show individual sessions within commits
        #[arg(short, long)]
        verbose: bool,
        /// Export sessions from this commit hash
        #[arg(long = "from")]
        from_commit: Option<String>,
        /// Export sessions up to this commit hash
        #[arg(long = "to")]
        to_commit: Option<String>,
        /// Export format
        #[arg(value_enum, default_value_t = ExportFormat::Ascii)]
        format: ExportFormat,
    },

    /// Remove a specific commit (non-destructive)
    Rm {
        /// Hash of the commit to remove
        commit_hash: String,
    },

    /// Purge a commit (destructive)
    Purge {
        /// Hash of the commit to purge
        commit_hash: String,
    },

    /// Edit a specific commit
    Edit {
        /// Hash of the commit to edit
        commit_hash: String,
    },

    /// Show total time for today
    Today,
}

#[derive(Copy, Clone, ValueEnum)]
pub enum ExportFormat {
    Ascii,
    Csv,
}
