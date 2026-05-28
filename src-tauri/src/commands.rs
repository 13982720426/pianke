//! Tauri IPC 命令 — 前端与 Rust 后端的通信桥梁。

use std::process::Child;
use std::sync::Mutex;
use std::time::Duration;

use tauri::{command, AppHandle, Emitter, Manager, State};

use serde::Serialize;

use crate::env_check;
use crate::launcher::{self, state_file_path, AppState, MirrorConfig};
use crate::python_runtime::PythonRuntime;
use crate::updater;

/// 管理 Flask 子进程的生命周期。
pub struct FlaskProcess(pub Mutex<Option<Child>>);

/// init_setup 的返回值。
#[derive(Serialize)]
pub struct InitResult {
    pub python_version: Option<String>,
    pub disk_free_gb: f64,
    pub memory_gb: f64,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub status_msgs: Vec<String>,
    pub auto_start_modes: Option<Vec<String>>,
    /// 运行模式：dev 或 bundled
    pub mode: String,
}

/// 初始化设置 — 前端加载完成后第一个调用的命令。
#[command]
pub fn init_setup(app: AppHandle, state: State<'_, AppState>) -> InitResult {
    let state_path = state_file_path(&state.app_data_dir);
    let mut status_msgs: Vec<String> = Vec::new();

    // 确定运行模式
    let mode =
        if state.resource_dir.join("app.py").exists() || state.app_dir.join("app.py").exists() {
            "bundled"
        } else {
            "dev"
        };
    status_msgs.push(format!("运行模式: {}", mode));

    // ─── 步骤 1：确保 Python 环境就绪 ───
    {
        let mut vp = state.venv_python.lock().unwrap();
        if vp.as_os_str().is_empty() {
            let _ = app.emit("setup:status", "未找到 Python，正在尝试通过 uv 安装...");
            status_msgs.push("未找到 Python，正在尝试通过 uv 安装...".into());

            match PythonRuntime::setup_venv(
                (!state.python_path.as_os_str().is_empty())
                    .then(|| &state.python_path)
                    .map(|v| &**v),
                &state.app_data_dir,
                true,
                |msg| {
                    let _ = app.emit("setup:status", msg.to_string());
                },
            ) {
                Ok(venv_path) => {
                    *vp = venv_path;
                    let _ = app.emit("setup:status", "Python 环境已就绪");
                    status_msgs.push("Python 环境已就绪".into());
                }
                Err(e) => {
                    let msg = format!("Python 环境创建失败: {}", e);
                    let _ = app.emit("setup:status", &msg);
                    status_msgs.push(msg);
                }
            }
        }
    }

    // ─── 步骤 2：环境检测 ───
    let venv_ref = {
        let vp = state.venv_python.lock().unwrap();
        (!vp.as_os_str().is_empty()).then(|| vp.clone())
    };
    let env = env_check::check(venv_ref.as_deref());

    // ─── 步骤 3：更新检查（仅 bundled 模式） ───
    if mode == "bundled" {
        status_msgs.push("正在检查更新...".into());
        let vp = state.venv_python.lock().unwrap().clone();
        if !vp.as_os_str().is_empty() {
            updater::check_and_apply(&state.app_dir, &state_path, &vp, &mut |msg: &str| {
                let _ = app.emit("setup:status", msg.to_string());
                status_msgs.push(msg.to_string());
            });
        } else {
            status_msgs.push("跳过更新检查（Python 环境未就绪）".into());
        }
    }

    // ─── 步骤 4：自动启动检测 ───
    let auto_start_modes = {
        let install_state = launcher::load_install_state(&state_path);
        let vp = state.venv_python.lock().unwrap();
        if !install_state.modes.is_empty() && !vp.as_os_str().is_empty() {
            let runtime = PythonRuntime {
                venv_python: vp.clone(),
                app_dir: state.app_dir.clone(),
            };
            let packages = launcher::packages_for_modes(&install_state.modes);
            let sig = packages.join("|");
            let sig_changed = install_state.packages_sig != sig;
            if !sig_changed && runtime.deps_ready(&install_state.modes) {
                Some(install_state.modes)
            } else {
                None
            }
        } else {
            None
        }
    };

    InitResult {
        python_version: env.python_version,
        disk_free_gb: env.disk_free_gb,
        memory_gb: env.memory_gb,
        warnings: env.warnings,
        errors: env.errors,
        status_msgs,
        auto_start_modes,
        mode: mode.to_string(),
    }
}

