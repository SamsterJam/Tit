#!/usr/bin/env python3
import argparse
import json
import os
import hashlib
import tempfile
import subprocess
from datetime import datetime, timedelta
from colorama import init, Fore, Style
from termcolor import colored
from tabulate import tabulate
import csv
import re

# Initialize colorama
init(autoreset=True)

# Define the base directory for storing data
BASE_DIR = os.path.expanduser('~/.local/share/tit')
PROJECTS_DIR = os.path.join(BASE_DIR, 'projects')
HEAD_FILE = os.path.join(BASE_DIR, 'HEAD')

def load_data(file):
    if not os.path.exists(file):
        return []
    with open(file, 'r') as f:
        return json.load(f)

def save_data(data, file):
    os.makedirs(os.path.dirname(file), exist_ok=True)
    with open(file, 'w') as f:
        json.dump(data, f, indent=4)

def format_datetime(dt):
    return dt.strftime('%Y-%m-%dT%H:%M:%S')

def format_display_datetime(dt):
    return dt.strftime('%m/%d %I:%M %p')

def generate_commit_hash(sessions):
    sessions_str = json.dumps(sessions, sort_keys=True)
    return hashlib.sha1(sessions_str.encode()).hexdigest()

def get_current_project():
    if not os.path.exists(HEAD_FILE):
        return None
    with open(HEAD_FILE, 'r') as f:
        project = f.read().strip()
        return project if project else None

def set_current_project(project):
    with open(HEAD_FILE, 'w') as f:
        f.write(project)

def get_deleted_sessions_file(project):
    project_dir = os.path.join(PROJECTS_DIR, project)
    return os.path.join(project_dir, 'deleted_sessions.json')

def get_project_paths(project):
    project_dir = os.path.join(PROJECTS_DIR, project)
    committed_file = os.path.join(project_dir, 'committed_sessions.json')
    uncommitted_file = os.path.join(project_dir, 'uncommitted_sessions.json')
    return committed_file, uncommitted_file

def format_display_datetime(dt):
    return dt.strftime('%Y-%m-%d %I:%M %p')

def parse_display_datetime(dt_str):
    return datetime.strptime(dt_str, '%Y-%m-%d %I:%M %p')

def strip_ansi_codes(text):
    ansi_escape = re.compile(r'\x1B[@-_][0-?]*[ -/]*[@-~]')
    return ansi_escape.sub('', text)

def find_editor():
    editors = ['vim', 'nano', 'code', 'notepad']  # List of editors to check
    for editor in editors:
        if subprocess.call(['which', editor], stdout=subprocess.PIPE, stderr=subprocess.PIPE) == 0:
            return editor
    raise FileNotFoundError("No suitable text editor found. Please install one of the following: vim, nano, code, notepad.")

def resolve_commit_hash(partial_hash, committed_data, deleted_data):
    matches = []
    for commit in committed_data + deleted_data:
        if commit.get('hash').startswith(partial_hash):
            matches.append(commit.get('hash'))
    if len(matches) == 1:
        return matches[0]
    elif len(matches) > 1:
        raise ValueError(f"Error: Ambiguous hash '{partial_hash}' matches multiple commits.")
    else:
        raise ValueError(f"Error: No commit found with hash starting with '{partial_hash}'.")

def start_session():
    project = get_current_project()
    if not project:
        print(colored("Error: No project selected. Use 'tit init <project>' to create a project or 'tit checkout <project>' to switch to a project.", 'red'))
        return

    _, uncommitted_file = get_project_paths(project)
    data = load_data(uncommitted_file)
    if data and 'end' not in data[-1]:
        print(colored("Error: A session is already in progress.", 'red'))
        return
    data.append({'start': format_datetime(datetime.now())})
    save_data(data, uncommitted_file)
    print(colored("Session started.", 'green'))

