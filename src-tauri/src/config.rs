use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("{0}")]
    Message(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

impl Serialize for ConfigError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub user_name: String,
    pub user_email: String,
    /// Absolute or ~ path to private key. Empty = don't set core.sshCommand.
    pub ssh_key_path: String,
    /// Optional Host alias from ~/.ssh/config (informational / future remote rewrite).
    #[serde(default)]
    pub ssh_host_alias: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Repo {
    pub id: String,
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub profile_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    #[serde(default)]
    pub repos: Vec<Repo>,
    #[serde(default)]
    pub profiles: Vec<Profile>,
    #[serde(default)]
    pub selected_repo_id: Option<String>,
}

fn config_dir() -> Result<PathBuf, ConfigError> {
    let base = dirs::home_dir()
        .ok_or_else(|| ConfigError::Message("cannot resolve home directory".into()))?;
    let preferred = base.join(".gitswitch");
    let legacy = base.join(".gitbench");
    // Keep using legacy dir if already present and new dir doesn't exist yet.
    if !preferred.exists() && legacy.exists() {
        return Ok(legacy);
    }
    Ok(preferred)
}

fn config_path() -> Result<PathBuf, ConfigError> {
    Ok(config_dir()?.join("config.json"))
}

pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    if path == "~" {
        if let Some(home) = dirs::home_dir() {
            return home;
        }
    }
    PathBuf::from(path)
}

pub fn load_config() -> Result<AppConfig, ConfigError> {
    let path = config_path()?;
    if !path.exists() {
        let mut cfg = AppConfig::default();
        cfg.profiles = default_profiles();
        save_config(&cfg)?;
        return Ok(cfg);
    }
    let raw = fs::read_to_string(&path)?;
    let cfg: AppConfig = serde_json::from_str(&raw)?;
    Ok(cfg)
}

