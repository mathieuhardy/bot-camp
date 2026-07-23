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
