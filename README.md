# Tit - Time Tracker CLI

**Tit** is a simple command-line tool to track and manage your project time sessions. Inspired by `git`, it helps you start, stop, commit, and log your time sessions efficiently.

![Tit Icon](Tit.png)

## Features

- **Start and Stop Sessions**: Easily start and stop time tracking sessions.
- **Commit Sessions**: Commit your sessions with meaningful messages.
- **Log Sessions**: View a log of all committed and uncommitted sessions.
- **Project Management**: Initialize, list, switch, and delete projects.
- **Export Sessions**: Export sessions to ASCII table or CSV format.
- **Edit Commits**: Edit specific commits.
- **Remove and Purge Commits**: Non-destructively remove or destructively purge commits.
- **Total Time Calculation**: Return total project time.

## Installation

1. **Clone the repository:**

   ```sh
   git clone https://github.com/samsterjam/tit.git
   ```

2. **Navigate to the project directory:**

   ```sh
   cd tit
   ```

3. **Install dependencies:**

   ```sh
   pip install -r requirements.txt
   ```

4. **Make the script executable:**

   ```sh
   chmod +x tit.py
   ```

5. **(Optional) Add to your PATH:**

   link it to path:

   ```sh
   sudo ln -s $(pwd)/tit.py /usr/local/bin/tit
   ```

   or just move it:

   ```sh
   sudo mv tit.py /usr/local/bin/tit
   ```

## Usage

### Initialize a New Project

```sh
tit init <project_name>
```

### Start a New Session

```sh
tit start
```

### End the Current Session

```sh
tit end
```

### Commit the Current Session

```sh
tit commit -m "Your commit message"
```

### Show Log of All Sessions

```sh
tit log
```

### Show Status of the Current Project

```sh
tit status
```

### List All Available Projects

```sh
tit projects
```

### Switch to a Different Project

```sh
tit checkout <project_name>
```

### Delete a Project

```sh
tit delete <project_name>
```

### Show Total Time from All Committed, Non-Deleted Commits

```sh
tit time
```

### Discard Uncommitted Sessions

```sh
tit reset
```

### Remove a Specific Commit

```sh
tit rm <commit_hash>
```

### Purge a Commit (Destructive)

```sh
tit purge <commit_hash>
```

### Edit a Specific Commit

```sh
tit edit <commit_hash>
```

### Export All Sessions to ASCII Table or CSV

```sh
tit export [ascii|csv] [-a|--all]
```

## Example Workflow

1. **Initialize a project:**

   ```sh
   tit init my_project
   ```

2. **Start a session:**

   ```sh
   tit start
   ```

3. **End the session:**

   ```sh
   tit end
   ```

4. **Commit the session:**

   ```sh
   tit commit -m "Completed initial setup"
   ```

5. **View the log:**
   ```sh
   tit log
   ```

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

---