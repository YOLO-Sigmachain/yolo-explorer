# YOLO explorer

This is the YOLO SigmaChain fork of [Kadia Explorer](https://github.com/arkadianet/kadia-explorer).

- **Starts from:** upstream `master` at commit `25fda1d09476b0d4fb8f674ca26d0e0f75c63a40` (the upstream project has no tags).
- **What changes:**
  - A chain profile (name, ticker, address prefix, block time, storage rent, emission) set in the `[chain]` section of the configuration file, with a three-party emission (miner, treasury, liquidity fund) and a miner lock in blocks.
  - Frontend settings for chain name, ticker, logo and a network switch, and the Applications section can be switched off.
  - Tests for the chain profile in the API and the frontend.
- **What does not change:** With no `[chain]` section in the configuration, the indexer uses the built-in Ergo mainnet profile.
- **Example settings for a YOLO network:** `explorer.yolo.example.toml`.
- **Compare with upstream:** Every change is in the commits after the starting commit above. GitHub's compare view against the upstream project shows them.
- **Licence:** MIT OR Apache-2.0, as declared in the upstream Rust package files. The upstream repository does not yet carry the licence texts.

---

## Upstream README

Everything below is the README of Kadia Explorer, kept as upstream wrote it. It describes the upstream project, not this fork. The hosted site it names is upstream's.

# Kadia Explorer

A standalone Ergo blockchain explorer: a Rust indexer + JSON API over an embedded
[redb](https://github.com/cberner/redb) store, and a SvelteKit frontend. It syncs from any
Ergo node's REST API (the Rust node's extra index is not required), keeps its own compact
index (boxes, transactions, addresses, tokens, script templates, register values, storage
rent), and serves it at <https://explorer.kadia.io>.

| Part                   | Where                          | Docs                                                                                    |
| ---------------------- | ------------------------------ | --------------------------------------------------------------------------------------- |
| Indexer + API binary   | `bin/explorer`, `crates/xp-*`  | [`bin/explorer/README.md`](bin/explorer/README.md)                                      |
| Frontend               | `frontend/`                    | [`frontend/README.md`](frontend/README.md)                                              |
| Design specs and plans | `docs/superpowers/`            | start with the [design spec](docs/superpowers/specs/2026-09-05-ergo-explorer-design.md) |
| Changes                | [`CHANGELOG.md`](CHANGELOG.md) |                                                                                         |

## Quick start

```sh
cargo build --release -p explorer
cp explorer.example.toml explorer.toml   # point `url` at an Ergo node
./target/release/explorer --config explorer.toml
# API on http://127.0.0.1:8090/v1/status
cd frontend && npm ci && npm run dev      # proxies /v1 to the API
```

Gates: `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`;
frontend `npm test && npm run check && npm run lint && npm run build` and `npx playwright test`.