def stop_session():
    project = get_current_project()
    if not project:
        print(colored("Error: No project selected. Use 'tit init <project>' to create a project or 'tit checkout <project>' to switch to a project.", 'red'))
        return

    _, uncommitted_file = get_project_paths(project)
    data = load_data(uncommitted_file)
    if not data or 'end' in data[-1]:
        print(colored("Error: No session in progress.", 'red'))
        return
    data[-1]['end'] = format_datetime(datetime.now())
    save_data(data, uncommitted_file)
    print(colored("Session stopped.", 'green'))

def commit_session(message):
    project = get_current_project()
    if not project:
        print(colored("Error: No project selected. Use 'tit init <project>' to create a project or 'tit checkout <project>' to switch to a project.", 'red'))
        return

    committed_file, uncommitted_file = get_project_paths(project)
    uncommitted_data = load_data(uncommitted_file)
    if not uncommitted_data or 'end' not in uncommitted_data[-1]:
        print(colored("Error: No session to commit.", 'red'))
        return

    commit_data = {
        'sessions': uncommitted_data,
        'message': message,
        'hash': generate_commit_hash(uncommitted_data)
    }

    committed_data = load_data(committed_file)
    committed_data.append(commit_data)

    save_data([], uncommitted_file)
    save_data(committed_data, committed_file)
    print(colored(f"Session committed with message: {message}", 'green', attrs=['bold']))

def show_total_time():
    project = get_current_project()
    if not project:
        print(colored("Error: No project selected. Use 'tit init <project>' to create a project or 'tit checkout <project>' to switch to a project.", 'red'))
        return

    committed_file, uncommitted_file = get_project_paths(project)
    deleted_file = get_deleted_sessions_file(project)
    committed_data = load_data(committed_file)
    uncommitted_data = load_data(uncommitted_file)
    deleted_data = load_data(deleted_file)

    total_committed_duration = timedelta()
    total_uncommitted_duration = timedelta()

    for commit in committed_data:
        if commit in deleted_data:
            continue
        sessions = commit.get('sessions', [])
        for session in sessions:
            start = datetime.fromisoformat(session.get('start'))
            end = datetime.fromisoformat(session.get('end'))
            total_committed_duration += (end - start)

    for session in uncommitted_data:
        start = datetime.fromisoformat(session.get('start'))
        end = session.get('end')
        if end is None or end == 'In Progress':
            end = datetime.now()
        else:
            end = datetime.fromisoformat(end)
        total_uncommitted_duration += (end - start)

    total_committed_duration_str = str(total_committed_duration).split('.')[0]  # Remove microseconds

    if total_uncommitted_duration > timedelta():
        print(colored(f"Committed: {total_committed_duration_str}", 'green'))
        total_uncommitted_duration_str = str(total_uncommitted_duration).split('.')[0]  # Remove microseconds
        print(colored(f"Uncommitted: {total_uncommitted_duration_str}", 'yellow'))

        total_duration = total_committed_duration + total_uncommitted_duration
        total_duration_str = str(total_duration).split('.')[0]  # Remove microseconds
        print(colored("---------------------", 'blue'))
        print(colored(f"Total: {total_duration_str}", 'blue', attrs=['bold']))
    else:
        print(colored(f"Total: {total_committed_duration_str}", 'green', attrs=['bold']))

