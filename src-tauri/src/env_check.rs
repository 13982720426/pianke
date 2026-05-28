//! 环境检测：Python 版本、磁盘空间、内存。

use std::path::Path;
use std::process::Command;

use serde::Serialize;

/// 环境检测结果，返回给前端展示。
#[derive(Serialize, Clone)]
pub struct EnvInfo {
    /// Python 版本字符串，如 "Python 3.12.3"，None 表示未找到
    pub python_version: Option<String>,
    /// 当前工作目录所在磁盘的剩余空间（GB）
    pub disk_free_gb: f64,
    /// 系统总内存（GB）
    pub memory_gb: f64,
    /// 不阻塞启动的提示信息
    pub warnings: Vec<String>,
    /// 阻塞启动的错误
    pub errors: Vec<String>,
}

/// 在系统中查找 Python 3.10+，返回版本字符串和路径。
pub fn find_python() -> Option<(String, String)> {
    // 优先检测 python3，再试 python
    for name in &["python3", "python"] {
        if let Ok(output) = Command::new(name).arg("--version").output() {
            if output.status.success() {
                let ver = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if ver.is_empty() {
                    let ver = String::from_utf8_lossy(&output.stderr).trim().to_string();
                    if !ver.is_empty() && (ver.contains("3.10") || ver.contains("3.11")
                        || ver.contains("3.12") || ver.contains("3.13"))
                    {
                        return Some((ver, name.to_string()));
                    }
                } else if ver.contains("3.10") || ver.contains("3.11")
                    || ver.contains("3.12") || ver.contains("3.13")
                {
                    return Some((ver, name.to_string()));
                }
            }
        }
    }
    None
}

/// 如果传入了 venv Python 路径，优先用它检测版本。
/// 否则用系统 PATH 中的 Python。
fn detect_python(venv_python: Option<&Path>) -> Option<(String, String)> {
    if let Some(vp) = venv_python {
        if vp.exists() {
            if let Ok(output) = Command::new(vp).arg("--version").output() {
                if output.status.success() {
                    let ver = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !ver.is_empty() {
                        return Some((ver, vp.to_string_lossy().to_string()));
                    }
                }
            }
        }
    }
    find_python()
}

/// 获取磁盘剩余空间（GB）
fn disk_free_gb(path: &Path) -> f64 {
    use sysinfo::Disks;
    let disks = Disks::new_with_refreshed_list();
    for disk in &disks {
        if path.to_string_lossy().starts_with(&disk.mount_point().to_string_lossy().as_ref()) {
            return disk.available_space() as f64 / 1_073_741_824.0;
        }
    }
    0.0
}

/// 获取系统总内存（GB）
fn total_memory_gb() -> f64 {
    let sys = sysinfo::System::new_all();
    sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0
}

/// 执行完整环境检测。
/// venv_python: 如果 venv 已创建，传入 venv 的 Python 路径。
pub fn check(venv_python: Option<&Path>) -> EnvInfo {
    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    // 检测 Python
    let python_version = detect_python(venv_python).map(|(v, _)| v);

    if python_version.is_none() && venv_python.is_none() {
        errors.push("未找到 Python 3.10+。请安装 Python 或稍后通过 uv 自动安装。".into());
    }

    // 磁盘空间
    let cwd = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
    let disk_free = disk_free_gb(&cwd);

    if disk_free < 1.0 {
        errors.push(format!("磁盘空间不足：仅剩 {:.1} GB", disk_free));
    } else if disk_free < 5.0 {
        warnings.push(format!("磁盘空间偏小：{:.1} GB，专家模式可能需要 3GB+ 临时空间", disk_free));
    }

    // 内存
    let mem_gb = total_memory_gb();
    if mem_gb < 2.0 {
        errors.push(format!("内存不足：仅 {:.1} GB，专家模式需要至少 4GB", mem_gb));
    } else if mem_gb < 4.0 {
        warnings.push(format!("内存偏小：{:.1} GB，建议使用极速模式", mem_gb));
    }

    EnvInfo {
        python_version,
        disk_free_gb: disk_free,
        memory_gb: mem_gb,
        warnings,
        errors,
    }
}
