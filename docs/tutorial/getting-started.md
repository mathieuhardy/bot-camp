# Get started

## Install

### From source

Requires a Rust toolchain (edition 2024, so a recent stable compiler —
1.85 or newer).

```sh
git clone <repo-url>
cd bot-camp
cargo build --release
```

The binary is produced at `target/release/bot-camp`.

### With Docker

```sh
docker build -t bot-camp .
```

## Setup

The server has no required configuration yet — it starts with sane
defaults out of the box. This section will grow as config file support
(`config.rs`, YAML/TOML) lands.

| Setting | Default | Notes |
|---|---|---|
| Listen address | `0.0.0.0:3000` | Not yet configurable. |
| Log level | `info` | Override with the `RUST_LOG` env var (e.g. `RUST_LOG=debug`). |

## Logs

Every request produces one JSON line on stdout, at `info` level:

```json
{"timestamp":"...","level":"INFO","message":"request","method":"GET","path":"/ratelimit/foo","ip":"127.0.0.1","user_agent":"curl/8.5.0","status":429,"latency_ms":0,"rule":"rate_limit_limited","target":"bot_camp::logging"}
```

`rule` reports which middleware (if any) decided the response — e.g.
`rate_limit_limited`, `rate_limit_banned`, `rate_limit_blocked`,
`honeypot_sprung`, `honeypot_banned`, `challenge_blocked` — or `none`
for an ordinary request. Pipe stdout to a file or a log collector to
replay/analyze how a crawler under test actually behaved.

## Deployment

### Run locally

```sh
cargo run --release
# or, after building:
./target/release/bot-camp
```

### Run with Docker

```sh
docker run --rm -p 3000:3000 bot-camp
```

The server listens on port `3000` inside the container; `-p 3000:3000`
exposes it on the host.

### Verify it's up

```sh
curl -i http://localhost:3000/health
```

Expect a `200 OK` response with an empty body.

Next: the [tutorial](./tutorial.md) walks through this first request in
more detail, and the [API reference](./api.md) documents every route.
