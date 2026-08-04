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

## 3. Ask for a specific HTTP status code

`/status/{code}` is the first crawler-testing route: it makes the server
respond with whatever status code you ask for, so you can check how your
crawler reacts to it.

```sh
curl -i http://localhost:3000/status/404
```

Expected response:

```
HTTP/1.1 404 Not Found
content-length: 0
```

Anything outside the 100-999 range is rejected with `400 Bad Request`.

## 4. Inspect and inject headers

`/headers/echo` reflects back everything the server received — the
quickest way to check what your crawler actually sends:

```sh
curl -H "X-Foo: bar" http://localhost:3000/headers/echo
```

```json
{"accept": ["*/*"], "host": ["localhost:3000"], "x-foo": ["bar"]}
```

`/headers/set` does the opposite: it lets *you* force arbitrary response
headers via the query string, to check how your crawler reacts to them.

```sh
curl -i "http://localhost:3000/headers/set?x-robots-tag=noindex"
```

```
HTTP/1.1 200 OK
x-robots-tag: noindex
content-length: 0
```

## What's next

This tutorial only walks through `/health`, `/status/{code}`, and
`/headers/*` so far, but bot-camp has grown well beyond that —
redirects, canonical tags, robots.txt, rate limiting, anti-bot
mechanisms, HTML/JS content scenarios, URL discovery, a generic
`POST /response` endpoint, and a live dashboard. This tutorial will
grow to walk through each family one by one; until then, the
[API reference](./api.md) is the source of truth for what's currently
available, and the [root README](../../README.md) gives a feature
overview.
