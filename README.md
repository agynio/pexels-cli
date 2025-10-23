# Pexels CLI (Rust)

Agent-optimized CLI for the Pexels API.

Install
- Build from source: `cargo install --path pexels`

Auth
- Env: `PEXELS_TOKEN` (or `PEXELS_API_KEY`); fallback order: `PEXELS_TOKEN` → `PEXELS_API_KEY`.
- Config file: `~/.config/pexels/config.yaml` (or OS equivalent). Use `pexels auth login [TOKEN]`.

Usage examples
- `pexels auth status`
- `pexels photos search -q cats`
- `pexels photos curated`
- `pexels videos popular`
- `pexels collections featured`

Output
- Successful outputs are wrapped as `{ data: <payload> }` for single-resource outputs, and `{ data: <items[]>, meta: { total_results?, next_page?, prev_page?, request_id? } }` for list endpoints.
- For list endpoints, `data` is the items array (photos/videos/collections/media). For single-resource endpoints, `data` is the object and `meta` is omitted.
- `page`/`per_page` are omitted. `next_page`/`prev_page` are integers (page numbers) or null.
- Field selection via `--fields` supports dot paths and sets: `@ids,@urls,@files,@thumbnails,@all`.
- Some fields are omitted by default for lighter responses; include heavy fields via `--fields`.

Testing
- Unit tests cover projection, config precedence, error mapping, and page parsing.
- Live tests run in CI when `PEXELS_TOKEN` is present and event is safe. Commands:
  - `pexels auth status`
  - `pexels photos search -q cats`
  - `pexels photos curated`
  - `pexels videos popular`
  - `pexels collections featured`

CI/Release
- CI runs: lint -> unit tests -> build -> live tests (guarded by `secrets.PEXELS_TOKEN` and internal PRs/main).
- On push to main, a release tag is created and binaries for Linux/macOS/Windows are uploaded.
