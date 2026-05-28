//! GitHub 更新检查与自动更新。

use std::path::Path;
use std::process::Command;

use crate::launcher::{self, InstallState};

const GITHUB_OWNER: &str = "zhaoyue4810";
const GITHUB_REPO: &str = "pianke";
const GITHUB_BRANCH: &str = "main";

/// 从 GitHub API 获取远程最新提交 SHA。
fn remote_commit_sha() -> Option<String> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/commits/{}",
        GITHUB_OWNER, GITHUB_REPO, GITHUB_BRANCH
    );

    let output = Command::new("curl")
        .args([
            "-s",
            "-H",
            "User-Agent: pianke-updater",
            "--connect-timeout",
            "8",
            "--max-time",
            "10",
            &url,
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let body = String::from_utf8_lossy(&output.stdout);
    let sha = body
        .split("\"sha\":\"")
        .nth(1)?
        .split('\"')
        .next()?
        .to_string();

    Some(sha)
}

/// 检查并应用更新。
/// 传入 venv Python 路径用于 HTTP 请求（复用 curl）和 tar 解压。
pub fn check_and_apply(
    app_dir: &Path,
    state_path: &Path,
    _venv_python: &Path,
    status_cb: &mut impl FnMut(&str),
) {
    let install_state = launcher::load_install_state(state_path);
    let local_sha = install_state.commit_sha.clone();

    let remote_sha = match remote_commit_sha() {
        Some(s) => s,
        None => {
            status_cb("无法连接 GitHub 检查更新，跳过");
            return;
        }
    };

    if local_sha == remote_sha {
        status_cb(&format!("已是最新版本（{}）", &remote_sha[..8]));
        return;
    }

    if local_sha.is_empty() {
        // 首次运行，只记录 SHA，不覆盖
        status_cb(&format!("标记当前版本为 {}", &remote_sha[..8]));
        let new_state = InstallState {
            commit_sha: remote_sha,
            ..install_state
        };
        launcher::save_install_state(state_path, &new_state);
        return;
    }

    status_cb(&format!(
        "发现新版本 {}（当前 {}），正在下载...",
        &remote_sha[..8],
        if local_sha.len() >= 8 { &local_sha[..8] } else { &local_sha }
    ));

    // 下载 tarball
    let tarball_url = format!(
        "https://codeload.github.com/{}/{}/tar.gz/{}",
        GITHUB_OWNER, GITHUB_REPO, remote_sha
    );

    let tar_path = app_dir.join(".update.tar.gz");
    let download_output = Command::new("curl")
        .args([
            "-sL",
            "--connect-timeout",
            "15",
            "--max-time",
            "60",
            "-o",
            &tar_path.to_string_lossy(),
            &tarball_url,
        ])
        .output();

    if let Err(e) = download_output {
        status_cb(&format!("下载失败: {}", e));
        return;
    }

    if !tar_path.exists() {
        status_cb("下载失败：文件未创建");
        return;
    }

    // 解压并应用更新
    match apply_tarball(&tar_path, app_dir) {
        Ok(()) => {
            let new_state = InstallState {
                commit_sha: remote_sha.clone(),
                ..install_state
            };
            launcher::save_install_state(state_path, &new_state);
            status_cb(&format!("已更新到版本 {}", &remote_sha[..8]));
        }
        Err(e) => {
            status_cb(&format!("更新失败: {}", e));
        }
    }
}

/// 解压 tarball 并覆盖 app_dir 中的文件。
fn apply_tarball(tar_path: &Path, app_dir: &Path) -> anyhow::Result<()> {
    // 使用系统 tar 命令解压
    let tmp_dir = app_dir.join(".update_tmp");
    let _ = std::fs::remove_dir_all(&tmp_dir);
    std::fs::create_dir_all(&tmp_dir)?;

    let output = Command::new("tar")
        .args([
            "-xzf",
            &tar_path.to_string_lossy(),
            "-C",
            &tmp_dir.to_string_lossy(),
        ])
        .output()?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("解压失败: {}", err);
    }

    // tarball 顶层是 pianke-<sha>/，取里面内容
    let entries: Vec<_> = std::fs::read_dir(&tmp_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .collect();

    if entries.len() != 1 {
        let _ = std::fs::remove_dir_all(&tmp_dir);
        anyhow::bail!("更新包结构异常");
    }

    let src = entries[0].path();

    // 保留文件/目录列表（不被覆盖）
    let preserve: Vec<&str> = vec![".venv", ".pic_selecter_install.json", "models", "__pycache__", ".git"];

    // 复制每个项目到 app_dir
    for entry in std::fs::read_dir(&src)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if preserve.contains(&name.as_str()) {
            continue;
        }
        let target = app_dir.join(&name);
        if target.exists() {
            if target.is_dir() {
                let _ = std::fs::remove_dir_all(&target);
            } else {
                let _ = std::fs::remove_file(&target);
            }
        }
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&entry.path(), &target)?;
        } else {
            let _ = std::fs::copy(&entry.path(), &target);
        }
    }

    // 清理临时文件
    let _ = std::fs::remove_dir_all(&tmp_dir);
    let _ = std::fs::remove_file(tar_path);

    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let target = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&entry.path(), &target)?;
        } else {
            let _ = std::fs::copy(&entry.path(), &target);
        }
    }
    Ok(())
}
