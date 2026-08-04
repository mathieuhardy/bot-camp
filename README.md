# bot-camp

[![CI](https://github.com/mathieuhardy/bot-camp/actions/workflows/ci.yml/badge.svg)](https://github.com/mathieuhardy/bot-camp/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**A self-hosted test server for crawlers and scrapers.** Point your bot
at it to check how it actually behaves against HTTP status codes and
headers, `robots.txt`/meta robots, redirects and canonical tags, rate
limiting and anti-bot mechanisms, and tricky HTML/JS content — before
it meets the real web.

## Why

Most crawler test sites (e.g. crawler-test.com) work as hundreds of
hardcoded static pages, one per case. That's the wrong model for a tool
developers self-host and want to extend: bot-camp is instead a generic
engine driven by URL parameters and query strings, plus one
ultra-configurable [`POST /response`](docs/tutorial/api.md#post-response)
endpoint that composes status code, headers, delay, and page content in
a single JSON request. New test cases don't need a recompile, and
existing single-purpose routes stay as documented, discoverable
building blocks rather than one-off pages.

It also ships as a single self-hosted binary — no external database,
no required config file, and (see below) no runtime dependency on
Node/npm even though its dashboard is a real Svelte app.

## Features

- **HTTP status codes & headers** — any status from 100 to 999
  (`/status/{code}`), echo received headers (`/headers/echo`), inject
  arbitrary response headers (`/headers/set`), controlled response
  delay (`/delay/{ms}`) and body size (`/large-response/{kb}`), Basic
  Auth (`/auth/basic`).
- **robots.txt & meta robots** — a `robots.txt` you can rewrite at
  runtime (`PUT /robots.txt`), meta robots + `X-Robots-Tag` including
  deliberately conflicting combinations (`/robots/meta`).
- **Redirects & canonical** — every redirect status with method-
  preservation semantics (`/redirect/{code}`), chains and loops
  (`/redirect/chain`, `/redirect/loop`), header- and meta-refresh
  redirects, canonical tag variations (`/canonical`), URL
  normalization checks (`/normalize`).
- **HTML & JS content** — title/H1/word-count/duplicate-content
  scenarios (`/content`), signals injected into the DOM after a delay
  (`/js-render`), charset mismatches and double-encoding
  (`/encoding`), deliberately malformed markup (`/broken-html`).
- **URL discovery** — a fixed, deterministic set of target URLs spread
  across 11 different HTML mechanisms (`<a>`, `<link>`, `<img>`,
  `<script src>`, an HTML comment, a JS string, a CSS `url()`, a
  protocol-relative href, `<form>`, `<iframe>`, `<area>`) to check what
  your crawler actually extracts (`/discovery`).
- **Rate limiting & anti-bot** — pluggable algorithms (token bucket,
  fixed/sliding window, minimum interval), keyed by IP/User-Agent/both,
  with allow/block lists and a two-tier ban (`/ratelimit/*`); a
  honeypot that bans on first visit to a hidden link (`/honeypot/*`); a
  "checking your browser" JS challenge gate (`/challenge/*`).
- **Generic response endpoint** — `POST /response` composes status,
  headers, delay, and either a raw body or a full page description in
  one JSON request, for test cases that need several of the above at
  once.
- **Live dashboard** — `/dashboard`, a Svelte + shadcn-svelte
  single-page app streaming rate limiter/honeypot decisions over a
  WebSocket in real time.
- **Structured logs** — one JSON line per request (method, path, IP,
  User-Agent, status, latency, and which rule — if any — decided the
  response), to replay or analyze how a crawler under test behaved.

See the [API reference](docs/tutorial/api.md) for every route,
documented in full with request/response tables and examples.

## Quick start

### With Docker

```sh
docker build -t bot-camp .
docker run --rm -p 3000:3000 bot-camp
```

### From source

Requires a Rust toolchain (edition 2024 — 1.85 or newer) and Node.js
(to build the dashboard's frontend once, embedded into the binary at
compile time — see [`frontend/README.md`](frontend/README.md)):

```sh
git clone git@github.com:mathieuhardy/bot-camp.git
cd bot-camp
(cd frontend && npm ci && npm run build)
cargo run --release
```

### With Nix

```sh
nix develop   # drops you into a shell with the Rust toolchain and Node
```

Either way, the server listens on `0.0.0.0:3000`:

```sh
curl -i http://localhost:3000/health
```

## Documentation

- [Get started](docs/tutorial/getting-started.md) — install, setup,
  deployment.
- [Tutorial](docs/tutorial/tutorial.md) — a minimal walkthrough of your
  first request.
- [API reference](docs/tutorial/api.md) — every route, in full.

## Development

```sh
cargo fmt --all -- --check
cargo clippy --all-targets
cargo nextest run --locked
```

The dashboard's frontend (`frontend/`) is a separate Vite/Svelte
project whose build output is embedded into the Rust binary at compile
time — `frontend/dist` must exist (and be current) before `cargo
build`/`cargo test`. See [`frontend/README.md`](frontend/README.md).

CI (`.github/workflows/ci.yml`) runs formatting, linting, the frontend
typecheck, and the full test suite on every push and pull request.

## License

MIT — see [LICENSE](LICENSE).