def reset_sessions():
    project = get_current_project()
    if not project:
        print(colored("Error: No project selected. Use 'tit init <project>' to create a project or 'tit checkout <project>' to switch to a project.", 'red'))
        return

    _, uncommitted_file = get_project_paths(project)
    uncommitted_data = load_data(uncommitted_file)
    if not uncommitted_data:
        print(colored("No uncommitted sessions to reset.", 'yellow'))
        return

    # List uncommitted sessions
    print(colored("Uncommitted sessions:", 'yellow'))
    for session in uncommitted_data:
        start = session.get('start')
        end = session.get('end', 'In Progress')
        start_time = datetime.fromisoformat(start)
        if end == 'In Progress':
            duration = datetime.now() - start_time
        else:
            end_time = datetime.fromisoformat(end)
            duration = end_time - start_time
        duration_str = str(duration).split('.')[0]  # Remove microseconds
        print(colored(f"  - Start: {format_display_datetime(start_time)}, Duration: {duration_str}", 'red'))

    # Prompt for confirmation
    confirm = input(colored("Are you sure you want to discard these sessions? [y/N]: ", 'yellow')).strip().lower()
    if confirm not in ['y', 'yes']:
        print(colored("Reset aborted.", 'green'))
        return

    # Reset uncommitted sessions
    save_data([], uncommitted_file)
    print(colored("Uncommitted sessions have been discarded.", 'green'))

def log_sessions(show_all=False, verbose=False):
    project = get_current_project()
    if not project:
        print(colored("Error: No project selected. Use 'tit init <project>' to create a project or 'tit checkout <project>' to switch to a project.", 'red'))
        return

    committed_file, uncommitted_file = get_project_paths(project)
    deleted_file = get_deleted_sessions_file(project)
    committed_data = load_data(committed_file)
    uncommitted_data = load_data(uncommitted_file) if show_all else []
    deleted_data = load_data(deleted_file) if show_all else []

    for commit in committed_data:
        commit_hash = commit.get('hash')
        message = commit.get('message', '')
        if verbose:
            print(colored(f"commit {commit_hash}", 'yellow', attrs=['bold']))
            print(f"Date: {datetime.now().strftime('%a %b %d %H:%M:%S %Y %z')}\n")
            print(colored(f"    {message}", 'green'))
            sessions = commit.get('sessions', [])
            print(f"    Sessions ({len(sessions)}):")
            for session in sessions:
                start = session.get('start')
                end = session.get('end')
                start_time = datetime.fromisoformat(start)
                end_time = datetime.fromisoformat(end)
                duration = end_time - start_time
                duration_str = str(duration).split('.')[0]  # Remove microseconds
                print(f"      - Start: {format_display_datetime(start_time)}, Duration: {colored(duration_str, attrs=['bold'])}")
            print()
        else:
            print(f"[{colored(commit_hash[:7], 'yellow')}] {colored(message, 'green', attrs=['bold'])}")
            sessions = commit.get('sessions', [])
            for session in sessions:
                start = session.get('start')
                end = session.get('end')
                start_time = datetime.fromisoformat(start)
                end_time = datetime.fromisoformat(end)
                duration = end_time - start_time
                duration_str = str(duration).split('.')[0]  # Remove microseconds
                print(f"└─ {colored(duration_str, 'blue')} | Start: {format_display_datetime(start_time)}")
            print()

    if show_all:
        if uncommitted_data:
            print("Uncommitted Sessions:")
            for session in uncommitted_data:
                start = session.get('start')
                end = session.get('end', 'In Progress')
                start_time = datetime.fromisoformat(start)
                if end == 'In Progress':
                    duration = datetime.now() - start_time
                else:
                    end_time = datetime.fromisoformat(end)
                    duration = end_time - start_time
                duration_str = str(duration).split('.')[0]  # Remove microseconds
                print(colored(f"  - Start: {format_display_datetime(start_time)}, Duration: {duration_str}", 'red'))

        if deleted_data:
            print("Deleted Sessions:")
            for commit in deleted_data:
                commit_hash = commit.get('hash')
                message = commit.get('message', '')
                print(f"{Fore.RED}{Style.DIM}[Removed] commit {commit_hash}{Style.RESET_ALL}")
                print(colored(f"Date: {datetime.now().strftime('%a %b %d %H:%M:%S %Y %z')}\n", 'dark_grey'))
                print(colored(f"    {message}", 'dark_grey'))
                sessions = commit.get('sessions', [])
                print(colored(f"    Sessions ({len(sessions)}):", 'dark_grey'))
                for session in sessions:
                    start = session.get('start')
                    end = session.get('end')
                    start_time = datetime.fromisoformat(start)
                    end_time = datetime.fromisoformat(end)
                    duration = end_time - start_time
                    duration_str = str(duration).split('.')[0]  # Remove microseconds
                    print(colored(f"      - Start: {format_display_datetime(start_time)}, Duration: {duration_str}", 'dark_grey'))
                print()

