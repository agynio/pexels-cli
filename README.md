# Pexels CLI (Rust)

Agent-optimized CLI for the Pexels API.

Install
- Build from source: `cargo install --path pexels`

Auth
- Env: `PEXELS_TOKEN` (or `PEXELS_API_KEY`)
- Config file: `~/.config/pexels/config.yaml` (or OS equivalent). Use `pexels auth login --token ...`.

Usage examples
- `pexels photos search 'cats' --per-page 5`
- `pexels videos popular --json --fields @urls`
- `pexels collections list --all --limit 50`
- `PEXELS_TOKEN=... pexels quota view`
- `pexels --host http://localhost:8080 util ping`

Output
- Default YAML with stable order; `--json` for JSON; `--raw` for raw HTTP body.
- Field selection via `--fields` supports dot paths and sets: `@ids,@urls,@files,@thumbnails,@all`.

Testing
- Tests use a mock server via `--host` override. No live API calls in CI.

CI/Release
- CI runs fmt, clippy, tests, and integration (mock) tests.
- On push to main, a release tag is created and binaries for Linux/macOS/Windows are uploaded.
