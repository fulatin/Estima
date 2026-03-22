# Estima

AI 控制的实时音频效果处理器

Estima 是一款专业级音频效果处理器，将 LV2 插件与 AI 自然语言控制相结合。基于 JACK 音频实现低延迟实时处理。

## 功能特性

- **实时音频处理**：通过 JACK 实现低延迟音频处理
- **LV2 插件支持**：加载并串联系统上安装的任何 LV2 音频插件
- **AI 控制**：使用自然语言控制效果（如"添加一些混响"、"增加失真度"）
- **多 AI 提供商支持**：支持 SiliconFlow、DeepSeek、OpenAI、Anthropic 和 Ollama
- **预设系统**：保存和加载效果链预设
- **旁路模式**：快速切换所有效果开/关
- **多种界面**：CLI 命令行、Tauri GUI (Vue 3) 和 egui GUI

## 系统要求

- Linux（需要 JACK 音频支持）
- [JACK Audio Connection Kit](https://jackaudio.org/)
- LV2 插件（通过系统包管理器安装）

### 安装依赖

**Ubuntu/Debian:**
```bash
sudo apt install jackd2 lv2-dev
# 安装一些 LV2 插件
sudo apt install calf-plugins guitarix-lv2
```

**Arch Linux:**
```bash
sudo pacman -S jack2 lv2
sudo pacman -S calf guitarix
```

**Fedora:**
```bash
sudo dnf install jack-audio-connection-kit lv2
sudo dnf install calf-plugins guitarix
```

## 编译

```bash
# 克隆仓库
git clone https://github.com/your-repo/estima.git
cd estima

# 编译所有目标
cargo build --release

# 或编译特定目标
cargo build --release -p estima-cli      # 仅 CLI
cargo build --release -p estima-gui      # Tauri GUI
cargo build --release -p estima-gui-egui # egui GUI
```

### 前端编译（Tauri GUI）

```bash
cd crates/gui/src-ui
npm install
npm run build
cd ../../..
cargo build --release -p estima-gui
```

## 使用方法

### 启动 JACK

运行 Estima 前请确保 JACK 已启动：

```bash
# 启动 JACK（根据需要调整参数）
jackd -d alsa -r 48000 -p 256 &
```

### 命令行界面

```bash
./target/release/estima-cli
```

**CLI 命令：**
```
/list [filter]           列出可用插件
/load <uri>              通过 URI 加载插件
/remove <id|@last>       移除插件
/param <id|@last> <参数名> <值>  设置参数
/params [uri]            显示插件参数
/status                  显示当前插件链
/clear                   清除所有插件
/bypass                  切换旁路开/关
/save [name]             保存当前配置为预设
/open [name]             加载预设（不指定名称则列出）
/presets                 列出可用预设
/help                    显示帮助
/quit                    退出
```

**自然语言示例：**
```
"添加一个混响效果"
"增加一些失真"
"旁路所有效果"
"移除所有效果"
"把延迟调长一点"
```

### 图形界面

**Tauri GUI（推荐）：**
```bash
./run-gui.sh
# 或
WEBKIT_DISABLE_COMPOSITING_MODE=1 ./target/release/estima-gui
```

**egui GUI（轻量级备选）：**
```bash
./run-gui-egui.sh
# 或
./target/release/estima-gui-egui
```

## AI 配置

Estima 会自动从环境变量检测 AI 提供商。设置以下之一：

| 提供商 | 环境变量 | 模型变量 |
|-------|---------|---------|
| SiliconFlow | `SILICONFLOW_API_KEY` | `SILICONFLOW_MODEL` |
| DeepSeek | `DEEPSEEK_API_KEY` | `DEEPSEEK_MODEL` |
| OpenAI | `OPENAI_API_KEY` | `OPENAI_MODEL` |
| Anthropic | `ANTHROPIC_API_KEY` | `ANTHROPIC_MODEL` |
| 自定义 | `AI_API_KEY` + `AI_BASE_URL` | `AI_MODEL` |
| Ollama | （本地，无需密钥） | `OLLAMA_MODEL` |

**示例：**
```bash
export SILICONFLOW_API_KEY="your-api-key"
export SILICONFLOW_MODEL="Qwen/Qwen2.5-7B-Instruct"
./target/release/estima-cli
```

## 项目架构

```
estima/
├── crates/
│   ├── core/           # 核心库
│   │   └── src/
│   │       ├── audio/
│   │       │   ├── jack_engine.rs    # JACK 音频引擎
│   │       │   └── plugin_chain.rs   # LV2 插件管理
│   │       ├── ai/                   # AI 提供商
│   │       │   ├── mod.rs
│   │       │   ├── openai.rs         # 多提供商支持
│   │       │   └── ollama.rs
│   │       └── control/
│   │           └── interpreter.rs    # 命令解析
│   ├── cli/            # CLI 应用
│   ├── gui/            # Tauri + Vue 3 GUI
│   └── gui-egui/       # egui GUI（备选）
```

## 预设管理

预设以 JSON 文件格式保存，扩展名为 `.estima.json`：

```bash
# 保存当前链
/save my_reverb_chain

# 加载预设
/open my_reverb_chain

# 列出预设
/presets
```

## 常见问题

### "Failed to create JACK client"

确保 JACK 正在运行：
```bash
jack_wait -c  # 应输出 "running"
```

### 找不到插件

安装 LV2 插件：
```bash
# Ubuntu/Debian
sudo apt install calf-plugins guitarix-lv2 swh-plugins

# 查看已安装插件
lv2ls
```

### Tauri GUI 在 Wayland 下显示异常

使用提供的脚本或设置环境变量：
```bash
WEBKIT_DISABLE_COMPOSITING_MODE=1 ./target/release/estima-gui
```

或使用 egui 备选版本：
```bash
./target/release/estima-gui-egui
```

## 许可证

MIT

## 贡献

欢迎贡献！请随时提交问题和拉取请求。
