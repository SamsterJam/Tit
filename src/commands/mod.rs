pub mod commit;
pub mod project;
pub mod report;
pub mod session;

use crate::cli::Command;
use crate::error::TitError;
use crate::store::Store;

pub fn dispatch(store: &Store, command: Command) -> Result<(), TitError> {
    match command {
        Command::Start => session::start(store),
        Command::End => session::end(store),
        Command::Status => session::status(store),
        Command::Reset => session::reset(store),

        Command::Commit { message, words } => {
            let msg = message.unwrap_or_else(|| words.join(" "));
            if msg.trim().is_empty() {
                return Err(TitError::EmptyMessage);
            }
            commit::commit(store, &msg)
        }
        Command::Log {
            all,
            verbose,
            from_commit,
            to_commit,
        } => commit::log(
            store,
            all,
            verbose,
            from_commit.as_deref(),
            to_commit.as_deref(),
        ),
        Command::Rm { commit_hash } => commit::rm(store, &commit_hash),
        Command::Purge { commit_hash } => commit::purge(store, &commit_hash),
        Command::Edit { commit_hash } => commit::edit(store, &commit_hash),

        Command::Init { project } => project::init(store, &project),
        Command::Projects => project::list(store),
        Command::Checkout { project } => project::checkout(store, &project),
        Command::Delete { project } => project::delete(store, &project),

        Command::Time => report::time(store),
        Command::Today => report::today(store),
        Command::Export {
            all,
            verbose,
            from_commit,
            to_commit,
            format,
        } => report::export(
            store,
            all,
            format,
            from_commit.as_deref(),
            to_commit.as_deref(),
            verbose,
        ),
    }
}
