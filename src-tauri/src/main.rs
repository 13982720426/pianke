//! 片刻 (Pianke) — Tauri 桌面应用的入口点。
//!
//! 架构说明：
//! - setup 阶段只做路径解析和状态初始化，不发送事件（前端监听器尚未注册）
//! - 所有环境检测、更新检查等耗时操作由前端主动调用 init_setup 命令触发
//! - Flask 进程的生命周期由 FlaskProcess 管理，应用退出时通过 RunEvent::Exit 自动 kill

mod commands;
mod env_check;
mod launcher;
mod python_runtime;
mod updater;

use std::sync::Mutex;

use tauri::Manager;
use tauri::menu::{Menu, MenuItem, Submenu, PredefinedMenuItem};

fn main() {
    env_logger::init();

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(commands::FlaskProcess(Mutex::new(None)))
        .setup(|app| {
            let resource_dir = app
                .path()
                .resource_dir()
                .map_err(|e| anyhow::anyhow!("无法获取资源目录: {}", e))?;

            let app_data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| anyhow::anyhow!("无法获取数据目录: {}", e))?;

            let python_path = env_check::find_python()
                .map(|(_, path)| std::path::PathBuf::from(path));

            // 提取资源文件到数据目录
            let app_dir = python_runtime::PythonRuntime::extract_resources(
                &resource_dir,
                &app_data_dir,
            ).unwrap_or_else(|e| {
                log::error!("Resource extraction failed: {}", e);
                // 开发模式：使用项目根目录
                std::env::current_dir()
                    .unwrap_or_else(|_| app_data_dir.join("app"))
            });

            // 尝试快速创建 venv（不阻塞 UI）
            let venv_python = python_runtime::PythonRuntime::setup_venv(
                python_path.as_deref(),
                &app_data_dir,
                false,
                |_| {},
            ).unwrap_or_else(|e| {
                log::warn!("Fast venv creation failed: {} (will retry in init_setup)", e);
                std::path::PathBuf::new()
            });

            let home_url = app.get_webview_window("main")
                .and_then(|w| w.url().ok())
                .map(|u| u.to_string())
                .unwrap_or_default();

            let state = launcher::AppState {
                resource_dir,
                app_data_dir,
                python_path: python_path.unwrap_or_default(),
                venv_python: Mutex::new(venv_python),
                app_dir,
                home_url,
            };
            app.manage(state);

            // 设置菜单
            let menu = Menu::with_items(app, &[
                &Submenu::with_items(app, "片刻", true, &[
                    &PredefinedMenuItem::about(app, Some("关于片刻"), None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &MenuItem::with_id(app, "check_update", "检查更新", true, None::<&str>)?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::services(app, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::hide(app, Some("隐藏片刻"))?,
                    &PredefinedMenuItem::hide_others(app, Some("隐藏其他"))?,
                    &PredefinedMenuItem::show_all(app, Some("显示全部"))?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::quit(app, Some("退出片刻"))?,
                ])?,
            ])?;
            app.set_menu(menu)?;

            Ok(())
        })
        .on_menu_event(|app_handle, event| {
            if event.id().as_ref() == "check_update" {
                // 1. 停掉 Flask
                if let Some(flask_state) = app_handle.try_state::<commands::FlaskProcess>() {
                    if let Some(mut child) = flask_state.0.lock().unwrap().take() {
                        log::info!("Stopping Flask for update check...");
                        kill_process_group(child.id());
                        let _ = child.kill();
                        let _ = child.wait();
                    }
                }
                // 清理端口
                kill_port_process(5057);

                // 2. 导航回 Tauri 前端重新 init
                if let Some(w) = app_handle.get_webview_window("main") {
                    if let Some(app_state) = app_handle.try_state::<launcher::AppState>() {
                        if !app_state.home_url.is_empty() {
                            if let Ok(url) = url::Url::parse(&app_state.home_url) {
                                let _ = w.navigate(url);
                            }
                        }
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::init_setup,
            commands::get_modes,
            commands::start_setup,
        ])
        .build(tauri::generate_context!())
        .expect("启动失败");

    app.run(|app_handle, event| {
        if let tauri::RunEvent::Exit = event {
            // 清理 Flask 子进程
            if let Some(state) = app_handle.try_state::<commands::FlaskProcess>() {
                if let Some(mut child) = state.0.lock().unwrap().take() {
                    log::info!("Shutting down Flask...");
                    kill_process_group(child.id());
                    let _ = child.kill();
                    let _ = child.wait();
                }
            }

            // 端口级兜底清理
            let port = std::env::var("PIC_SELECTER_PORT")
                .ok()
                .and_then(|s| s.parse::<u16>().ok())
                .unwrap_or(5057);
            kill_port_process(port);
        }
    });
}

#[cfg(unix)]
fn kill_process_group(pid: u32) {
    let pid = pid as i32;
    let _ = std::process::Command::new("kill")
        .args(["-TERM", &format!("-{}", pid)])
        .status();
    std::thread::sleep(std::time::Duration::from_millis(500));
    let _ = std::process::Command::new("kill")
        .args(["-KILL", &format!("-{}", pid)])
        .status();
}

#[cfg(windows)]
fn kill_process_group(pid: u32) {
    let _ = std::process::Command::new("taskkill")
        .args(["/T", "/F", "/PID", &pid.to_string()])
        .status();
}

#[cfg(unix)]
fn kill_port_process(port: u16) {
    let _ = std::process::Command::new("lsof")
        .args(["-ti", &format!(":{}", port)])
        .output()
        .ok()
        .and_then(|o| {
            let pids = String::from_utf8_lossy(&o.stdout);
            for pid_str in pids.lines() {
                if let Ok(pid) = pid_str.trim().parse::<i32>() {
                    let _ = std::process::Command::new("kill")
                        .args(["-TERM", &format!("{}", pid)])
                        .status();
                }
            }
            Some(())
        });
}

#[cfg(windows)]
fn kill_port_process(port: u16) {
    if let Ok(output) = std::process::Command::new("netstat").args(["-ano"]).output() {
        let out = String::from_utf8_lossy(&output.stdout);
        for line in out.lines() {
            if line.contains(&format!(":{}", port)) && line.contains("LISTENING") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(pid_str) = parts.last() {
                    let _ = std::process::Command::new("taskkill")
                        .args(["/F", "/PID", pid_str])
                        .status();
                }
            }
        }
    }
}
