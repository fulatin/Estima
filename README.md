# Estima

AI-Controlled Real-Time Audio Effects Processor

Estima is a professional audio effects processor that combines LV2 plugins with AI-powered natural language control. Built on JACK audio for low-latency real-time processing.

## Features

- **Real-Time Audio Processing**: Low-latency audio processing via JACK
- **LV2 Plugin Support**: Load and chain any LV2 audio plugins installed on your system
- **AI Control**: Control effects using natural language (e.g., "Add some reverb", "Make it more distorted")
- **Multiple AI Providers**: Support for SiliconFlow, DeepSeek, OpenAI, Anthropic, and Ollama
- **Preset System**: Save and load effect chain presets
- **Bypass Mode**: Quickly toggle all effects on/off
- **Multiple Interfaces**: CLI, Tauri GUI (Vue 3)

## Requirements

- Linux (JACK audio support required)
- [JACK Audio Connection Kit](https://jackaudio.org/)
- LV2 plugins (install via your distribution's package manager)

### Installing Dependencies

**Ubuntu/Debian:**
```bash
sudo apt install jackd2 lv2-dev
# Install some LV2 plugins
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

## Building

```bash
# Clone the repository
git clone https://github.com/your-repo/estima.git
cd estima

# Build all targets
cargo build --release

# Or build specific targets
cargo build --release -p estima-cli      # CLI only
cargo build --release -p estima-gui      # Tauri GUI
```

### Frontend Build (Tauri GUI)

```bash
cd crates/gui/src-ui
npm install
npm run build
cd ../../..
cargo build --release -p estima-gui
```

## Usage

### Start JACK

Before running Estima, ensure JACK is running:

```bash
# Start JACK (adjust parameters as needed)
jackd -d alsa -r 48000 -p 256 &
```

### CLI Interface

```bash
./target/release/estima-cli
```

**CLI Commands:**
```
/list [filter]           List available plugins
/load <uri>              Load a plugin by URI
/remove <id|@last>       Remove a plugin
/param <id|@last> <name> <value>  Set parameter
/params [uri]            Show parameters for plugin
/status                  Show current plugin chain
/clear                   Clear all plugins
/bypass                  Toggle bypass on/off
/save [name]             Save current config as preset
/open [name]             Load a preset (list if no name)
/presets                 List available presets
/help                    Show help
/quit                    Exit
```

**Natural Language Examples:**
```
"Add a reverb effect"
"Give me more distortion"
"Bypass the effects"
"Remove all effects"
"Make the delay longer"
```

### GUI Interface

**Tauri GUI (recommended):**
```bash
./run-gui.sh
# or
WEBKIT_DISABLE_COMPOSITING_MODE=1 ./target/release/estima-gui
```

## AI Configuration

Estima auto-detects AI providers from environment variables. Set one of the following:

| Provider | Environment Variable | Model Variable |
|---------|---------------------|----------------|
| SiliconFlow | `SILICONFLOW_API_KEY` | `SILICONFLOW_MODEL` |
| DeepSeek | `DEEPSEEK_API_KEY` | `DEEPSEEK_MODEL` |
| OpenAI | `OPENAI_API_KEY` | `OPENAI_MODEL` |
| Anthropic | `ANTHROPIC_API_KEY` | `ANTHROPIC_MODEL` |
| Custom | `AI_API_KEY` + `AI_BASE_URL` | `AI_MODEL` |
| Ollama | (local, no key) | `OLLAMA_MODEL` |

**Example:**
```bash
export SILICONFLOW_API_KEY="your-api-key"
export SILICONFLOW_MODEL="Qwen/Qwen2.5-7B-Instruct"
./target/release/estima-cli
```

## Architecture

```
estima/
├── crates/
│   ├── core/           # Core library
│   │   └── src/
│   │       ├── audio/
│   │       │   ├── jack_engine.rs    # JACK audio engine
│   │       │   └── plugin_chain.rs   # LV2 plugin management
│   │       ├── ai/                   # AI providers
│   │       │   ├── mod.rs
│   │       │   ├── openai.rs         # Multi-provider support
│   │       │   └── ollama.rs
│   │       └── control/
│   │           └── interpreter.rs    # Command parsing
│   ├── cli/            # CLI application
│   ├── gui/            # Tauri + Vue 3 GUI
```

## Presets

Presets are saved as JSON files with `.estima.json` extension:

```bash
# Save current chain
/save my_reverb_chain

# Load a preset
/open my_reverb_chain

# List presets
/presets
```

## Troubleshooting

### "Failed to create JACK client"

Ensure JACK is running:
```bash
jack_wait -c  # Should print "running"
```

### No plugins found

Install LV2 plugins:
```bash
# Ubuntu/Debian
sudo apt install calf-plugins guitarix-lv2 swh-plugins

# Check installed plugins
lv2ls
```

### Tauri GUI display issues on Wayland

Use the provided script or set environment variables:
```bash
WEBKIT_DISABLE_COMPOSITING_MODE=1 ./target/release/estima-gui
```

Or use the egui fallback:
```bash
./target/release/estima-gui-egui
```

## License

MIT

## Contributing

Contributions are welcome! Please feel free to submit issues and pull requests.