def status():
    project = get_current_project()
    if not project:
        print(colored("Error: No project selected. Use 'tit init <project>' to create a project or 'tit checkout <project>' to switch to a project.", 'red'))
        return
    
    colored_project = colored(f"({project})", 'blue', attrs=['bold'])

    _, uncommitted_file = get_project_paths(project)
    uncommitted_data = load_data(uncommitted_file)
    if not uncommitted_data or 'end' in uncommitted_data[-1]:
        uncommitted_count = len(uncommitted_data)
        if uncommitted_count == 0:
             print(f"{colored_project} {colored('No session in progress. 0 uncommitted session(s).', 'green')}")
        else:
            print(f"{colored_project} No session in progress. {uncommitted_count} uncommitted session(s).")
            for session in uncommitted_data:
                start = session.get('start')
                end = session.get('end', 'In Progress')
                message = session.get('message', '')
                start_time = datetime.fromisoformat(start)
                if end == 'In Progress':
                    duration = datetime.now() - start_time
                else:
                    end_time = datetime.fromisoformat(end)
                    duration = end_time - start_time
                print(colored(f"  - Start: {format_display_datetime(start_time)}, Duration: {str(duration).split('.')[0]}", 'red'))
    else:
        start = uncommitted_data[-1]['start']
        start_time = datetime.fromisoformat(start)
        elapsed_time = datetime.now() - start_time
        elapsed_str = str(elapsed_time).split('.')[0]  # Remove microseconds
        print(colored(f"Session in progress. Started at: {format_display_datetime(start_time)} (Elapsed: {elapsed_str})", 'yellow', attrs=['bold']))
        if(len(uncommitted_data)-1 > 0):
            print(f"{len(uncommitted_data) - 1} uncommitted session(s).")
        
        for session in uncommitted_data[:-1]:
            start = session.get('start')
            end = session.get('end', 'In Progress')
            message = session.get('message', '')
            start_time = datetime.fromisoformat(start)
            if end == 'In Progress':
                duration = datetime.now() - start_time
            else:
                end_time = datetime.fromisoformat(end)
                duration = end_time - start_time
            print(colored(f"  - Start: {format_display_datetime(start_time)}, Duration: {str(duration).split('.')[0]}", 'red'))

def remove_commit(commit_hash):
    project = get_current_project()
    if not project:
        print(colored("Error: No project selected. Use 'tit init <project>' to create a project or 'tit checkout <project>' to switch to a project.", 'red'))
        return

    committed_file, _ = get_project_paths(project)
    deleted_file = get_deleted_sessions_file(project)
    committed_data = load_data(committed_file)
    deleted_data = load_data(deleted_file)

    try:
        full_hash = resolve_commit_hash(commit_hash, committed_data, deleted_data)
    except ValueError as e:
        print(colored(str(e), 'red'))
        return

    commit_to_remove = None
    for commit in committed_data:
        if commit.get('hash') == full_hash:
            commit_to_remove = commit
            break

    if not commit_to_remove:
        # Check if the commit is already in the deleted sessions
        for commit in deleted_data:
            if commit.get('hash') == full_hash:
                purge_commit(full_hash)
                return

        print(colored(f"Error: Commit with hash '{full_hash}' not found.", 'red'))
        return

    # Move the commit to deleted sessions
    committed_data.remove(commit_to_remove)
    deleted_data.append(commit_to_remove)
    save_data(committed_data, committed_file)
    save_data(deleted_data, deleted_file)
    print(colored(f"Commit '{full_hash}' has been removed.", 'green'))

