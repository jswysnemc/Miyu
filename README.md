<p align="center">
  <img src="pics/miyu-logo.png" alt="Miyu" width="180">
</p>

# Miyu

**中文** | [English](./README.en.md)

<p align="center">
  <img alt="release" src="https://img.shields.io/badge/release-v0.1.4-24292f?style=flat-square" />
  <img alt="license" src="https://img.shields.io/badge/license-MIT-2ea44f?style=flat-square" />
  <img alt="language" src="https://img.shields.io/badge/language-Rust-dea584?style=flat-square&logo=rust&logoColor=white" />
  <img alt="edition" src="https://img.shields.io/badge/edition-2021-f74c00?style=flat-square" />
  <img alt="platform" src="https://img.shields.io/badge/platform-Linux%20%7C%20Windows-0078d4?style=flat-square" />
  <img alt="ui" src="https://img.shields.io/badge/ui-Terminal%20TUI%20%7C%20Web-6f42c1?style=flat-square" />
</p>
<p align="center">
  <img alt="features" src="https://img.shields.io/badge/features-REPL%20%7C%20Shell%20%7C%20Plugins%20%7C%20Gateways%20%7C%20Memory-e36209?style=flat-square" />
  <img alt="llm" src="https://img.shields.io/badge/LLM-OpenAI%20%7C%20Anthropic%20%7C%20Responses-0969da?style=flat-square" />
</p>

终端里的二次元桌面助手：由大模型驱动，偏日常聊天、娱乐与系统排障，而不是专业 Coding Agent。

## 她是谁

Miyu 的人物设定灵感来自动画角色 [久遠寺未有](http://www.minatosoft.com/kimiaru/chara-miyu.html)。

![](./pics/miyuwallpaper.png)

默认接入 [opencode](https://github.com/anomalyco/opencode) 公共模型服务，也可配置任意 OpenAI 兼容 / Anthropic / Responses 协议的自建或第三方 API。支持与 `fish`、`zsh`、`bash`（及 PowerShell）无缝集成：在 shell 里直接说话即可对话。

```text
miyu config    # 终端 TUI 配置
miyu           # 交互 REPL
```

![](./pics/shell-init.png)

![](./pics/tui.png)

## 功能概览

| 能力 | 说明 |
|------|------|
| 交互 REPL | 双模式（Yolo / Plan）、流式渲染、斜杠命令、会话 resume、Agent 复用 |
| 配置 TUI | 供应商、模型、插件、网关等可视化编辑 |
| Shell 集成 | Command-not-found / 钩子触发无缝对话 |
| 多协议 LLM | OpenAI Chat、OpenAI Responses、Anthropic Messages |
| 工具与插件 | 文件/命令、搜索、表情包、知识库、记忆、深度研究等 |
| 消息网关 | QQ / 微信等渠道（可选） |
| Web 界面 | 可选浏览器前端 |
| 会话与恢复 | SQLite 轮次、中断恢复、上下文压缩 |

### 内置插件（节选）

<details>
<summary>展开介绍</summary>

- **表情包**：按情景自动或手动发送；库随人格隔离。  
  ![](./pics/nvidiafuckyou.png)
- **玄学 / 骰子**：周易、吉凶、塔罗、比大小等娱乐向工具。  
  ![](./pics/玄学.png)
- **闹钟**：番茄钟、计时；可自定义音频。  
  ![](./pics/set_alarm.png)
- **知识库**：默认含日用 Linux 资料；`miyu kb` 或自然语言管理。  
  ![](./pics/kb.png)
- **Linux 游戏兼容性**：ProtonDB、Anti-Cheat、Can I Play On Linux 等综合判断。  
  ![](./pics/gaming.png)
- **网络搜索 / 搜图 / 生图**：基础搜索开箱可用；可选 Tavily、Firecrawl 等；生图需自行配置。  
  ![](./pics/web-search-config.png)
- **天气 / 汇率 / man / Arch 工具链**：日常查询与 AUR、Wiki、PKGBUILD 审查等。  
- **记忆**：事实 + 往事，对话中联想召回。  
  ![](./pics/记忆.png)
- **深度研究 / 深度诊断**：重量级多步研究与排障（消耗较多 token）。  
  ![](./pics/深度研究.png)

</details>

### 调试（可选）

用环境变量将会话的 HTTP 请求落盘（含流式重组响应）：

```bash
export MIYU_DEBUG_HTTP=1
# 可选：只记录指定会话
export MIYU_DEBUG_HTTP_SESSION='session_xxx'
miyu
```

输出目录：`$XDG_CACHE_HOME/miyu/debug-http/<session_id>/...`（请求头已脱敏）。

## 安装

### Arch Linux

```bash
yay -S miyu
```

### Windows 10/11

需要 Rust MSVC 工具链、Visual Studio Build Tools 的“使用 C++ 的桌面开发”组件，以及 Node.js 22+。在 PowerShell 中执行：

```powershell
git clone https://github.com/SHORiN-KiWATA/Miyu.git
Set-Location Miyu\web
npm ci
npm run build
Set-Location ..
cargo build --release --locked
.\target\release\miyu.exe --version
.\target\release\miyu.exe init
```

Windows 支持 PowerShell 命令未找到拦截、CLI、Web 工作台、Web 终端、剪贴板、音频闹钟和命令执行。审计模式的 Shell 沙盒依赖 Linux bubblewrap，在 Windows 上不启用；`check_issue` 系统诊断目前只支持 Linux 和 macOS。文件搜索工具需要单独安装 `rg`，工作区 Git 功能需要 `git`。

### Linux 从源码构建

需要 **Rust 1.96+**（或当前稳定版）、Node.js 22+、C 工具链、`pkg-config`、ALSA 开发库。

```bash
git clone https://github.com/SHORiN-KiWATA/Miyu.git
cd Miyu
cd web
npm ci
npm run build
cd ..
cargo build --release --locked
./target/release/miyu --version
```

依赖示例：

```bash
# Arch
sudo pacman -S --needed rust cargo pkgconf alsa-lib gcc

# Fedora
sudo dnf install cargo rust rust-std-static pkgconf-pkg-config alsa-lib-devel gcc

# Ubuntu 24.04
sudo apt install curl build-essential pkg-config libasound2-dev ca-certificates
curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable
. "$HOME/.cargo/env"
```

## 设计理念

- **桌面助手，不是 Coding Agent**：拟真、系统集成、日用排障优先。
- **开箱即用、保持轻量**：不以重量级 GUI / 3D 桌宠为主路径。
- **自然语言优先**：尽量少记 CLI 子命令，shell 无缝触发。

欢迎贡献方向：系统排障、知识与检索、角色扮演与趣味、shell/守护进程集成、性能与缺陷修复。  
PR 请说明设计意图与使用场景；**一个 PR 对应一个功能**。

## 许可

MIT License，见 [LICENSE](./LICENSE)。

---

[English documentation →](./README.en.md)
