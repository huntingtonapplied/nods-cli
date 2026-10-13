# Changelog

All notable changes to the NODS CLI are documented here.
The format is based on [Keep a Changelog](https://keepachangelog.com/).

## [0.1.0] - 2026-06-28

Initial release. Rust (clap) binary named `nods`, published via SEGA to
`downloads.nods.dev/cli/releases/<version>/nods-<os>-<arch>`.

### Added
- `nods login` — store a NODS API key (`nods_...`) in `~/.nods/config.yaml`,
  validated against `GET /v1/account/profile` before saving.
- `nods status` — account + active-session summary.
- `nods doctor` — config / key / API-reachability diagnostics (non-zero exit on failure).
- `nods metrics [usage|performance|backends|daily|errors]` — analytics views.
- `nods completion [bash|zsh|fish|...]` — shell-completion script generation.
- `nods configs [list|show|validate|delete]` — workspace configurations.
- `nods sessions [list|show|launch|kill]` — terminal sessions.
- `nods templates [list|show]` — public template library.
- `nods events` — recent account events.
- `nods resources` — live system resource usage.
- Global `--output table|json`, `--api-url`, `--verbose`.
- HTTP client with retry/backoff on 429/5xx and connect/timeout errors.
- Config precedence: env (`NODS_API_KEY` / `NODS_API_URL`) > `~/.nods/config.yaml` > defaults.
- Categorized exit codes (0 ok, 2 auth, 3 not-found, 4 validation, 5 network, 6 server).
- `install.sh` curl-installer with SHA256SUMS verification.