def export_sessions(show_all=False, format='ascii', from_commit=None, to_commit=None):
    project = get_current_project()
    if not project:
        print(colored("Error: No project selected. Use 'tit init <project>' to create a project or 'tit checkout <project>' to switch to a project.", 'red'))
        return

    committed_file, uncommitted_file = get_project_paths(project)
    committed_data = load_data(committed_file)
    uncommitted_data = load_data(uncommitted_file)

    table_data = []
    total_duration = timedelta()

    # Helper function to get the commit time from the first session's start time
    def get_commit_time(commit):
        if commit['sessions']:
            return datetime.fromisoformat(commit['sessions'][0]['start'])
        return None

    # Get the times for from_commit and to_commit
    from_time = None
    to_time = None

    if from_commit:
        for commit in committed_data:
            if commit.get('hash') == from_commit:
                from_time = get_commit_time(commit)
                break

    if to_commit:
        for commit in committed_data:
            if commit.get('hash') == to_commit:
                to_time = get_commit_time(commit)
                break

    # Filter commits based on from_time and to_time
    if from_time or to_time:
        filtered_committed_data = []
        for commit in committed_data:
            commit_time = get_commit_time(commit)
            if commit_time:
                if from_time and to_time:
                    if from_time <= commit_time <= to_time:
                        filtered_committed_data.append(commit)
                elif from_time:
                    if commit_time >= from_time:
                        filtered_committed_data.append(commit)
                elif to_time:
                    if commit_time <= to_time:
                        filtered_committed_data.append(commit)
        committed_data = filtered_committed_data

    for commit in committed_data:
        sessions = commit.get('sessions', [])
        message = commit.get('message', 'No message')
        commit_duration = timedelta()
        for session in sessions:
            start = datetime.fromisoformat(session.get('start'))
            end = datetime.fromisoformat(session.get('end'))
            duration = end - start
            commit_duration += duration

        total_duration += commit_duration

        if len(sessions) == 1:
            start = datetime.fromisoformat(sessions[0].get('start'))
            table_data.append([colored(message,attrs=['bold']), colored(str(commit_duration).split('.')[0],attrs=['bold']), colored(format_display_datetime(start), attrs=['bold'])])
        else:
            commit_date = format_display_datetime(datetime.now())  # Assuming the commit date is now
            table_data.append([colored(message,attrs=['bold']), colored(str(commit_duration).split('.')[0],attrs=['bold']), colored(commit_date, attrs=['bold'])])
            for i, session in enumerate(sessions, start=1):
                start = datetime.fromisoformat(session.get('start'))
                end = datetime.fromisoformat(session.get('end'))
                duration = end - start
                table_data.append([f"└── (Session {i})", str(duration).split('.')[0], format_display_datetime(start)])

    if show_all:
        for session in uncommitted_data:
            start = datetime.fromisoformat(session.get('start'))
            end = session.get('end')
            if end is None or end == 'In Progress':
                end = datetime.now()
            else:
                end = datetime.fromisoformat(end)
            duration = end - start
            total_duration += duration
            table_data.append([colored("Uncommitted", 'yellow'), colored(str(duration).split('.')[0], 'yellow'), colored(format_display_datetime(start), 'yellow')])

    # Add total row
    total_duration_str = str(total_duration).split('.')[0]  # Remove microseconds
    export_time = format_display_datetime(datetime.now())
    table_data.append([colored("Total", 'green', attrs=['bold']), colored(total_duration_str, 'green', attrs=['bold']), colored(export_time, 'green', attrs=['bold'])])

    headers = ["Message", "Duration", "Date"]

    if format == 'ascii':
        table = tabulate(table_data, headers, tablefmt="grid")
        print(table)
    elif format == 'csv':
        csv_file = f"{project}_sessions.csv"
        with open(csv_file, 'w', newline='') as file:
            writer = csv.writer(file)
            writer.writerow(headers)
            for row in table_data:
                writer.writerow([strip_ansi_codes(text) for text in row])
        print(colored(f"Sessions exported to {csv_file}", 'green'))
    else:
        print(colored(f"Error: Unsupported export format '{format}'", 'red'))

