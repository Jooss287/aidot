# aidot

> A CLI that manages and syncs AI coding tool configurations across Claude Code, Cursor, GitHub Copilot, Codex CLI, and more — all from a single preset.

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

[한국어](README.md)

---

## Introduction

**aidot** (AI dotfiles) is a CLI tool that unifies configuration management across multiple LLM coding tools (Claude Code, Cursor, GitHub Copilot, Codex CLI, etc.) into a single preset.

Manage your settings in one preset repository and apply them to all detected LLM tools with a single `aidot pull`.

### Key Features

- **Tool-agnostic configuration** - Unified presets independent of language or IDE
- **Auto-detection & conversion** - Automatically detects installed LLM tools and converts to each tool's format
- **Git-based preset sharing** - Version control team/personal settings via Git repositories
- **Environment sync** - Maintain identical settings across multiple machines, useful for team sharing

---

## Quick Start

```bash
# Install (macOS/Linux)
curl -fsSL https://raw.githubusercontent.com/Jooss287/aidot/main/scripts/install.sh | bash

# Check installed LLM tools
aidot detect

# Register a team preset repository
aidot repo add team https://github.com/myteam/llm-config

# Apply settings to the current project
aidot pull team
```

---

## Installation

### Script Install (Recommended)

**macOS / Linux:**
```bash
curl -fsSL https://raw.githubusercontent.com/Jooss287/aidot/main/scripts/install.sh | bash
```

**Windows (PowerShell):**
```powershell
irm https://raw.githubusercontent.com/Jooss287/aidot/main/scripts/install.ps1 | iex
```

### Manual Install from GitHub Releases

