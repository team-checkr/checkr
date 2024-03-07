use std::{path::Path, process::Stdio};

use color_eyre::eyre::{bail, Context};
use tokio::process::Command;

static GIT_SSH_SEMAPHORE: stdx::concurrency::Semaphore = stdx::concurrency::semaphore();

static SSH_CONTROL_FOLDER: once_cell::sync::Lazy<tempfile::TempDir> =
    once_cell::sync::Lazy::new(|| {
        tempfile::tempdir().expect("could not create temporary directory for ssh control path")
    });
static GIT_SSH_COMMAND: once_cell::sync::Lazy<String> = once_cell::sync::Lazy::new(|| {
    format!(
        "ssh -o ControlPath={control_path}/%r@%h:%p -o ControlMaster=auto -o ControlPersist=60",
        control_path = SSH_CONTROL_FOLDER.path().display()
    )
});

pub async fn clone_or_pull(git: &str, path: impl AsRef<Path>) -> color_eyre::Result<()> {
    let path = path.as_ref();
    if !path.join(".git").try_exists().unwrap_or(false) {
        clone(git, path).await
    } else {
        checkout_main(git, path).await?;
        pull(git, path).await
    }
}

pub async fn clone(git: &str, path: impl AsRef<Path>) -> color_eyre::Result<()> {
    let _permit = GIT_SSH_SEMAPHORE.acquire().await;

    tracing::debug!(?git, "cloning group git repository");
    let status = Command::new("git")
        .arg("clone")
        .arg(git)
        .args(["."])
        .env("GIT_SSH_COMMAND", GIT_SSH_COMMAND.as_str())
        .current_dir(path)
        .stderr(Stdio::inherit())
        .stdout(Stdio::inherit())
        .status()
        .await
        .wrap_err_with(|| format!("could not clone group git repository: '{git}'"))?;
    tracing::debug!(code=?status.code(), "git clone status");
    if !status.success() {
        bail!("git clone failed");
    }
    Ok(())
}

pub async fn checkout_main(git: &str, path: impl AsRef<Path>) -> color_eyre::Result<()> {
    tracing::debug!(?git, "checking out main branch");
    let status = Command::new("git")
        .arg("checkout")
        .arg("main")
        .env("GIT_SSH_COMMAND", GIT_SSH_COMMAND.as_str())
        .current_dir(path)
        .stderr(Stdio::inherit())
        .stdout(Stdio::inherit())
        .status()
        .await
        .wrap_err_with(|| {
            format!("could not checkout main branch of group git repository: '{git}'")
        })?;
    tracing::debug!(code=?status.code(), "git checkout status");
    if !status.success() {
        bail!("git checkout failed");
    }
    Ok(())
}

pub async fn pull(git: &str, path: impl AsRef<Path>) -> color_eyre::Result<()> {
    let _permit = GIT_SSH_SEMAPHORE.acquire().await;

    tracing::debug!(?git, "pulling group git repository");
    let status = Command::new("git")
        .arg("pull")
        .env("GIT_SSH_COMMAND", GIT_SSH_COMMAND.as_str())
        .current_dir(&path)
        .stderr(Stdio::inherit())
        .stdout(Stdio::inherit())
        .status()
        .await
        .wrap_err_with(|| format!("could not pull group git repository: '{git}'"))?;
    tracing::debug!(code=?status.code(), "git pull status");
    if !status.success() {
        bail!("git pull failed");
    }
    Ok(())
}

pub async fn hash(path: impl AsRef<Path>) -> color_eyre::Result<String> {
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("HEAD")
        .current_dir(path)
        .stderr(Stdio::inherit())
        .stdout(Stdio::piped())
        .output()
        .await
        .wrap_err("could not get git hash")?;
    if !output.status.success() {
        bail!("git rev-parse HEAD failed");
    }
    let hash = String::from_utf8(output.stdout).wrap_err("git hash is not valid utf8")?;
    Ok(hash.trim().to_string())
}

pub async fn checkout_latest_before(
    git: &str,
    path: impl AsRef<Path>,
    before: chrono::DateTime<chrono::FixedOffset>,
    ignored_authors: &[String],
) -> color_eyre::Result<bool> {
    let path = path.as_ref();
    checkout_main(git, path).await?;
    tracing::debug!(?before, "checking out latest commit before");
    let before = before.format("%Y-%m-%d %H:%M:%S").to_string();
    let mut cmd = Command::new("git");
    cmd.args(["rev-list", "-n", "1"])
        .arg(format!("--before={before}"))
        .arg("--perl-regexp")
        .arg(format!("--author='^(?!{})'", ignored_authors.join("|")))
        .arg("HEAD")
        .current_dir(path);
    let result = cmd
        .output()
        .await
        .wrap_err_with(|| format!("could not get latest commit before {before}"))?;
    if !result.status.success() {
        bail!("git rev-list failed");
    }
    let commit_rev_bytes = result.stdout;
    let commit_rev = std::str::from_utf8(&commit_rev_bytes).unwrap().trim();
    if commit_rev.is_empty() {
        tracing::warn!("no commit found before {before}");
        return Ok(false);
    }
    tracing::debug!(?commit_rev, ?path, "latest commit before");
    let result = Command::new("git")
        .arg("checkout")
        .arg(commit_rev)
        .current_dir(path)
        .stderr(Stdio::inherit())
        .stdout(Stdio::inherit())
        .output()
        .await
        .wrap_err_with(|| format!("could not checkout latest commit: {commit_rev}"))?;
    tracing::debug!(?result);
    if !result.status.success() {
        bail!("git checkout failed");
    }
    Ok(true)
}
