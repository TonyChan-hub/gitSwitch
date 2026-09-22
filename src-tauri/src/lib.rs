mod config;
mod git;

use config::{AppConfig, ConfigError, Profile, SshKeyInfo};
use git::{BranchInfo, RepoStatus};
use std::path::PathBuf;

fn repo_path(repo_id: &str) -> Result<PathBuf, ConfigError> {
    let cfg = config::load_config()?;
    let repo = config::find_repo(&cfg, repo_id)?;
    Ok(PathBuf::from(&repo.path))
}

fn profile_for_repo(repo_id: &str) -> Result<Option<Profile>, ConfigError> {
    let cfg = config::load_config()?;
    let repo = config::find_repo(&cfg, repo_id)?;
    Ok(match &repo.profile_id {
        Some(pid) => Some(config::find_profile(&cfg, pid)?.clone()),
        None => None,
    })
}

#[tauri::command]
fn get_config() -> Result<AppConfig, ConfigError> {
    config::load_config()
}

#[tauri::command]
fn add_repo(path: String, name: Option<String>) -> Result<AppConfig, ConfigError> {
    config::add_repo(&path, name)
}

#[tauri::command]
fn remove_repo(repo_id: String) -> Result<AppConfig, ConfigError> {
    config::remove_repo(&repo_id)
}

#[tauri::command]
fn select_repo(repo_id: String) -> Result<AppConfig, ConfigError> {
    config::select_repo(&repo_id)
}

#[tauri::command]
fn upsert_profile(profile: Profile) -> Result<AppConfig, ConfigError> {
    let mut p = profile;
    if p.id.trim().is_empty() {
        p.id = uuid::Uuid::new_v4().to_string();
    }
    config::upsert_profile(p)
}

#[tauri::command]
fn delete_profile(profile_id: String) -> Result<AppConfig, ConfigError> {
    config::delete_profile(&profile_id)
}

#[tauri::command]
fn bind_repo_profile(
    repo_id: String,
    profile_id: Option<String>,
) -> Result<AppConfig, ConfigError> {
    config::bind_repo_profile(&repo_id, profile_id)
}

#[tauri::command]
fn list_branches(repo_id: String) -> Result<Vec<BranchInfo>, ConfigError> {
    let path = repo_path(&repo_id)?;
    git::list_branches(&path)
}

#[tauri::command]
fn checkout_branch(repo_id: String, branch: String) -> Result<(), ConfigError> {
    let path = repo_path(&repo_id)?;
    git::checkout_branch(&path, &branch)
}

#[tauri::command]
fn create_branch(repo_id: String, name: String, checkout: bool) -> Result<(), ConfigError> {
    let path = repo_path(&repo_id)?;
    git::create_branch(&path, &name, checkout)
}

#[tauri::command]
fn delete_branch(repo_id: String, name: String, force: bool) -> Result<(), ConfigError> {
    let path = repo_path(&repo_id)?;
    git::delete_branch(&path, &name, force)
}

#[tauri::command]
fn get_repo_status(repo_id: String) -> Result<RepoStatus, ConfigError> {
    let path = repo_path(&repo_id)?;
    git::repo_status(&path)
}

#[tauri::command]
fn apply_profile(repo_id: String, profile_id: String) -> Result<RepoStatus, ConfigError> {
    let cfg = config::load_config()?;
    let path = PathBuf::from(&config::find_repo(&cfg, &repo_id)?.path);
    let profile = config::find_profile(&cfg, &profile_id)?.clone();
    git::apply_profile(&path, &profile)?;
    config::bind_repo_profile(&repo_id, Some(profile_id))?;
    git::repo_status(&path)
}

#[tauri::command]
fn fetch_repo(repo_id: String) -> Result<String, ConfigError> {
    let path = repo_path(&repo_id)?;
    let profile = profile_for_repo(&repo_id)?;
    git::fetch(&path, profile.as_ref())
}

#[tauri::command]
fn push_repo(repo_id: String, set_upstream: bool) -> Result<String, ConfigError> {
    let path = repo_path(&repo_id)?;
    let profile = profile_for_repo(&repo_id)?;
    git::push(&path, profile.as_ref(), set_upstream)
}

#[tauri::command]
fn recent_log(repo_id: String, limit: Option<usize>) -> Result<Vec<String>, ConfigError> {
    let path = repo_path(&repo_id)?;
    git::recent_log(&path, limit.unwrap_or(12))
}

#[tauri::command]
fn list_ssh_keys() -> Result<Vec<SshKeyInfo>, ConfigError> {
    config::list_ssh_keys()
}

#[tauri::command]
fn import_missing_ssh_profiles() -> Result<AppConfig, ConfigError> {
    config::import_missing_ssh_profiles()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_config,
            add_repo,
            remove_repo,
            select_repo,
            upsert_profile,
            delete_profile,
            bind_repo_profile,
            list_branches,
            checkout_branch,
            create_branch,
            delete_branch,
            get_repo_status,
            apply_profile,
            fetch_repo,
            push_repo,
            recent_log,
            list_ssh_keys,
            import_missing_ssh_profiles,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
