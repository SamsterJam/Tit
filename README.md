# Tit - a git-inspired time tracker

![Tit Icon](Tit.png)

`tit` tracks time the way `git` tracks code: `start`/`end` sessions, `commit`
them with a message, `log` your history, and `checkout` between projects. It's a
single static binary with no runtime dependencies.

> This is the Rust rewrite of Tit. The original Python version is archived at
> [tit-python](https://github.com/SamsterJam/tit-python). The Rust version is a
> drop-in replacement: it reads and writes the same data in
> `~/.local/share/tit/`, so existing projects keep working with no migration.


## Build & install

```sh
cargo build --release
sudo mv target/release/tit /usr/local/bin/.
```

## Data & compatibility

State lives in `~/.local/share/tit/`, identical to the original tool:

```
HEAD                                    current project
projects/<name>/committed_sessions.json
projects/<name>/uncommitted_sessions.json
projects/<name>/deleted_sessions.json
```

Your existing projects work as-is with no migration. New commit hashes are
byte-compatible with the Python tool's hashing. Set `TIT_DATA_DIR` to use a
different location (handy for scripts and tests).

## Commands

| Command | Aliases | Description |
|---------|---------|-------------|
| `tit init <project>` | | Create and switch to a project |
| `tit checkout <project>` | | Switch projects |
| `tit projects` | | List projects (current is marked `*`) |
| `tit delete <project>` | | Delete a project (confirms) |
| `tit start` | `s` | Start a session |
| `tit end` | `e` | End the current session |
| `tit status` | | Show current session / uncommitted state |
| `tit reset` | | Discard uncommitted sessions (confirms) |
| `tit commit -m <msg>` | `c` | Commit uncommitted sessions |
| `tit log [-a] [-v]` | `l` | Show commit log (`-a` all, `-v` verbose) |
| `tit time` | | Total committed time |
| `tit today` | | Total time tracked today |
| `tit export [-a] [-v] [--from <h>] [--to <h>] [ascii\|csv]` | | Export a summary |
| `tit edit <hash>` | | Edit a commit's message/sessions in `$EDITOR` |
| `tit rm <hash>` | | Soft-remove a commit (recoverable; second `rm` purges) |
| `tit purge <hash>` | | Permanently delete a commit (confirms) |

Commit hashes accept any unique prefix, just like git.


## Example

```sh
tit init my_project
tit start
# ... work ...
tit end
tit commit -m "Completed initial setup"
tit log
```

## License

Copyright (C) 2026 SamsterJam.

Licensed under the GNU General Public License v3.0 or later. See [LICENSE](LICENSE)
for the full text.
