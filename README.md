# 片刻 (Pianke)

> **让 AI 协助初筛与分组，把最终的审美决定权留给自己。**

[![Python 3.10+](https://img.shields.io/badge/python-3.10+-blue.svg)](https://www.python.org/)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey.svg)](#-桌面应用一键启动)
[![Tauri](https://img.shields.io/badge/desktop-Tauri-ffc131?logo=tauri)](src-tauri/)
[![Build](https://img.shields.io/github/actions/workflow/status/zhaoyue4810/pianke/build.yml?branch=tauri-desktop&label=build)](https://github.com/zhaoyue4810/pianke/actions/workflows/build.yml)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

**片刻** 是一款专为摄影师和摄影爱好者设计的**本地照片双语/擂台式选片工具**。它能够将一次拍摄中相似的几十甚至上百张照片自动归入“同一个瞬间”的组中，然后通过直观的 **左右 A/B 擂台 PK** 方式，让你快速挑出最满意的一张。

---

## 核心特性

- 🔒 **纯本地与隐私保护**：除“土豪模式”外，所有照片的分析、初筛、分组均在本地完成，不上传任何云端，保护隐私。
- 🤖 **智能质量初筛**：自动识别并剔除模糊、过曝、欠曝、闭眼等技术性废片，并支持一键复核召回。
- 📁 **多信号相似分组**：融合 **DINOv2 视觉语义**、**人脸特征识别** 及 **EXIF 时间/GPS**，将视觉上“同一个瞬间”的连拍和相似帧精准归组。
- ⚔️ **双图擂台比拼**：通过 `←`/`→` 键盘快捷键在相似照片中进行两两对决，极速筛选。
- 📸 **RAW 格式原生支持**：
  - **RAW+JPG 双拍**：以 JPG 进行极速分析与呈现，文件操作时 RAW 与 JPG 自动配对（包括 `.xmp` 伴随文件）同步搬运或重命名。
  - **纯 RAW 拍摄**：毫秒级提取 RAW 内嵌预览图进行分析，不进行缓慢的 demosaic 解码。
- 🔄 **无损反悔与自动续做**：
  - **多级撤销**：支持单步撤销、整组重选及全局重做。
  - **进度保存**：状态实时持久化于照片目录，随时关闭，下次启动自动恢复进度。

---

## 工作模式对比

你可以根据硬件配置和照片类型，在网页首页自由切换以下三种模式：

| 模式 | 核心定位 | 废片初筛方式 | 分组逻辑 | 首次启动开销 | 联网要求 |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **极速 (Fast)** | 纯本地、极速、适合低配设备 | 本地轻量通用算法 | 本地图像比对 (感知哈希等) | 约 200MB 依赖 | **完全不需要** |
| **专家 (Expert)** | 本地 AI、识人更准 | 本地 AI 模型（人脸、技术美学质量） | 本地 AI 模型 (DINOv2) + 人脸识别 + EXIF 融合 | 约 2-3GB 依赖 / 首次需下载 600MB 模型 | 仅首次下载模型 |
| **土豪 (Tycoon)** | 大模型视觉判定、人话解说 | 远程多模态大模型 (火山/GPT-4V/Qwen等) | 与专家模式相同 | 约 5MB 依赖 / 配置 API Key | 每张图调用 LLM 接口 |

> 💡 **选择建议**：
> - **拍物/风景/低配电脑** ➔ 推荐 **极速 (Fast)** 模式，对多角度、静物及产品图有非常稳定的分组表现。
> - **拍人像/婚礼/活动/主流设备** ➔ 推荐 **专家 (Expert)** 模式，能够精准识别“同人不同姿势不同背景”并归入同组。
> - **需要详细退片理由** ➔ 推荐 **土豪 (Tycoon)** 模式，大模型将以“人话”解释退片原因（如“左侧小孩闭眼了”）。

---

## 快速开始

### 🖥️ 桌面应用一键启动（推荐）

适合未安装 Python 环境或不熟悉命令行的用户。下载对应系统的安装包，双击安装即可使用。

| 系统 | 下载安装 | 首次启动说明 |
| :--- | :--- | :--- |
| **macOS** | `片刻.dmg` → 拖到 Applications → 双击打开 | 首次启动会弹出安装向导，选择模式后自动下载 Python 依赖 |
| **Windows** | `片刻.exe` → 双击安装 | 同上 |
| **Linux** | `片刻.AppImage` → 双击运行 | 同上 |

> **零依赖运行**：安装包约 5MB（不含 Python）。首次启动时自动通过 [uv](https://docs.astral.sh/uv/) 下载所需的 Python 环境和 pip 包，全程无需手动装任何东西。

### 🐍 开发者手动启动

如果你已安装 Python 环境并希望从源码运行：

```bash
# 1. 创建并激活虚拟环境
python3 -m venv .venv
source .venv/bin/activate  # Windows: .venv\Scripts\activate

# 2. 安装项目依赖
pip install -r requirements.txt

# 3. 运行服务
python app.py

# 常用参数：
python app.py --port 8080 --no-browser
```

> ⚠️ **OpenCV 冲突修复**：若手动安装后启动报错，请运行：
> ```bash
> pip uninstall -y opencv-python opencv-python-headless
> pip install --force-reinstall --no-deps "opencv-contrib-python>=4.9"
> ```

### 🔧 自行构建桌面安装包

需要安装 Rust 和 Node.js，然后：

```bash
# 安装 Rust（已有可跳过）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 安装 Tauri CLI
cargo install tauri-cli --version "^2"

# 构建（产物在 src-tauri/target/release/bundle/ 下）
cd src-tauri
cargo tauri build

# 一次性构建全部平台（需要对应平台的工具链）
# 或在 GitHub Actions 中自动构建 — 见 .github/workflows/build.yml
```

> **跨平台 CI 构建**：仓库已配置 GitHub Actions 流水线，推送到 GitHub 后自动构建 macOS / Windows / Linux 三平台安装包。前往 Actions 页面手动触发「构建」workflow 即可获取。

---

## 桌面应用架构

```
┌──────────────────────────────────────────┐
│  Tauri (Rust) — 桌面壳                    │
│  • macOS .app / Windows .exe / Linux     │
│  • 原生窗口、菜单栏、Dock 图标             │
│  • 首次启动安装向导（环境检测+模式选择）    │
│  • 自动下载 Python + pip 依赖             │
│  • 管理 Flask 子进程生命周期               │
│  • 退出时自动清理后台进程                  │
├──────────────────────────────────────────┤
│  Python (Flask) — 照片选片引擎            │
│  • 文件扫描、质量初筛、分组                │
│  • DINOv2 + 人脸识别 + 美学评分           │
│  • 擂台 PK、水印渲染、文件搬运             │
│  • RAW / HEIC 解码                       │
├──────────────────────────────────────────┤
│  Web UI (HTML/CSS/JS) — 用户界面          │
│  • 首页配置、处理进度、初筛复核            │
│  • 分组预览、A/B 擂台 PK                  │
│  • Winner 总览、相机水印导出              │
└──────────────────────────────────────────┘
```

---

## 选片工作流

1. **导入与配置**：在首页选择工作模式，输入或拖入照片文件夹的绝对路径，点击 **开始整理**。
2. **分析与进度**：系统流式显示扫描进度和预估剩余时间。
3. **初筛复核**：平铺展示被自动淘汰的废片及原因。**点击任意照片即可直接将其放回待分组列表**。未点击的将直接归入淘汰区。
4. **分组预览与调参**：预览分组效果。如果分组过细或过粗，可实时拖动滑块调节阈值并点击 **重新分组** 即时更新。
5. **左右擂台 PK**：在相似组内进行两两对比。
   - `←` / `→`：保留左侧 / 保留右侧 (对侧淘汰)
   - `↑` / `↓`：全要 / 全不要
   - `[` / `]`：仅淘汰左侧 / 仅淘汰右侧 (处理单张图像损坏)
   - `S`：跳过本组 (暂存至队尾最后处理)
   - `Z`：切换缩放等级 (1x ➔ 2x ➔ 4x)，两图同步平移
   - `Shift + Z`：撤销当前组的上一步操作
6. **完成与导出**：展示所有 Winner 照片，支持一键打开结果文件夹。

---

## 输出与目录策略

本工具采用**单文件夹、单会话**策略，会在你指定的**照片目录**下生成以下结构：

```text
📂 你的照片文件夹/
├── 📂 winners/                # 最终胜出保留的照片 (你的成片)
├── 📂 losers/                 # 被淘汰的照片
├── 📄 .pic_selecter_state.json # 进度持久化文件 (可随时中断并继续)
└── 📂 _pic_selecter/          # 缓存与日志目录
    ├── 📂 thumbs/             # 缩略图缓存 (重复加载近乎瞬时)
    ├── 📄 log.txt             # 详细处理日志
    └── 📄 skipped.log         # 坏图/无法读取文件清单
```

### 归档模式

你可以在首页的“更多选项”中选择以下归档模式：
- **移动模式（默认，推荐）**：将原片直接移动到 `winners/` 或 `losers/` 目录。最省磁盘空间，符合一次性选片直觉。反悔时文件会无损搬回原位。
- **复制模式**：原片保持不动，在 `winners/` 和 `losers/` 中创建副本。需要双倍磁盘空间。

---

## 相机水印

选片完成后，在 Winner 总览页点击 **相机水印** 可以为胜出的照片批量添加 EXIF 信息水印。
水印会读取照片的相机品牌、机型、镜头、焦距、光圈、快门、ISO、拍摄时间并合成在画面上，
也会自动匹配品牌 Logo（富士、佳能、尼康、索尼、徕卡、哈苏、奥林巴斯等）。

共 11 个样式 ID。详尽程度直接编码在样式名里（**-详尽** 带镜头·参数·时间；
**-极简** 仅品牌 Logo + 机型）：

| ID | 名称 | 风格 |
| :--- | :--- | :--- |
| **A** | 标准底栏 | 白色信息条，左机型/镜头 ｜ 中品牌 Logo ｜ 右参数/时间 |
| **B_full / B_clean** | 极简底栏-详尽 / 极简底栏-极简 | 顶/左/右贴边窄白 + 底部居中 Logo + 信息 |
| **C_full / C_clean** | 毛玻璃悬浮-详尽 / 毛玻璃悬浮-极简 | 原图模糊作背景 + 缩小照片悬浮居中带阴影 |
| **D_full / D_clean** | 经典白边相框-详尽 / 经典白边相框-极简 | 顶/左/右窄白 + 底大白边居中放品牌 |
| **F_full / F_clean** | 杂志风-详尽 / 杂志风-极简 | 左下色卡（从图自动提取）+ 右下品牌信息 |
| **G** | 极简白边 | 照片四周均匀窄白边，无任何文字 |
| **H** | 相机回放 | 上原图 + 下模糊版 + 居中富士 X-T5，LCD 与取景器都显示画面 |

输出到 `winners/watermarked_<时间戳>/` 子目录，保留原 EXIF（旋转标志归位），JPEG 92 质量、progressive。

---

## 常见问题 (FAQ)

<details>
<summary><b>1. 启动后浏览器没有自动打开？</b></summary>
手动在浏览器中访问 <code>http://localhost:5057</code> 即可。
</details>

<details>
<summary><b>2. 默认的 5057 端口被占用怎么办？</b></summary>
在启动前设置环境变量改变端口：
<ul>
  <li>macOS: <code>export PIC_SELECTER_PORT=8080</code></li>
  <li>Windows: <code>set PIC_SELECTER_PORT=8080</code></li>
</ul>
后再运行启动器。
</details>

<details>
<summary><b>3. 专家模式首次启动卡在“加载模型”？</b></summary>
由于首次运行需要下载约 600MB 的 AI 模型文件，视网络情况可能需要 1-3 分钟。若长时间无响应，请检查终端日志或网络连接。
</details>

<details>
<summary><b>4. 土豪 (Tycoon) 模式如何配置 API Key？</b></summary>
本模式支持兼容 OpenAI 协议的视觉大模型（如火山引擎豆包等）。
启动前在系统环境变量中设置 <code>ARK_API_KEY=你的key</code>（或在网页端弹窗输入，Key 仅安全保存在本地 <code>~/.config/pic_selecter/ark_key</code>）。
</details>

<details>
<summary><b>5. 极速模式与专家模式在质量初筛上有什么区别？</b></summary>
两者的质量评估维度不同：
<ul>
  <li><b>极速模式</b>：侧重检测“整张图片偏软/手抖/失焦”等全局性模糊。</li>
  <li><b>专家模式</b>：侧重检测人像中的“背景清晰但人脸模糊/闭眼”等局部性人像废片。</li>
</ul>
</details>

<details>
<summary><b>6. 如何彻底卸载和清理缓存？</b></summary>
<ul>
  <li><b>清理照片缓存</b>：直接删除对应照片目录下的 <code>winners/</code>、<code>losers/</code>、<code>.pic_selecter_state.json</code> 及 <code>_pic_selecter/</code> 文件夹（移动模式下请先移回照片）。</li>
  <li><b>完全卸载程序</b>：删除解压出的项目文件夹，并清理本地全局工具缓存：
    <ul>
      <li>macOS: <code>rm -rf ~/.local/bin/uv ~/.local/share/uv/</code></li>
      <li>Windows: 删除 <code>%USERPROFILE%\.local\bin\uv.exe</code> 和 <code>%USERPROFILE%\.local\share\uv\</code></li>
    </ul>
  </li>
</ul>
</details>

<details>
<summary><b>7. HEIC 格式图片在网页上无法预览？</b></summary>
Windows 系统可能需要安装微软官方的 HEIF 扩展才能正常在浏览器中预览该格式；片刻工具在后端能够正常分析处理该格式。
</details>

---

## 隐私与安全说明

1. 本地模式下，所有照片数据、模型特征计算**完全在本地运行**，无任何外发流量。
2. Web 服务器仅绑定本地 `127.0.0.1` 环回地址，局域网及外网设备均无法访问。
3. 接口提供严格的 Origin 和 Referer 校验，防止跨站请求伪造 (CSRF/DNS Rebinding)。如果需要对脚本访问开启 token 鉴权，可在运行前设置环境变量 `PIC_SELECTER_TOKEN=your_token`。
4. ⚠️ **土豪模式例外**：该模式下，照片的缩略图会被发送至你配置的大模型服务商接口。如果对数据隐私有极高要求，请仅使用**极速**或**专家**模式。

---

## 反馈与贡献

欢迎通过 [GitHub Issues](https://github.com/zhaoyue4810/pianke/issues) 提交反馈或建议。

# 个人微信
15828377122
