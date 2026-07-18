# NODS CLI

> **Component doc** (this is the `cli/` REST client). Canonical references:
> [`../docs/CLI_USER_GUIDE.md`](../docs/CLI_USER_GUIDE.md), [`../docs/CLI_COMMAND_REFERENCE.md`](../docs/CLI_COMMAND_REFERENCE.md),
> and [`../docs/API_REFERENCE.md`](../docs/API_REFERENCE.md). Docs map: [`../docs/README.md`](../docs/README.md).

Manage NODS terminal-workspace configs and sessions from your terminal.

NODS is a terminal-workspace orchestrator (Rust/Axum backend + Next.js product
app + Tauri/Electron desktop). This CLI is a **Rust** binary (clap), unlike the
other AHL CLIs which are Python/Click + Nuitka. It talks to the same NODS API.

## Install

```bash
# From a release (curl-installer; SEGA-published)
curl -fsSL https://downloads.nods.dev/cli/install.sh | bash -s -- --version <version>

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
api_url: http://localhost:8016   # optional; prod: https://nods.dev
timeout: 60                      # optional, seconds
```

Environment variables override the config file:

- `NODS_API_KEY` — API key
- `NODS_API_URL` — API base URL (default `http://localhost:8016`)
- `NODS_TIMEOUT` — request timeout (seconds)
- `NODS_MAX_RETRIES` — retry count for 429/5xx and connect errors (default 3)

Authentication uses the AHL-standard header `Authorization: ApiKey nods_...`.

## Output formats

`--output table` (default) or `--output json`. JSON is stable and suitable for
scripting/CI.

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

`nods` is in SEGA's `BINARY_PROJECTS` as `nods: "rust"` (cargo), **not** in
`CLI_PROJECTS` (the Nuitka/Python path). The release artifact is built with
`cargo build --release` and published to
`downloads.nods.dev/cli/releases/<version>/nods-<os>-<arch>` with a sibling
`SHA256SUMS` and `install.sh` — the same on-disk layout as the Python CLIs.