def purge_commit(commit_hash):
    project = get_current_project()
    if not project:
        print(colored("Error: No project selected. Use 'tit init <project>' to create a project or 'tit checkout <project>' to switch to a project.", 'red'))
        return

    committed_file, uncommitted_file = get_project_paths(project)
    deleted_file = get_deleted_sessions_file(project)
    committed_data = load_data(committed_file)
    uncommitted_data = load_data(uncommitted_file)
    deleted_data = load_data(deleted_file)

    try:
        full_hash = resolve_commit_hash(commit_hash, committed_data, deleted_data)
    except ValueError as e:
        print(colored(str(e), 'red'))
        return

    commit_to_purge = None
    source = None

    # Check in committed sessions
    for commit in committed_data:
        if commit.get('hash') == full_hash:
            commit_to_purge = commit
            source = 'committed'
            break

    # Check in deleted sessions
    if not commit_to_purge:
        for commit in deleted_data:
            if commit.get('hash') == full_hash:
                commit_to_purge = commit
                source = 'deleted'
                break

    if not commit_to_purge:
        print(colored(f"Error: Commit with hash '{full_hash}' not found.", 'red'))
        return

    # Prompt for confirmation
    confirm = input(colored(f"Are you sure you want to permanently delete commit '{full_hash}'? This action cannot be undone. [y/N]: ", 'yellow')).strip().lower()
    if confirm not in ['y', 'yes']:
        print(colored("Purge aborted.", 'green'))
        return

    # Remove the commit from the appropriate source
    if source == 'committed':
        committed_data.remove(commit_to_purge)
        save_data(committed_data, committed_file)
    elif source == 'uncommitted':
        uncommitted_data.remove(commit_to_purge)
        save_data(uncommitted_data, uncommitted_file)
    elif source == 'deleted':
        deleted_data.remove(commit_to_purge)
        save_data(deleted_data, deleted_file)

    print(colored(f"Commit '{full_hash}' has been permanently deleted.", 'green'))

def edit_commit(commit_hash):
    project = get_current_project()
    if not project:
        print(colored("Error: No project selected. Use 'tit init <project>' to create a project or 'tit checkout <project>' to switch to a project.", 'red'))
        return

    committed_file, _ = get_project_paths(project)
    committed_data = load_data(committed_file)

    try:
        full_hash = resolve_commit_hash(commit_hash, committed_data, [])
    except ValueError as e:
        print(colored(str(e), 'red'))
        return

    commit_data = None
    for commit in committed_data:
        if commit.get('hash') == full_hash:
            commit_data = commit
            break

    if not commit_data:
        print(colored(f"Error: Commit with hash '{full_hash}' not found.", 'red'))
        return

    sessions = commit_data["sessions"]
    current_message = commit_data.get("message", "")

    # Create a temporary file
    with tempfile.NamedTemporaryFile(delete=False, mode='w+', suffix=".tmp") as temp_file:
        temp_file_path = temp_file.name

        # Write the commit message and sessions to the temporary file
        temp_file.write(f"Commit Message: {current_message}\n\n")
        for i, session in enumerate(sessions, start=1):
            start = datetime.fromisoformat(session["start"])
            end = datetime.fromisoformat(session["end"])
            temp_file.write(f"[Session {i}]\n")
            temp_file.write(f"Start-Time: {format_display_datetime(start)}\n")
            temp_file.write(f"End-Time: {format_display_datetime(end)}\n\n")

    # Find an available text editor
    try:
        editor = find_editor()
    except FileNotFoundError as e:
        print(colored(str(e), 'red'))
        return

    # Open the temporary file in the text editor
    subprocess.call([editor, temp_file_path])

    # Read the edited file
    with open(temp_file_path, 'r') as temp_file:
        edited_content = temp_file.read()

    # Parse the edited content
    edited_sessions = []
    lines = edited_content.splitlines()
    try:
        new_message = lines[0].split("Commit Message: ")[1].strip()
        for i in range(2, len(lines), 4):
            start_line = lines[i + 1].split("Start-Time: ")[1].strip()
            end_line = lines[i + 2].split("End-Time: ")[1].strip()
            edited_sessions.append({
                "start": parse_display_datetime(start_line).isoformat(),
                "end": parse_display_datetime(end_line).isoformat()
            })
    except (IndexError, ValueError) as e:
        print(colored("Error: Failed to parse the edited file. Please ensure the format is correct.", 'red'))
        os.remove(temp_file_path)
        return

    # Check if there are no sessions left
    if not edited_sessions:
        committed_data.remove(commit_data)
        print(colored(f"Commit '{full_hash}' has been deleted because no sessions were left.", 'yellow'))
    else:
        # Update the commit data
        commit_data["sessions"] = edited_sessions
        commit_data["message"] = new_message
        commit_data["hash"] = generate_commit_hash(edited_sessions)  # Re-hash the commit
        print(colored(f"Commit '{full_hash}' has been edited.", 'green'))

    # Save the updated data
    save_data(committed_data, committed_file)

    # Clean up the temporary file
    os.remove(temp_file_path)

