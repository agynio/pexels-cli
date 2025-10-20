# Pexels CLI (Rust)

Agent-optimized CLI for the Pexels API.

Install
- Build from source: `cargo install --path pexels`

Auth
- Env: `PEXELS_TOKEN` (or `PEXELS_API_KEY`)
- Config file: `~/.config/pexels/config.yaml` (or OS equivalent). Use `pexels auth login --token ...`.

Usage examples
- `pexels photos search 'cats' --per-page 5`
- `pexels photos url <id>` -> outputs `{ data: string, meta }` where data is src.<size> (default `original`). Use `--size` with one of: `original|large2x|large|medium|small|portrait|landscape|tiny`.
- `pexels photos download <id> <path>` -> downloads to path and outputs `{ data: { path, bytes }, meta }`
- `pexels videos popular --json --fields @urls`
- `pexels collections list --all --limit 50`
- `PEXELS_TOKEN=... pexels quota view`
- `pexels --host http://localhost:8080 util ping`

Output
- Default YAML with stable order; `--json` for JSON; `--raw` for raw HTTP body.
- Field selection via `--fields` supports dot paths and sets: `@ids,@urls,@files,@thumbnails,@all`.
- All structured outputs are wrapped as `{ data, meta }`. For list endpoints, `data` is the projected items array and `meta` carries paging info. For single resources, `data` falls back to non-empty projection.

Testing
- Tests use a mock server via `--host` override.
- Live tests run only when `PEXELS_TOKEN` is present (for pushes to `main` and PRs within the upstream repo). They are skipped on forks and when no token is available.

CI/Release
- CI runs fmt, clippy, tests, and integration (mock) tests.
- On push to main, a release tag is created and binaries for Linux/macOS/Windows are uploaded.