Download the appropriate binary for your platform from the [Releases page](https://github.com/Jooss287/aidot/releases):

| Platform | Filename |
|----------|----------|
| Linux (x64) | `aidot-vX.X.X-x86_64-unknown-linux-gnu.tar.gz` |
| Linux (ARM64) | `aidot-vX.X.X-aarch64-unknown-linux-gnu.tar.gz` |
| macOS (Intel) | `aidot-vX.X.X-x86_64-apple-darwin.tar.gz` |
| macOS (Apple Silicon) | `aidot-vX.X.X-aarch64-apple-darwin.tar.gz` |
| Windows (x64) | `aidot-vX.X.X-x86_64-pc-windows-msvc.zip` |

Add to your PATH after downloading.

### Build from Source

```bash
git clone https://github.com/Jooss287/aidot
cd aidot
cargo build --release

# Binary location: target/release/aidot
```

---

## Usage

### Command Overview

| Command | Description |
|---------|-------------|
| `aidot init` | Initialize a new preset repository |
| `aidot init --from-existing` | Extract preset from existing LLM configurations |
| `aidot repo add <name> <url>` | Register a preset repository |
| `aidot repo list` | List registered repositories |
| `aidot repo remove <name>` | Remove a repository |
| `aidot pull <name>` | Apply a preset |
| `aidot pull --dry-run` | Preview changes |
| `aidot detect` | Detect installed LLM tools |
| `aidot status` | Show current configuration status |
| `aidot diff <name>` | Compare preset with current config |
| `aidot cache update` | Update cached repositories |
| `aidot update` | Self-update aidot binary |

### Repository Management

```bash
# Register a Git repository
aidot repo add team https://github.com/myteam/llm-config

# Register a local folder as a preset
aidot repo add local-dev ./presets/dev-config --local

# Register with default flag (auto-applied on pull)
aidot repo add team https://github.com/myteam/llm-config --default

# List registered repositories
aidot repo list

# Remove a repository
aidot repo remove team
```

### Applying Settings

```bash
# Apply a specific preset
aidot pull team

# Apply all default repositories
aidot pull

# Apply to specific tools only
aidot pull team --tools claude,cursor

# Preview changes
aidot pull team --dry-run

# Overwrite existing settings
aidot pull team --force
```

### Creating Presets

```bash
# Create an empty preset structure
aidot init

# Extract preset from existing LLM configurations
aidot init --from-existing
```

---

## Preset Structure

An aidot preset has the following structure:

```
llm-preset/
├── .aidot-config.toml       # Preset configuration file
├── rules/                   # Rules/instructions (LLM behavior rules)
│   ├── code-style.md
│   └── security.md
├── memory/                  # Project memory (architecture, workflow)
│   └── architecture.md
├── commands/                # Custom slash commands
│   └── review.md
├── mcp/                     # MCP server settings
│   └── filesystem.json
├── hooks/                   # Event-driven automation hooks
│   └── pre-commit.json
├── agents/                  # AI agent definitions
│   └── code-reviewer.md
├── skills/                  # Agent utilities
│   └── api-client.ts
└── settings/                # Tool-specific general settings
    └── preferences.json
```

### .aidot-config.toml Example

```toml
[metadata]
name = "Team LLM Config"
version = "1.0.0"
description = "Shared team LLM settings"

[rules]
files = ["rules/*.md"]

[memory]
directory = "memory/"

[commands]
directory = "commands/"

[mcp]
directory = "mcp/"

[hooks]
directory = "hooks/"

[agents]
directory = "agents/"

[skills]
directory = "skills/"

[settings]
directory = "settings/"
```

---

## Supported Tools

### Claude Code

| Preset | Output |
|--------|--------|
| `rules/*.md` | `.claude/rules/` |
| `memory/*.md` | `.claude/CLAUDE.md` |
| `commands/*.md` | `.claude/commands/` |
| `mcp/*.json` | `.claude/settings.local.json` (mcpServers) |
| `hooks/*.json` | `.claude/hooks.json` |
| `agents/*.md` | `.claude/agents/` |
| `skills/*.ts` | `.claude/skills/` |
| `settings/*.json` | `.claude/settings.local.json` |

### Cursor

| Preset | Output |
|--------|--------|
| `rules/*.md` | `.cursor/rules/*.md` or `*.mdc` (with front matter) |
| `memory/*.md` | `.cursorrules` (Always Apply section) |
| `commands/*.md` | `.cursor/commands/` |
| `mcp/*.json` | `.cursor/mcp.json` |
| `hooks/*.json` | `.cursor/hooks.json` |
| `agents/*.md` | `.cursor/agents/` |
| `skills/*.ts` | `.cursor/skills/` |

### GitHub Copilot

| Preset | Output |
|--------|--------|
| `rules/*.md` | `.github/instructions/*.instructions.md` |
| `memory/*.md` | `.github/copilot-instructions.md` (Project Context) |
| `commands/*.md` | `.github/prompts/*.prompt.md` |
| `mcp/*.json` | `.vscode/mcp.json` |
| `agents/*.md` | `.github/agents/*.agent.md` |
| `skills/*.ts` | `.github/skills/` |

### Codex CLI

| Preset | Output |
|--------|--------|
| `rules/*.md` | `.codex/AGENTS.md` (merged with memory) |
| `memory/*.md` | `.codex/AGENTS.md` (merged with rules) |
| `commands/*.md` | `.codex/skills/<name>/SKILL.md` (deprecated → converted to skills) |
| `mcp/*.json` | `.codex/config.toml` (`[mcp_servers.*]`, JSON → TOML) |
| `agents/*.md` | `.codex/agents/<name>.md` + `.codex/config.toml` (`[agents.*]`) |
| `skills/**` | `.codex/skills/<name>/SKILL.md` (YAML front matter auto-generated) |
| `settings/*.json` | Not supported (warning, skipped) |

---

## Examples

### 1. New Project Setup

```bash
# Create a project
cargo new my-app && cd my-app

# Check LLM tools
aidot detect
# ✓ Cursor (IDE)
# ✓ Claude Code (CLI)

# Apply team settings
aidot pull team
# ✓ Cursor → .cursorrules
# ✓ Claude Code → .claude/rules/
```

### 2. Create Preset from Existing Config

```bash
# In a project with existing LLM settings
cd my-existing-project

# Extract settings into a preset
aidot init --from-existing
# Found:
#   ✓ .cursorrules
#   ✓ .claude/rules/
# Converting to aidot preset...

# Commit and share via Git
git init && git add . && git commit -m "Add LLM preset"
git remote add origin https://github.com/myteam/llm-config
git push -u origin main
```

### 3. Team Settings Sharing

```bash
# Teammate A: Register preset repository
aidot repo add team https://github.com/myteam/llm-config --default

# Teammate B: In a new project
cd new-project
aidot pull  # Automatically applies default repositories
```

---

## Contributing

If you'd like to contribute, please see [CONTRIBUTING.md](CONTRIBUTING.md).

---

## References

### Claude Code
- [Settings](https://code.claude.com/docs/en/settings)
- [Skills](https://code.claude.com/docs/en/skills)

### GitHub Copilot
- [Custom Instructions](https://docs.github.com/en/copilot/how-tos/configure-custom-instructions/add-repository-instructions)
- [Custom Agents](https://docs.github.com/en/copilot/how-tos/use-copilot-agents/coding-agent/create-custom-agents)

### Cursor
- [Rules](https://cursor.com/docs/context/rules)
- [Subagents](https://cursor.com/docs/agent/subagents)

### Codex CLI
- [AGENTS.md](https://developers.openai.com/codex/guides/agents-md)
- [Skills](https://developers.openai.com/codex/skills)

---

## License

MIT License
