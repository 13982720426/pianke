//! 功能模式定义、安装状态管理。

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

/// Tauri 全局应用状态。
pub struct AppState {
    /// 资源目录（打包后的 Resources/ 或可执行文件同目录）
    pub resource_dir: PathBuf,
    /// 应用数据目录（~/Library/Application Support/com.pianke.desktop/）
    pub app_data_dir: PathBuf,
    /// 系统 Python 路径（find_python 找到的）
    pub python_path: PathBuf,
    /// venv 中的 Python 路径（空表示尚未创建）
    pub venv_python: Mutex<PathBuf>,
    /// 代码目录（resource_dir 解压到的位置）
    pub app_dir: PathBuf,
    /// 主页 URL（用于菜单"检查更新"后重新导航）
    pub home_url: String,
}

/// 每种模式的元信息。
#[derive(Serialize, Clone)]
pub struct ModeInfo {
    pub key: String,
    pub label: String,
    pub description: String,
    pub download_size: String,
    pub time_estimate: String,
    pub selected: bool,
}

/// 安装状态（持久化到 JSON 文件）。
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct InstallState {
    pub modes: Vec<String>,
    pub packages_sig: String,
    pub commit_sha: String,
}

/// 镜像配置。
pub struct MirrorConfig {
    pub use_mirror: bool,
    pub pypi_url: String,
    pub hf_url: String,
}

impl Default for MirrorConfig {
    fn default() -> Self {
        let no_mirror = std::env::var("PIANKE_NO_MIRROR").unwrap_or_default() == "1";
        MirrorConfig {
            use_mirror: !no_mirror,
            pypi_url: "https://pypi.tuna.tsinghua.edu.cn/simple/".into(),
            hf_url: "https://hf-mirror.com".into(),
        }
    }
}

/// 获取所有可用模式。
pub fn get_modes(previous: &[String]) -> Vec<ModeInfo> {
    let all = vec![
        ModeInfo {
            key: "fast".into(),
            label: "极速模式（Fast）".into(),
            description: "纯本地、极速、适合低配设备。零 AI 模型依赖，图像哈希 + 传统 CV 分组。"
                .into(),
            download_size: "约 200MB".into(),
            time_estimate: "1-3 分钟".into(),
            selected: previous.contains(&"fast".to_string()),
        },
        ModeInfo {
            key: "expert".into(),
            label: "专家模式（Expert）".into(),
            description: "本地 AI、识人更准。DINOv2 + 人脸识别 + EXIF 融合分组。".into(),
            download_size: "约 2-3GB".into(),
            time_estimate: "5-15 分钟".into(),
            selected: previous.contains(&"expert".to_string()),
        },
        ModeInfo {
            key: "tycoon".into(),
            label: "土豪模式（Tycoon）".into(),
            description: "大模型视觉判定、人话解说。需自备 API Key。".into(),
            download_size: "约 5MB".into(),
            time_estimate: "约 30 秒".into(),
            selected: previous.contains(&"tycoon".to_string()),
        },
    ];
    all
}

/// 安装状态文件路径。
pub fn state_file_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(".pic_selecter_install.json")
}

/// 从文件加载安装状态。
pub fn load_install_state(path: &Path) -> InstallState {
    if path.exists() {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        InstallState::default()
    }
}

/// 保存安装状态到文件。
pub fn save_install_state(path: &Path, state: &InstallState) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(state) {
        let _ = std::fs::write(path, json);
    }
}

/// 根据模式列表返回需要安装的 pip 包列表。
pub fn packages_for_modes(modes: &[String]) -> Vec<String> {
    let mut pkgs: Vec<String> = Vec::new();

    // Core packages (all modes)
    let core = vec![
        "Pillow>=10.0",
        "pillow-heif>=0.16",
        "numpy>=1.26",
        "scipy>=1.11",
        "flask>=3.0",
        "imagehash>=4.3",
        "opencv-contrib-python>=4.9",
        "rawpy>=0.18",
        "piexif>=1.1.3",
    ];
    for p in core {
        pkgs.push(p.to_string());
    }

    // Vision base (expert or tycoon)
    if modes.iter().any(|m| m == "expert" || m == "tycoon") {
        pkgs.push("transformers>=4.40".into());
        pkgs.push("insightface>=0.7".into());
    }

    // Expert additional
    if modes.contains(&"expert".to_string()) {
        pkgs.push("pyiqa>=0.1.10".into());
        pkgs.push("timm>=0.9".into());
    }

    // Tycoon additional
    if modes.contains(&"tycoon".to_string()) {
        pkgs.push("openai>=1.40".into());
    }

    pkgs
}
