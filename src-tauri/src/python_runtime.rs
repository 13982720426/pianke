//! Python 运行时管理：资源提取、venv 创建、依赖安装、Flask 启动。

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use crate::launcher::MirrorConfig;

/// Python 运行时。
pub struct PythonRuntime {
    /// venv 中的 Python 路径
    pub venv_python: PathBuf,
    /// 应用代码目录
    pub app_dir: PathBuf,
}

impl PythonRuntime {
    /// 将资源文件从资源目录提取到应用数据目录。
    /// 处理 Tauri 资源打包的两种格式：
    ///   - `Resources/app.py` （resources 路径不包含 ../ 时）
    ///   - `Resources/_up_/app.py` （resources 路径包含 ../ 时，Tauri 用 _up_ 表示上一级）
    pub fn extract_resources(resource_dir: &Path, app_data_dir: &Path) -> anyhow::Result<PathBuf> {
        let app_dir = app_data_dir.join("app");

        // 检查是否已提取（快速跳过）
        if app_dir.join("app.py").exists() {
            return Ok(app_dir);
        }

        log::info!(
            "Extracting resources from {:?} to {:?}",
            resource_dir,
            app_dir
        );

        // 确定真正的资源根目录：处理 _up_ 目录
        let src_dir = if resource_dir.join("_up_").join("app.py").exists() {
            resource_dir.join("_up_")
        } else {
            resource_dir.to_path_buf()
        };

        // 递归复制
        let _ = std::fs::create_dir_all(&app_dir);
        Self::copy_dir(&src_dir, &app_dir)?;

        if !app_dir.join("app.py").exists() {
            // 开发模式回退：当前目录的父目录（项目根目录）
            let cwd = std::env::current_dir()?;
            let dev_root = cwd.parent().unwrap_or(&resource_dir);
            if dev_root.join("app.py").exists() {
                return Ok(dev_root.to_path_buf());
            }
            // 再试一次：cwd 本身（直接用 cargo run 时）
            if cwd.join("app.py").exists() {
                return Ok(cwd);
            }
            anyhow::bail!(
                "未找到 app.py（尝试路径: {:?}, {:?}, {:?}）",
                resource_dir.join("app.py"),
                resource_dir.join("_up_").join("app.py"),
                dev_root.join("app.py")
            );
        }

        Ok(app_dir)
    }

    fn copy_dir(src: &Path, dst: &Path) -> anyhow::Result<()> {
        if !src.exists() {
            return Ok(());
        }
        // 使用递归函数替代 walkdir
        copy_dir_recursive_inner(src, dst)
    }

    /// 设置 Python 虚拟环境。
    /// 如果系统有 uv，用 uv 加速；否则用 python3 -m venv。
    /// ensure_uv: 如果没有 uv，是否下载安装 uv。
    pub fn setup_venv(
        python_path: Option<&Path>,
        app_data_dir: &Path,
        ensure_uv: bool,
        status_cb: impl Fn(&str),
    ) -> anyhow::Result<PathBuf> {
        let venv_dir = app_data_dir.join(".venv");
        let venv_python = if cfg!(windows) {
            venv_dir.join("Scripts").join("python.exe")
        } else {
            venv_dir.join("bin").join("python")
        };

        // 如果 venv 已存在且可用，直接返回
        if venv_python.exists() {
            if let Ok(output) = Command::new(&venv_python).arg("--version").output() {
                if output.status.success() {
                    return Ok(venv_python);
                }
            }
            // venv 损坏，重新创建
            let _ = std::fs::remove_dir_all(&venv_dir);
        }

        // 找 uv。setup 阶段（ensure_uv=false）不允许 uv 下载 Python，避免窗口显示前卡住。
        let uv_path = Self::find_uv_for_paths(Some(app_data_dir), None);
        if let Some(uv) = &uv_path {
            if !ensure_uv && python_path.is_none() {
                status_cb("已找到 uv，等待安装页确认后再下载 Python...");
            } else {
                status_cb("使用 uv 创建虚拟环境...");
                let mut cmd = Command::new(uv);
                let python = python_path
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| ">=3.10".to_string());
                cmd.args(["venv", &venv_dir.to_string_lossy(), "--python", &python]);
                let output = cmd.output()?;
                if !output.status.success() {
                    let err = String::from_utf8_lossy(&output.stderr);
                    anyhow::bail!("uv venv 失败: {}", err);
                }
                status_cb("虚拟环境已就绪 (uv)");
                return Ok(venv_python);
            }
        }

        // 没有 uv，尝试用系统 Python 创建
        if let Some(py) = python_path {
            if !py.as_os_str().is_empty() {
                status_cb("使用系统 Python 创建虚拟环境...");
                let mut cmd = Command::new(py);
                cmd.args(["-m", "venv", &venv_dir.to_string_lossy()]);
                let output = cmd.output()?;
                if !output.status.success() {
                    let err = String::from_utf8_lossy(&output.stderr);
                    anyhow::bail!("venv 创建失败: {}", err);
                }
                // 升级 pip
                let _ = Command::new(&venv_python)
                    .args(["-m", "pip", "install", "--upgrade", "pip"])
                    .output();
                status_cb("虚拟环境已就绪 (venv)");
                return Ok(venv_python);
            }
        }

