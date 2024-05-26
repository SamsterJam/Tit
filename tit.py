#!/usr/bin/env python3
import argparse
import json
import os
import hashlib
from datetime import datetime
from colorama import init, Fore, Style
from termcolor import colored

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

def log_sessions(show_all=False):
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

    commit_to_remove = None
    for commit in committed_data:
        if commit.get('hash') == commit_hash:
            commit_to_remove = commit
            break

    if not commit_to_remove:
        # Check if the commit is already in the deleted sessions
        for commit in deleted_data:
            if commit.get('hash') == commit_hash:
                purge_commit(commit_hash)
                return

        print(colored(f"Error: Commit with hash '{commit_hash}' not found.", 'red'))
        return

    # Move the commit to deleted sessions
    committed_data.remove(commit_to_remove)
    deleted_data.append(commit_to_remove)
    save_data(committed_data, committed_file)
    save_data(deleted_data, deleted_file)
    print(colored(f"Commit '{commit_hash}' has been removed.", 'green'))
    
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

    commit_to_purge = None
    source = None

    # Check in committed sessions
    for commit in committed_data:
        if commit.get('hash') == commit_hash:
            commit_to_purge = commit
            source = 'committed'
            break

    # Check in deleted sessions
    if not commit_to_purge:
        for commit in deleted_data:
            if commit.get('hash') == commit_hash:
                commit_to_purge = commit
                source = 'deleted'
                break

    if not commit_to_purge:
        print(colored(f"Error: Commit with hash '{commit_hash}' not found.", 'red'))
        return

    # Prompt for confirmation
    confirm = input(colored(f"Are you sure you want to permanently delete commit '{commit_hash}'? This action cannot be undone. [y/N]: ", 'yellow')).strip().lower()
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

    print(colored(f"Commit '{commit_hash}' has been permanently deleted.", 'green'))

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

    subparsers.add_parser('status', help='Show the current status of the project')

    reset_parser = subparsers.add_parser('reset', help='Discard uncommitted sessions')

    init_parser = subparsers.add_parser('init', help='Initialize a new project')
    init_parser.add_argument('project', help='Name of the project to initialize')

    subparsers.add_parser('projects', help='List all available projects')

    checkout_parser = subparsers.add_parser('checkout', help='Switch to a different project')
    checkout_parser.add_argument('project', help='Name of the project to switch to')

    delete_parser = subparsers.add_parser('delete', help='Delete a project')
    delete_parser.add_argument('project', help='Name of the project to delete')

    rm_parser = subparsers.add_parser('rm', help='Remove a specific commit')
    rm_parser.add_argument('commit_hash', help='Hash of the commit to remove')

    purge_parser = subparsers.add_parser('purge', help='purge a commit (destructive)')
    purge_parser.add_argument('commit_hash', help='Hash of the commit to purge')

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
        log_sessions(args.all)
    elif args.command == 'status':
        status()
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
    else:
        parser.print_help()

if __name__ == "__main__":
    main()