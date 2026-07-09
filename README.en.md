<p align="center">
  <img src="pics/miyu-logo.png" alt="Miyu" width="180">
</p>

# Miyu

[中文](./README.md) | **English**

A terminal-native anime-style desktop assistant powered by LLMs. Miyu focuses on daily chat, light entertainment, and system troubleshooting—not on being a full coding agent.

## Who is she?

Miyu is inspired by the character [Miyu Kudou (久遠寺未有)](http://www.minatosoft.com/kimiaru/chara-miyu.html).

![](./pics/miyuwallpaper.png)

By default she uses the public model service from [opencode](https://github.com/anomalyco/opencode). You can also point her at any OpenAI-compatible, Anthropic Messages, or OpenAI Responses API. Optional seamless hooks for `fish`, `zsh`, `bash`, and PowerShell let you talk without leaving the shell.

```text
miyu config    # terminal TUI settings
miyu           # interactive REPL
```

![](./pics/shell-init.png)

![](./pics/tui.png)

## Features

| Area | Notes |
|------|--------|
| Interactive REPL | Yolo / Plan modes, streaming UI, slash commands, session resume, agent reuse |
| Config TUI | Providers, models, plugins, gateways |
| Shell integration | Command-not-found / hooks for frictionless chat |
| Multi-protocol LLM | OpenAI Chat, OpenAI Responses, Anthropic Messages |
| Tools & plugins | Files/commands, search, memes, knowledge base, memory, deep research, … |
| Messaging gateways | Optional QQ / WeChat-style channels |
| Web UI | Optional browser frontend |
| Sessions | SQLite turns, interrupt recovery, context compaction |

### Built-in plugins (highlights)

<details>
<summary>Show details</summary>

- **Memes**: situational auto-send; libraries are persona-scoped.  
  ![](./pics/nvidiafuckyou.png)
- **Divination / dice**: entertainment-oriented tools (I Ching, fortune, tarot, …).  
  ![](./pics/玄学.png)
- **Alarms**: timers and custom audio.  
  ![](./pics/set_alarm.png)
- **Knowledge base**: default Linux day-to-day notes; manage via `miyu kb` or chat.  
  ![](./pics/kb.png)
- **Linux game compatibility**: ProtonDB, anti-cheat trackers, community signals.  
  ![](./pics/gaming.png)
- **Web search / image search / image gen**: basic search works out of the box; optional Tavily/Firecrawl/etc.; image gen needs your own API.  
  ![](./pics/web-search-config.png)
- **Weather / FX / man / Arch helpers**: AUR status, Arch Wiki, PKGBUILD review, and more.
- **Memory**: facts + episodes with associative recall.  
  ![](./pics/记忆.png)
- **Deep research / diagnose**: multi-step workflows (token-heavy).  
  ![](./pics/深度研究.png)

</details>

### Optional HTTP debug

Log per-session LLM requests (headers, body, reconstructed stream) to disk:

```bash
export MIYU_DEBUG_HTTP=1
# optional: only one session
export MIYU_DEBUG_HTTP_SESSION='session_xxx'
miyu
```

Files land under `$XDG_CACHE_HOME/miyu/debug-http/<session_id>/...` (secrets in headers are redacted).

## Install

### Arch Linux

```bash
yay -S miyu
```

### Build from source

Requires **Rust 1.96+** (or current stable), a C toolchain, `pkg-config`, and ALSA development headers.

```bash
git clone https://github.com/SHORiN-KiWATA/Miyu.git
cd Miyu
cargo build --release --locked
./target/release/miyu --version
```

Example dependencies:

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

## Design goals

- **Desktop companion, not a coding agent**: realism, system integration, day-to-day fixes first.
- **Lightweight and approachable**: no heavy GUI/3D pet as the main path.
- **Natural language first**: few CLI flags; prefer shell hooks and chat.

Contribution ideas: troubleshooting, knowledge/retrieval, roleplay and fun features, deeper system integration, performance and bugfixes.  
PRs should explain intent and scenarios; **one feature per PR**.

## License

MIT License — see [LICENSE](./LICENSE).

---

[中文文档 →](./README.md)