        // 既没有 uv 也没有系统 Python，尝试下载 uv
        if ensure_uv {
            status_cb("正在下载 uv（Python 工具链管理器）...");
            match Self::install_uv() {
                Ok(uv) => {
                    status_cb("uv 安装成功，正在创建虚拟环境...");
                    let mut cmd = Command::new(&uv);
                    cmd.args(["venv", &venv_dir.to_string_lossy(), "--python", ">=3.10"]);
                    let output = cmd.output()?;
                    if !output.status.success() {
                        let err = String::from_utf8_lossy(&output.stderr);
                        anyhow::bail!("uv venv 失败: {}", err);
                    }
                    status_cb("虚拟环境已就绪 (uv)");
                    return Ok(venv_python);
                }
                Err(e) => {
                    anyhow::bail!("无法下载 uv: {}", e);
                }
            }
        }

        anyhow::bail!("未找到 Python 3.10+，且无法自动安装");
    }

    fn uv_file_name() -> &'static str {
        if cfg!(windows) {
            "uv.exe"
        } else {
            "uv"
        }
    }

    fn existing_file(path: PathBuf) -> Option<PathBuf> {
        path.is_file().then_some(path)
    }

    fn find_uv_for_paths(app_data_dir: Option<&Path>, app_dir: Option<&Path>) -> Option<PathBuf> {
        // 先查打包进应用资源的 uv。resources 会被提取到 app_data_dir/app。
        if let Some(dir) = app_dir {
            if let Some(path) =
                Self::existing_file(dir.join("scripts").join("bin").join(Self::uv_file_name()))
            {
                return Some(path);
            }
        }
        if let Some(dir) = app_data_dir {
            if let Some(path) = Self::existing_file(
                dir.join("app")
                    .join("scripts")
                    .join("bin")
                    .join(Self::uv_file_name()),
            ) {
                return Some(path);
            }
        }

        // 检查 PATH
        if let Some(path) = std::env::var_os("PATH") {
            for dir in std::env::split_paths(&path) {
                if let Some(path) = Self::existing_file(dir.join(Self::uv_file_name())) {
                    return Some(path);
                }
            }
        }

        // 检查常见位置
        let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"));
        if let Some(home) = home {
            let home = PathBuf::from(home);
            for dir in &[
                home.join(".local").join("bin"),
                home.join(".cargo").join("bin"),
            ] {
                if let Some(path) = Self::existing_file(dir.join(Self::uv_file_name())) {
                    return Some(path);
                }
            }
        }
        None
    }

    fn find_uv() -> Option<PathBuf> {
        Self::find_uv_for_paths(None, None)
    }

    fn find_uv_for_app(&self) -> Option<PathBuf> {
        Self::find_uv_for_paths(None, Some(&self.app_dir))
    }

    fn install_uv() -> anyhow::Result<PathBuf> {
        if cfg!(windows) {
            let output = Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-Command",
                    "irm https://astral.sh/uv/install.ps1 | iex",
                ])
                .output()?;

            if !output.status.success() {
                let err = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("uv 安装失败: {}", err);
            }

            return Self::find_uv().ok_or_else(|| anyhow::anyhow!("uv 安装后未找到"));
        }

        let output = Command::new("curl")
            .args(["-LSf", "https://astral.sh/uv/install.sh"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("下载 uv 失败: {}", err);
        }

        // 通过 sh 执行安装脚本
        let mut child = Command::new("sh")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        use std::io::Write;
        if let Some(stdin) = child.stdin.as_mut() {
            stdin.write_all(&output.stdout)?;
        }
        let install_output = child.wait_with_output()?;
        if !install_output.status.success() {
            let err = String::from_utf8_lossy(&install_output.stderr);
            anyhow::bail!("uv 安装失败: {}", err);
        }

        Self::find_uv().ok_or_else(|| anyhow::anyhow!("uv 安装后未找到"))
    }

    /// 安装 pip 包。
    pub fn install_packages(
        &self,
        packages: &[String],
        mirror: &MirrorConfig,
        progress_cb: impl Fn(&str),
    ) -> anyhow::Result<()> {
        if packages.is_empty() {
            return Ok(());
        }

        // 使用 uv 安装（更快）
        if let Some(uv) = self.find_uv_for_app().or_else(Self::find_uv) {
            let mut cmd = Command::new(&uv);
            cmd.args([
                "pip",
                "install",
                "--python",
                &self.venv_python.to_string_lossy(),
            ]);

            if mirror.use_mirror {
                cmd.args(["--index-url", &mirror.pypi_url]);
                cmd.args(["--extra-index-url", "https://pypi.org/simple/"]);
            }

            for pkg in packages {
                cmd.arg(pkg);
            }

            progress_cb(&format!("正在安装 {} 个依赖包...", packages.len()));
            let output = cmd.output()?;
            if !output.status.success() {
                let _err = String::from_utf8_lossy(&output.stderr);
                // 如果 uv 失败，回退到 pip
                progress_cb("uv 安装失败，回退到 pip...");
                return self.install_packages_pip(packages, mirror, progress_cb);
            }

            // 修复 OpenCV 冲突
            self.fix_opencv(&progress_cb)?;

            progress_cb("依赖安装完成");
            return Ok(());
        }

        // 回退到 pip
        self.install_packages_pip(packages, mirror, progress_cb)
    }

    fn install_packages_pip(
        &self,
        packages: &[String],
        mirror: &MirrorConfig,
        progress_cb: impl Fn(&str),
    ) -> anyhow::Result<()> {
        let mut cmd = Command::new(&self.venv_python);
        cmd.args(["-m", "pip", "install", "--disable-pip-version-check"]);

        if mirror.use_mirror {
            cmd.args(["-i", &mirror.pypi_url]);
            cmd.args(["--extra-index-url", "https://pypi.org/simple/"]);
        }

        for pkg in packages {
            cmd.arg(pkg);
        }

        progress_cb("正在用 pip 安装依赖（可能需要几分钟）...");
        let output = cmd.output()?;
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("pip 安装失败: {}", err);
        }

        // 修复 OpenCV 冲突
        self.fix_opencv(&progress_cb)?;

        progress_cb("依赖安装完成");
        Ok(())
    }

    /// 修复 OpenCV 包冲突：确保只有 opencv-contrib-python 被安装。
    fn fix_opencv(&self, progress_cb: &impl Fn(&str)) -> anyhow::Result<()> {
        // 检查是否有冲突包
        let check = Command::new(&self.venv_python)
            .args([
                "-c",
                "import importlib.metadata as m; names={'opencv-python','opencv-python-headless'}; found=[n for n in names if any(d.metadata['Name'].lower()==n for d in m.distributions())]; print('|'.join(found))",
            ])
            .output()?;

        let output_str = String::from_utf8_lossy(&check.stdout).to_string();
        let conflicts: Vec<String> = output_str
            .trim()
            .split('|')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        if conflicts.is_empty() {
            return Ok(());
        }

        progress_cb("检测到 OpenCV 冲突包，正在修复...");

        // 卸载冲突包
        let mut uninstall = Command::new(&self.venv_python);
        uninstall.args(["-m", "pip", "uninstall", "-y"]);
        for c in &conflicts {
            uninstall.arg(c);
        }
        uninstall.output()?;

        // 重新安装 contrib 版
        let uv = self.find_uv_for_app().or_else(Self::find_uv);
        let mut reinstall = Command::new(uv.as_ref().unwrap_or(&self.venv_python));
        if uv.is_some() {
            reinstall.args([
                "pip",
                "install",
                "--python",
                &self.venv_python.to_string_lossy(),
            ]);
        } else {
            reinstall.arg("-m").arg("pip").arg("install");
        }
        reinstall.args([
            "--force-reinstall",
            "--no-deps",
            "opencv-contrib-python>=4.9",
        ]);
        reinstall.output()?;

        progress_cb("OpenCV 已修复");
        Ok(())
    }

    /// 检查依赖是否已就绪。
    pub fn deps_ready(&self, modes: &[String]) -> bool {
        let check_script =
            if modes.contains(&"expert".to_string()) || modes.contains(&"tycoon".to_string()) {
                // 检查 torch 和 transformers
                "import torch; import transformers; import cv2; print('ok')"
            } else {
                // 仅检查核心包
                "import cv2; from PIL import Image; import numpy; print('ok')"
            };

        Command::new(&self.venv_python)
            .args(["-c", check_script])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// 启动 Flask 子进程。
    pub fn start_flask(&self, port: u16, mirror: &MirrorConfig) -> anyhow::Result<Child> {
        let app_script = self.app_dir.join("app.py");

        let mut cmd = Command::new(&self.venv_python);
        cmd.args([
            app_script.to_string_lossy().as_ref(),
            "--port",
            &port.to_string(),
            "--runtime",
            "auto",
            "--no-browser",
        ])
        // ⚠️ 重要：使用 inherit 而非 piped，避免 pipe buffer 满导致 Flask 进程卡死。
        // 如果未来需要捕获 Flask 日志，应该用线程单独读取管道。
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .current_dir(&self.app_dir);

        // 设置镜像环境变量
        if mirror.use_mirror {
            cmd.env("HF_ENDPOINT", &mirror.hf_url);
        }

        let child = cmd.spawn()?;
        log::info!("Flask 子进程已启动 (PID {})", child.id());
        Ok(child)
    }
}

/// 递归复制目录（替代 walkdir + copy 组合）。
fn copy_dir_recursive_inner(src: &Path, dst: &Path) -> anyhow::Result<()> {
    if !src.is_dir() {
        return Ok(());
    }
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let target = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_recursive_inner(&entry.path(), &target)?;
        } else if file_type.is_file() {
            let _ = std::fs::copy(entry.path(), &target);
        }
    }
    Ok(())
}