pub fn save_config(cfg: &AppConfig) -> Result<(), ConfigError> {
    let dir = config_dir()?;
    fs::create_dir_all(&dir)?;
    let path = config_path()?;
    let raw = serde_json::to_string_pretty(cfg)?;
    fs::write(path, raw)?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SshKeyInfo {
    pub name: String,
    pub path: String,
    pub pub_path: Option<String>,
    pub comment: Option<String>,
}

fn ssh_dir() -> Result<PathBuf, ConfigError> {
    let home = dirs::home_dir()
        .ok_or_else(|| ConfigError::Message("cannot resolve home directory".into()))?;
    Ok(home.join(".ssh"))
}

/// Discover private keys under ~/.ssh that have a matching `.pub` file.
pub fn list_ssh_keys() -> Result<Vec<SshKeyInfo>, ConfigError> {
    let dir = ssh_dir()?;
    if !dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut keys = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = match path.file_name().and_then(|s| s.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        // Prefer keys that ship with a .pub companion (standard ssh-keygen output).
        if name.ends_with(".pub")
            || name == "config"
            || name.starts_with("config.")
            || name == "known_hosts"
            || name.starts_with("known_hosts.")
            || name == "authorized_keys"
            || name == "authorized_keys2"
        {
            continue;
        }
        let pub_path = PathBuf::from(format!("{}.pub", path.display()));
        if !pub_path.is_file() {
            continue;
        }

        let comment = fs::read_to_string(&pub_path)
            .ok()
            .and_then(|s| {
                s.split_whitespace()
                    .nth(2)
                    .map(|c| c.trim().to_string())
                    .filter(|c| !c.is_empty())
            });

        keys.push(SshKeyInfo {
            name: name.clone(),
            path: format!("~/.ssh/{name}"),
            pub_path: Some(format!("~/.ssh/{name}.pub")),
            comment,
        });
    }

    keys.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(keys)
}

fn default_profiles() -> Vec<Profile> {
    match list_ssh_keys() {
        Ok(keys) if !keys.is_empty() => keys
            .into_iter()
            .map(|k| Profile {
                id: Uuid::new_v4().to_string(),
                name: k.comment.clone().unwrap_or_else(|| k.name.clone()),
                user_name: String::new(),
                user_email: String::new(),
                ssh_key_path: k.path,
                ssh_host_alias: String::new(),
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Create profiles for SSH keys that are not yet referenced by any profile.
pub fn import_missing_ssh_profiles() -> Result<AppConfig, ConfigError> {
    let mut cfg = load_config()?;
    let keys = list_ssh_keys()?;
    let existing: std::collections::HashSet<String> = cfg
        .profiles
        .iter()
        .map(|p| expand_tilde(&p.ssh_key_path).to_string_lossy().to_string())
        .collect();

    for key in keys {
        let abs = expand_tilde(&key.path).to_string_lossy().to_string();
        if existing.contains(&abs) {
            continue;
        }
        // Also skip if profile already points at the same ~/ path string.
        if cfg.profiles.iter().any(|p| {
            p.ssh_key_path == key.path
                || expand_tilde(&p.ssh_key_path).to_string_lossy() == abs.as_str()
        }) {
            continue;
        }
        cfg.profiles.push(Profile {
            id: Uuid::new_v4().to_string(),
            name: key.comment.clone().unwrap_or_else(|| key.name.clone()),
            user_name: String::new(),
            user_email: String::new(),
            ssh_key_path: key.path,
            ssh_host_alias: String::new(),
        });
    }

    save_config(&cfg)?;
    Ok(cfg)
}

pub fn add_repo(path: &str, name: Option<String>) -> Result<AppConfig, ConfigError> {
    let expanded = expand_tilde(path);
    if !expanded.is_dir() {
        return Err(ConfigError::Message(format!(
            "path is not a directory: {}",
            expanded.display()
        )));
    }
    if !expanded.join(".git").exists() {
        return Err(ConfigError::Message(format!(
            "not a git repository: {}",
            expanded.display()
        )));
    }

    let mut cfg = load_config()?;
    let abs = expanded
        .canonicalize()
        .unwrap_or(expanded)
        .to_string_lossy()
        .to_string();

    if cfg.repos.iter().any(|r| r.path == abs) {
        return Err(ConfigError::Message("repository already added".into()));
    }

    let display_name = name.unwrap_or_else(|| {
        Path::new(&abs)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| abs.clone())
    });

    let id = Uuid::new_v4().to_string();
    cfg.repos.push(Repo {
        id: id.clone(),
        name: display_name,
        path: abs,
        profile_id: cfg.profiles.first().map(|p| p.id.clone()),
    });
    cfg.selected_repo_id = Some(id);
    save_config(&cfg)?;
    Ok(cfg)
}

pub fn remove_repo(repo_id: &str) -> Result<AppConfig, ConfigError> {
    let mut cfg = load_config()?;
    cfg.repos.retain(|r| r.id != repo_id);
    if cfg.selected_repo_id.as_deref() == Some(repo_id) {
        cfg.selected_repo_id = cfg.repos.first().map(|r| r.id.clone());
    }
    save_config(&cfg)?;
    Ok(cfg)
}

pub fn select_repo(repo_id: &str) -> Result<AppConfig, ConfigError> {
    let mut cfg = load_config()?;
    if !cfg.repos.iter().any(|r| r.id == repo_id) {
        return Err(ConfigError::Message("repo not found".into()));
    }
    cfg.selected_repo_id = Some(repo_id.to_string());
    save_config(&cfg)?;
    Ok(cfg)
}

pub fn upsert_profile(profile: Profile) -> Result<AppConfig, ConfigError> {
    let mut cfg = load_config()?;
    if let Some(existing) = cfg.profiles.iter_mut().find(|p| p.id == profile.id) {
        *existing = profile;
    } else {
        cfg.profiles.push(profile);
    }
    save_config(&cfg)?;
    Ok(cfg)
}

pub fn delete_profile(profile_id: &str) -> Result<AppConfig, ConfigError> {
    let mut cfg = load_config()?;
    cfg.profiles.retain(|p| p.id != profile_id);
    for repo in &mut cfg.repos {
        if repo.profile_id.as_deref() == Some(profile_id) {
            repo.profile_id = None;
        }
    }
    save_config(&cfg)?;
    Ok(cfg)
}

pub fn bind_repo_profile(repo_id: &str, profile_id: Option<String>) -> Result<AppConfig, ConfigError> {
    let mut cfg = load_config()?;
    if let Some(ref pid) = profile_id {
        if !cfg.profiles.iter().any(|p| &p.id == pid) {
            return Err(ConfigError::Message("profile not found".into()));
        }
    }
    let repo = cfg
        .repos
        .iter_mut()
        .find(|r| r.id == repo_id)
        .ok_or_else(|| ConfigError::Message("repo not found".into()))?;
    repo.profile_id = profile_id;
    save_config(&cfg)?;
    Ok(cfg)
}

pub fn find_repo<'a>(cfg: &'a AppConfig, repo_id: &str) -> Result<&'a Repo, ConfigError> {
    cfg.repos
        .iter()
        .find(|r| r.id == repo_id)
        .ok_or_else(|| ConfigError::Message("repo not found".into()))
}

pub fn find_profile<'a>(cfg: &'a AppConfig, profile_id: &str) -> Result<&'a Profile, ConfigError> {
    cfg.profiles
        .iter()
        .find(|p| p.id == profile_id)
        .ok_or_else(|| ConfigError::Message("profile not found".into()))
}