def init_project(project):
    project_dir = os.path.join(PROJECTS_DIR, project)
    if os.path.exists(project_dir):
        print(colored(f"Error: Project '{project}' already exists.", 'red'))
        return
    os.makedirs(project_dir, exist_ok=True)
    set_current_project(project)
    print(colored(f"Initialized empty time tracking project '{project}'", 'green'))

def list_projects():
    if not os.path.exists(PROJECTS_DIR):
        print(colored("No projects found.", 'red'))
        return
    projects = os.listdir(PROJECTS_DIR)
    if not projects:
        print(colored("No projects found.", 'red'))
        return
    print(colored("Available projects"))
    print(colored("------------------"))
    for project in projects:
        print(colored(f"  {project}", 'blue', attrs=['bold']))

def delete_project(project):
    project_dir = os.path.join(PROJECTS_DIR, project)
    if not os.path.exists(project_dir):
        print(colored(f"Error: Project '{project}' does not exist.", 'red'))
        return

    # Prompt for confirmation
    confirm = input(colored(f"Are you sure you want to permanently delete the project '{project}'? This action cannot be undone. [y/N]: ", 'yellow')).strip().lower()
    if confirm not in ['y', 'yes']:
        print(colored("Project deletion aborted.", 'green'))
        return

    # Delete the project directory and its contents
    for root, dirs, files in os.walk(project_dir, topdown=False):
        for name in files:
            os.remove(os.path.join(root, name))
        for name in dirs:
            os.rmdir(os.path.join(root, name))
    os.rmdir(project_dir)
    print(colored(f"Deleted project '{project}'", 'red'))

    # If the deleted project was the current project, clear the HEAD_FILE
    if get_current_project() == project:
        set_current_project('')

    # Check if there are any remaining projects
    remaining_projects = os.listdir(PROJECTS_DIR)
    if not remaining_projects:
        print(colored("No projects remaining.", 'yellow'))

def checkout_project(project):
    project_dir = os.path.join(PROJECTS_DIR, project)
    if not os.path.exists(project_dir):
        print(colored(f"Error: Project '{project}' does not exist.", 'red'))
        return
    set_current_project(project)
    print(colored(f"Switched to project '{project}'", 'green'))

