<div align="center">
  <img src=".readme/logo.png" alt="NODS CLI" width="360"><br><br>
</div>

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](#license)
[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://rust-lang.org)
[![Status](https://img.shields.io/badge/status-active-success.svg)](#)

-----------------

**NODS CLI** is the command-line client for NODS, the Terminal Workspace Orchestrator. It lets you manage NODS terminal-workspace configs and sessions — list, validate, launch, and kill workspaces, and read usage/performance analytics — directly from your terminal, talking to the hosted NODS API.

This CLI is a **Rust** binary (`clap` + `reqwest`), unlike the other Huntington Applied CLIs, which are Python/Click + Nuitka. It is a thin REST client for the same NODS API served by the Rust/Axum backend; it does not perform orchestration itself (that is the `engine/` binary — also named `nods` — which drives tmux/SSH locally).

> **Component doc.** Canonical references: `../docs/CLI_USER_GUIDE.md`, `../docs/CLI_COMMAND_REFERENCE.md`, and `../docs/API_REFERENCE.md`. Docs map: `../docs/README.md`.

## Install

```bash
# Standalone binary (latest release; verifies SHA256 before installing)
curl -fsSL https://downloads.nods.dev/cli/install.sh | bash

# Pin a release, or install without sudo into ~/.local/bin
curl -fsSL https://downloads.nods.dev/cli/install.sh | bash -s -- --version 0.1.0
curl -fsSL https://downloads.nods.dev/cli/install.sh | bash -s -- --install-dir ~/.local/bin

# From source
cargo build --release      # produces target/release/nods
cargo install --path .     # installs `nods` onto your PATH
```

## Quick Start

```bash
# Authenticate (paste the API key from the product app: Account > Developer)
nods login

# Check setup / connectivity
nods doctor

# Account + active-session summary
nods status

# Workspace configs
nods configs list
nods configs show <id>
nods configs validate <id>

# Sessions
nods sessions list --status active
nods sessions launch <config_id>
nods sessions kill <session_id>

# Analytics
nods metrics usage
nods metrics performance

# JSON output anywhere
nods -o json sessions list
```

## Authentication

Get an API key from the product app (**Account → Developer**), then run `nods login` to store it. Keys use the `nods_` prefix and are sent in the standard header `Authorization: ApiKey nods_...`. The key can also be provided via the `NODS_API_KEY` environment variable (see [Configuration](#configuration)).

## Commands

| Command | API endpoint | Description |
|---------|--------------|-------------|
| `nods login` | `GET /v1/account/profile` (validate) | Store API key in `~/.nods/config.yaml` |
| `nods status` | `GET /v1/account/profile`, `GET /v1/sessions?status=active` | Account + active-session summary |
| `nods doctor` | `GET /health`, `GET /v1/account/profile` | Diagnose setup and connectivity |
| `nods metrics <kind>` | `GET /v1/analytics/{usage,performance,backends,daily,errors}` | Analytics views |
| `nods completion <shell>` | — | Shell-completion script (clap_complete) |
| `nods configs list` | `GET /v1/configs` | List workspace configs |
| `nods configs show <id>` | `GET /v1/configs/:id` | Show one config |
| `nods configs validate <id>` | `POST /v1/configs/:id/validate` | Validate a config |
| `nods configs delete <id>` | `DELETE /v1/configs/:id` | Delete a config |
| `nods sessions list` | `GET /v1/sessions` | List sessions |
| `nods sessions show <id>` | `GET /v1/sessions/:id` | Show one session |
| `nods sessions launch <config_id>` | `POST /v1/sessions/launch/:config_id` | Launch a session |
| `nods sessions kill <id>` | `DELETE /v1/sessions/:id` | Stop a session |
| `nods templates list` | `GET /v1/templates` | List public templates |
| `nods templates show <id>` | `GET /v1/templates/:id` | Show one template |
| `nods events` | `GET /v1/events` | Recent account events |
| `nods resources` | `GET /v1/sessions/system/resources` | Live resource usage |

## Configuration

Config is stored in `~/.nods/config.yaml`:

```yaml
api_key: nods_live_abc123...
api_url: https://api.nods.dev    # optional; defaults to prod. Local dev: http://localhost:8016
timeout: 60                      # optional, seconds
```

Environment variables override the config file:

- `NODS_API_KEY` — API key
- `NODS_API_URL` — API base URL (default `https://api.nods.dev`)
- `NODS_TIMEOUT` — request timeout (seconds)
- `NODS_MAX_RETRIES` — retry count for 429/5xx and connect errors (default 3)

## Output formats

`--output table` (default) or `--output json`. JSON is stable and suitable for scripting/CI.

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Authentication error (no/invalid key) |
| 3 | Not found |
| 4 | Validation error |
| 5 | Network error |
| 6 | Server error (5xx) |

## Build & publish

`nods` is in SEGA's `BINARY_PROJECTS` as `nods: "rust"` (cargo), **not** in `CLI_PROJECTS` (the Nuitka/Python path). The release artifact is built with `cargo build --release` and published to `downloads.nods.dev/cli/releases/<version>/nods-<os>-<arch>` with a sibling `SHA256SUMS` and `install.sh` — the same on-disk layout as the Python CLIs.

## Documentation & resources

- Project front door — `../README.md`
- CLI User Guide — `../docs/CLI_USER_GUIDE.md`
- CLI Command Reference — `../docs/CLI_COMMAND_REFERENCE.md`
- API Reference — `../docs/API_REFERENCE.md`
- Documentation map — `../docs/README.md`

## License

This project is licensed under the MIT License.
