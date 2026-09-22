use crate::config::{expand_tilde, ConfigError, Profile};
use serde::Serialize;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchInfo {
    pub name: String,
    pub is_current: bool,
    pub is_remote: bool,
    pub upstream: Option<String>,
    /// Remote name this branch belongs to / tracks (e.g. "origin").
    pub remote: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoStatus {
    pub path: String,
    pub current_branch: Option<String>,
    pub is_dirty: bool,
    pub ahead: u32,
    pub behind: u32,
    pub remotes: Vec<RemoteInfo>,
    pub user_name: Option<String>,
    pub user_email: Option<String>,
    pub ssh_command: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteInfo {
    pub name: String,
    pub url: String,
}

fn run_git(repo: &Path, args: &[&str]) -> Result<String, ConfigError> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .map_err(|e| ConfigError::Message(format!("failed to run git: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let msg = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            format!("git {:?} failed", args)
        };
        return Err(ConfigError::Message(msg));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn run_git_with_env(
    repo: &Path,
    args: &[&str],
    ssh_command: Option<&str>,
) -> Result<String, ConfigError> {
    let mut cmd = Command::new("git");
    cmd.args(args).current_dir(repo);
    if let Some(ssh) = ssh_command {
        cmd.env("GIT_SSH_COMMAND", ssh);
    }
    let output = cmd
        .output()
        .map_err(|e| ConfigError::Message(format!("failed to run git: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(ConfigError::Message(if stderr.is_empty() {
            format!("git {:?} failed", args)
        } else {
            stderr
        }));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn build_ssh_command(profile: &Profile) -> Option<String> {
    let key = profile.ssh_key_path.trim();
    if key.is_empty() {
        return None;
    }
    let expanded = expand_tilde(key);
    Some(format!(
        "ssh -i {} -o IdentitiesOnly=yes",
        expanded.display()
    ))
}

fn remote_from_ref(ref_name: &str) -> Option<String> {
    let trimmed = ref_name.trim();
    if trimmed.is_empty() {
        return None;
    }
    trimmed.split('/').next().map(|s| s.to_string())
}

pub fn list_branches(repo: &Path) -> Result<Vec<BranchInfo>, ConfigError> {
    let current = run_git(repo, &["rev-parse", "--abbrev-ref", "HEAD"])
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| s != "HEAD");

    let local_raw = run_git(
        repo,
        &[
            "for-each-ref",
            "--format=%(refname:short)|%(upstream:short)",
            "refs/heads",
        ],
    )?;

    let mut branches = Vec::new();
    for line in local_raw.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let mut parts = line.splitn(2, '|');
        let name = parts.next().unwrap_or("").trim().to_string();
        let upstream = parts
            .next()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let remote = upstream.as_deref().and_then(remote_from_ref);
        let is_current = current.as_ref() == Some(&name);
        branches.push(BranchInfo {
            name,
            is_current,
            is_remote: false,
            upstream,
            remote,
        });
    }

    if let Ok(remote_raw) =
        run_git(repo, &["for-each-ref", "--format=%(refname:short)", "refs/remotes"])
    {
        for line in remote_raw.lines() {
            let name = line.trim();
            if name.is_empty() || name.ends_with("/HEAD") {
                continue;
            }
            let remote = remote_from_ref(name);
            branches.push(BranchInfo {
                name: name.to_string(),
                is_current: false,
                is_remote: true,
                upstream: None,
                remote,
            });
        }
    }

    branches.sort_by(|a, b| {
        b.is_current
            .cmp(&a.is_current)
            .then(a.is_remote.cmp(&b.is_remote))
            .then(a.remote.cmp(&b.remote))
            .then(a.name.cmp(&b.name))
    });

    Ok(branches)
}

pub fn checkout_branch(repo: &Path, branch: &str) -> Result<(), ConfigError> {
    if branch.is_empty() || branch.contains("..") || branch.starts_with('-') || branch.contains(' ')
    {
        return Err(ConfigError::Message("invalid branch name".into()));
    }

    let local_ref = format!("refs/heads/{branch}");
    if run_git(repo, &["show-ref", "--verify", &local_ref]).is_ok() {
        run_git(repo, &["switch", branch])?;
        return Ok(());
    }

    // Remote ref like origin/feature → create/switch local tracking branch
    if let Some((_, local)) = branch.rsplit_once('/') {
        if !local.is_empty() {
            let local_ref = format!("refs/heads/{local}");
            if run_git(repo, &["show-ref", "--verify", &local_ref]).is_ok() {
                run_git(repo, &["switch", local])?;
                return Ok(());
            }
            run_git(repo, &["switch", "--track", "-c", local, branch])?;
            return Ok(());
        }
    }

    run_git(repo, &["switch", branch])?;
    Ok(())
}

pub fn create_branch(repo: &Path, name: &str, checkout: bool) -> Result<(), ConfigError> {
    if name.is_empty()
        || name.contains("..")
        || name.starts_with('-')
        || name.contains(' ')
        || name.contains('\n')
    {
        return Err(ConfigError::Message("invalid branch name".into()));
    }
    if checkout {
        run_git(repo, &["switch", "-c", name])?;
    } else {
        run_git(repo, &["branch", name])?;
    }
    Ok(())
}

pub fn delete_branch(repo: &Path, name: &str, force: bool) -> Result<(), ConfigError> {
    if name.is_empty() || name.contains("..") || name.starts_with('-') {
        return Err(ConfigError::Message("invalid branch name".into()));
    }
    let flag = if force { "-D" } else { "-d" };
    run_git(repo, &["branch", flag, name])?;
    Ok(())
}

pub fn repo_status(repo: &Path) -> Result<RepoStatus, ConfigError> {
    let current_branch = run_git(repo, &["rev-parse", "--abbrev-ref", "HEAD"])
        .ok()
        .map(|s| s.trim().to_string());

    let porcelain = run_git(repo, &["status", "--porcelain"])?;
    let is_dirty = !porcelain.trim().is_empty();

    let mut ahead = 0u32;
    let mut behind = 0u32;
    if let Ok(counts) = run_git(repo, &["rev-list", "--left-right", "--count", "HEAD...@{upstream}"])
    {
        let parts: Vec<_> = counts.split_whitespace().collect();
        if parts.len() == 2 {
            ahead = parts[0].parse().unwrap_or(0);
            behind = parts[1].parse().unwrap_or(0);
        }
    }

    let remotes_raw = run_git(repo, &["remote", "-v"]).unwrap_or_default();
    let mut remotes = Vec::new();
    for line in remotes_raw.lines() {
        let parts: Vec<_> = line.split_whitespace().collect();
        if parts.len() >= 2 && parts.get(2).map(|s| s.contains("fetch")).unwrap_or(false) {
            remotes.push(RemoteInfo {
                name: parts[0].to_string(),
                url: parts[1].to_string(),
            });
        }
    }

    let user_name = run_git(repo, &["config", "--local", "--get", "user.name"])
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let user_email = run_git(repo, &["config", "--local", "--get", "user.email"])
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let ssh_command = run_git(repo, &["config", "--local", "--get", "core.sshCommand"])
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    Ok(RepoStatus {
        path: repo.display().to_string(),
        current_branch,
        is_dirty,
        ahead,
        behind,
        remotes,
        user_name,
        user_email,
        ssh_command,
    })
}

pub fn apply_profile(repo: &Path, profile: &Profile) -> Result<(), ConfigError> {
    if !profile.user_name.trim().is_empty() {
        run_git(repo, &["config", "--local", "user.name", profile.user_name.trim()])?;
    }
    if !profile.user_email.trim().is_empty() {
        run_git(
            repo,
            &["config", "--local", "user.email", profile.user_email.trim()],
        )?;
    }
    if let Some(ssh) = build_ssh_command(profile) {
        run_git(repo, &["config", "--local", "core.sshCommand", &ssh])?;
    } else {
        let _ = run_git(repo, &["config", "--local", "--unset", "core.sshCommand"]);
    }
    Ok(())
}

pub fn fetch(repo: &Path, profile: Option<&Profile>) -> Result<String, ConfigError> {
    let ssh = profile.and_then(build_ssh_command);
    run_git_with_env(repo, &["fetch", "--all", "--prune"], ssh.as_deref())
}

pub fn push(repo: &Path, profile: Option<&Profile>, set_upstream: bool) -> Result<String, ConfigError> {
    let ssh = profile.and_then(build_ssh_command);
    if set_upstream {
        let branch = run_git(repo, &["rev-parse", "--abbrev-ref", "HEAD"])?
            .trim()
            .to_string();
        run_git_with_env(
            repo,
            &["push", "-u", "origin", &branch],
            ssh.as_deref(),
        )
    } else {
        run_git_with_env(repo, &["push"], ssh.as_deref())
    }
}

pub fn recent_log(repo: &Path, limit: usize) -> Result<Vec<String>, ConfigError> {
    let n = limit.clamp(1, 50).to_string();
    let raw = run_git(
        repo,
        &["log", &format!("-{n}"), "--pretty=format:%h%x09%s%x09%an%x09%ar"],
    )?;
    Ok(raw.lines().map(|s| s.to_string()).filter(|s| !s.is_empty()).collect())
}
