# Tutorial

A minimal, hands-on walkthrough of your first request against bot-camp.
See [Get started](./getting-started.md) if you haven't installed or run
the server yet.

## 1. Start the server

```sh
cargo run --release
```

You should see a log line confirming the server is listening:

```
INFO bot_camp: Starting server on 0.0.0.0:3000
```

## 2. Check the server is alive

Every route in bot-camp is meant to simulate a specific crawler-facing
behavior — but before testing any of that, `/health` confirms the server
process itself is up and responding.

```sh
curl -i http://localhost:3000/health
```

Expected response:

```
HTTP/1.1 200 OK
content-length: 0
```

## What's next

bot-camp is still at the skeleton stage: `/health` is the only route so
far. As routes for HTTP status codes, redirects, robots.txt, rate
limiting, and HTML content scenarios are implemented (see the
[roadmap](../../README.md)), this tutorial will grow to walk through each
family one by one. Until then, the [API reference](./api.md) is the
source of truth for what's currently available.