def main():
    parser = argparse.ArgumentParser(
        description="Time Tracker CLI - A simple tool to track and manage your project time sessions.",
        formatter_class=argparse.ArgumentDefaultsHelpFormatter
    )
    subparsers = parser.add_subparsers(dest='command', title='Commands', description='Available commands', help='Use "tit <command> --help" for more information about a command.')

    subparsers.add_parser('start', help='Start a new session', aliases=['s'])
    subparsers.add_parser('end', help='End the current session', aliases=['e'])

    commit_parser = subparsers.add_parser('commit', help='Commit the current session with a message', aliases=['c'])
    commit_parser.add_argument('-m', '--message', help='Commit message')
    commit_parser.add_argument('message_positional', nargs='*', help='Commit message (positional)')

    log_parser = subparsers.add_parser('log', help='Show log of all sessions', aliases=['l'])
    log_parser.add_argument('-a', '--all', action='store_true', help='Show all sessions including uncommitted ones')
    log_parser.add_argument('-v', '--verbose', action='store_true', help='Show verbose log')

    time_parser = subparsers.add_parser('time', help='Show total time from all committed, non-deleted commits')

    subparsers.add_parser('status', help='Show the current status of the project')

    reset_parser = subparsers.add_parser('reset', help='Discard uncommitted sessions')

    init_parser = subparsers.add_parser('init', help='Initialize a new project')
    init_parser.add_argument('project', help='Name of the project to initialize')

    subparsers.add_parser('projects', help='List all available projects')

    checkout_parser = subparsers.add_parser('checkout', help='Switch to a different project')
    checkout_parser.add_argument('project', help='Name of the project to switch to')

    delete_parser = subparsers.add_parser('delete', help='Delete a project')
    delete_parser.add_argument('project', help='Name of the project to delete')

    export_parser = subparsers.add_parser('export', help='Export all sessions to a nice ASCII table or CSV')
    export_parser.add_argument('-a', '--all', action='store_true', help='Export all sessions including uncommitted ones')
    export_parser.add_argument('--from', dest='from_commit', help='Export sessions from this commit hash')
    export_parser.add_argument('--to', dest='to_commit', help='Export sessions up to this commit hash')
    export_parser.add_argument('format', nargs='?', default='ascii', choices=['ascii', 'csv'], help='Export format (default: ascii)')

    rm_parser = subparsers.add_parser('rm', help='Remove a specific commit')
    rm_parser.add_argument('commit_hash', help='Hash of the commit to remove')

    purge_parser = subparsers.add_parser('purge', help='Purge a commit (destructive)')
    purge_parser.add_argument('commit_hash', help='Hash of the commit to purge')

    edit_parser = subparsers.add_parser('edit', help='Edit a specific commit')
    edit_parser.add_argument('commit_hash', help='Hash of the commit to edit')

    args = parser.parse_args()

    if args.command in ['start', 's']:
        start_session()
    elif args.command in ['end', 'e']:
        stop_session()
    elif args.command in ['commit', 'c']:
        commit_message = args.message if args.message else ' '.join(args.message_positional)
        if not commit_message:
            print(colored("Error: Commit message is required.", 'red'))
            return
        commit_session(commit_message)
    elif args.command in ['log', 'l']:
        log_sessions(args.all, args.verbose)
    elif args.command == 'status':
        status()
    elif args.command == 'time':
        show_total_time()
    elif args.command == 'init':
        init_project(args.project)
    elif args.command == 'projects':
        list_projects()
    elif args.command == 'checkout':
        checkout_project(args.project)
    elif args.command == 'delete':
        delete_project(args.project)
    elif args.command == 'reset':
       reset_sessions()
    elif args.command == 'rm':
        remove_commit(args.commit_hash)
    elif args.command == 'purge':
        purge_commit(args.commit_hash)
    elif args.command == 'edit':
        edit_commit(args.commit_hash)
    elif args.command == 'export':
        export_sessions(args.all, args.format, args.from_commit, args.to_commit)
    else:
        parser.print_help()

if __name__ == "__main__":
    main()