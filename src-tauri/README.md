# 片刻 · 桌面壳

本目录是 [Tauri 2](https://v2.tauri.app/) 桌面壳的源码，用 Rust 编写。

## 目录结构

```
src-tauri/
├── Cargo.toml              # Rust 依赖
├── tauri.conf.json         # 窗口/打包/安全配置
├── build.rs                # 构建脚本
├── capabilities/default.json  # Tauri 权限
├── frontend/index.html     # 安装向导 UI（首次启动时显示）
├── icons/                  # 应用图标（png/icns/ico）
└── src/
    ├── main.rs             # 入口：窗口、菜单、退出清理
    ├── commands.rs         # IPC 命令（init_setup / get_modes / start_setup）
    ├── env_check.rs        # Python / 磁盘 / 内存环境检测
    ├── launcher.rs         # 模式定义 + 安装状态管理
    ├── python_runtime.rs   # Python 环境管理（uv/venv/pip/Flask 进程）
    └── updater.rs          # GitHub 更新检查 + 自动更新
```

## 架构

```
Tauri 壳（Rust）
  ├─ 首次启动 → 前端安装向导 → 环境检测 → 模式选择
  │                    ↓
  │           自动下载 Python + uv + pip 包
  │                    ↓
  │           启动 Flask 子进程（app.py）
  │                    ↓
  │           导航到 http://localhost:5057
  │
  ├─ 正常使用 → 原生窗口内嵌 Flask Web 界面
  │
  └─ 退出时 → kill Flask 进程组 → 清理端口
```

## 构建

```bash
# 开发模式
cargo tauri dev

# 生产构建（产物在 target/release/bundle/ 下）
cargo tauri build

# 跨平台构建 — 使用 GitHub Actions
# 推送到 GitHub 后自动构建所有平台
```
