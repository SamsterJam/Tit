//! `init`, `projects`, `checkout`, `delete`.

use owo_colors::OwoColorize;

use crate::error::TitError;
use crate::store::Store;
use crate::ui;

pub fn init(store: &Store, name: &str) -> Result<(), TitError> {
    store.init_project(name)?;
    println!(
        "{}",
        format!("Initialized empty time tracking project '{name}'").green()
    );
    Ok(())
}

pub fn list(store: &Store) -> Result<(), TitError> {
    let projects = store.list_projects();
    if projects.is_empty() {
        println!("{}", "No projects found.".red());
        return Ok(());
    }
    println!("Available projects");
    println!("------------------");
    let current = store.current_project();
    for project in projects {
        let marker = if current.as_deref() == Some(&project) {
            "* "
        } else {
            "  "
        };
        println!("{marker}{}", project.blue().bold());
    }
    Ok(())
}

pub fn checkout(store: &Store, name: &str) -> Result<(), TitError> {
    store.checkout_project(name)?;
    println!("{}", format!("Switched to project '{name}'").green());
    Ok(())
}

pub fn delete(store: &Store, name: &str) -> Result<(), TitError> {
    if !store.project_exists(name) {
        return Err(TitError::ProjectNotFound(name.to_string()));
    }

    let prompt = format!(
        "Are you sure you want to permanently delete the project '{name}'? \
         This action cannot be undone. [y/N]: "
    );
    if !ui::confirm(&prompt) {
        println!("{}", "Project deletion aborted.".green());
        return Ok(());
    }

    store.delete_project(name)?;
    println!("{}", format!("Deleted project '{name}'").red());

    if store.list_projects().is_empty() {
        println!("{}", "No projects remaining.".yellow());
    }
    Ok(())
}