/// 返回功能模式列表，标记上次选择的模式为 selected。
#[command]
pub fn get_modes(state: State<'_, AppState>) -> Vec<launcher::ModeInfo> {
    let install_state = launcher::load_install_state(&state_file_path(&state.app_data_dir));
    launcher::get_modes(&install_state.modes)
}

/// 开始安装依赖并启动 Flask 服务。
#[command]
pub fn start_setup(
    app: AppHandle,
    state: State<'_, AppState>,
    _flask: State<'_, FlaskProcess>,
    modes: Vec<String>,
) -> Result<(), String> {
    let valid_keys: Vec<String> = launcher::get_modes(&[])
        .into_iter()
        .map(|m| m.key)
        .collect();

    for m in &modes {
        if !valid_keys.contains(m) {
            return Err(format!("未知模式: {}", m));
        }
    }
    if modes.is_empty() {
        return Err("请至少选择一个模式".into());
    }
    if state.venv_python.lock().unwrap().as_os_str().is_empty() {
        return Err("无法找到 Python，请安装 Python 3.10+ 或安装 uv".into());
    }

    let mirror = MirrorConfig::default();
    let packages = launcher::packages_for_modes(&modes);
    let packages_sig = packages.join("|");

    let state_path = state_file_path(&state.app_data_dir);
    let saved_state = launcher::load_install_state(&state_path);
    let install_state = launcher::InstallState {
        modes: modes.clone(),
        packages_sig: packages_sig.clone(),
        commit_sha: saved_state.commit_sha,
    };
    launcher::save_install_state(&state_path, &install_state);

    let runtime = PythonRuntime {
        venv_python: state.venv_python.lock().unwrap().clone(),
        app_dir: state.app_dir.clone(),
    };

    let port: u16 = std::env::var("PIC_SELECTER_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5057);

    let need_install = saved_state.packages_sig != packages_sig || !runtime.deps_ready(&modes);

    // 后台线程：安装 + 启动 + 导航
    std::thread::spawn(move || {
        if need_install {
            let _ = app.emit(
                "setup:status",
                format!(
                    "准备安装 {} 个依赖包（首次可能需要几分钟）...",
                    packages.len()
                ),
            );

            if let Err(e) = runtime.install_packages(&packages, &mirror, |msg| {
                let _ = app.emit("setup:progress", msg.to_string());
            }) {
                let _ = app.emit("setup:error", format!("依赖安装失败: {}", e));
                return;
            }
        } else {
            let _ = app.emit("setup:status", "依赖已就绪，正在启动服务...".to_string());
        }

        match runtime.start_flask(port, &mirror) {
            Ok(child) => {
                if let Some(f) = app.try_state::<FlaskProcess>() {
                    *f.0.lock().unwrap() = Some(child);
                }

                let _ = app.emit("setup:status", "等待服务就绪...".to_string());
                // macOS 上 OpenCV 首次加载可能 20+ 秒，模型下载也耗时，给够 180 秒
                match wait_for_server(port, Duration::from_secs(180)) {
                    Ok(_) => {
                        let url = format!("http://localhost:{}", port);
                        let _ = app.emit("setup:ready", url.clone());
                        if let Some(w) = app.get_webview_window("main") {
                            if let Ok(parsed) = url::Url::parse(&url) {
                                let _ = w.navigate(parsed);
                            }
                        }

                        // Flask 存活监控
                        let mut failures: u32 = 0;
                        loop {
                            std::thread::sleep(Duration::from_secs(30));
                            if check_server(port) {
                                failures = 0;
                            } else {
                                failures += 1;
                                if failures >= 3 {
                                    let _ =
                                        app.emit("setup:error", "Flask 服务已无响应，请重启应用");
                                    break;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = app.emit("setup:error", format!("服务启动超时: {}", e));
                    }
                }
            }
            Err(e) => {
                let _ = app.emit("setup:error", format!("无法启动服务: {}", e));
            }
        }
    });

    Ok(())
}

fn check_server(port: u16) -> bool {
    std::net::TcpStream::connect_timeout(
        &std::net::SocketAddr::from(([127, 0, 0, 1], port)),
        Duration::from_secs(2),
    )
    .is_ok()
}

fn wait_for_server(port: u16, timeout: Duration) -> anyhow::Result<()> {
    let start = std::time::Instant::now();
    loop {
        if start.elapsed() > timeout {
            anyhow::bail!("Flask 服务在 {} 秒内未响应", timeout.as_secs());
        }
        if check_server(port) {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}
