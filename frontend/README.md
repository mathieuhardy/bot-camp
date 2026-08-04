# bot-camp dashboard

The dashboard's frontend: Svelte 5 + Tailwind + shadcn-svelte, built with
Vite. It has no server of its own — it fetches its initial state and live
event feed from `/dashboard/snapshot` and `/dashboard/ws`, served by the
main `bot-camp` binary.

## Build

```sh
npm ci
npm run build
```

This produces `frontend/dist`, which the Rust build **embeds into the
binary at compile time** (via `rust-embed` in `src/routes/dashboard.rs`).
That means `frontend/dist` must exist and be up to date before running
`cargo build`, `cargo test`, or `cargo run` from the repository root —
`cargo` has no way to trigger the frontend build itself.

In debug builds, `rust-embed` reads `frontend/dist` from disk on every
request instead of baking it in, so `npm run build` + a page reload is
enough to see frontend changes without recompiling the Rust binary.

## Develop

```sh
npm run dev      # Vite dev server with hot reload, proxying
                  # /dashboard/snapshot and /dashboard/ws to a bot-camp
                  # instance on localhost:3000 (see vite.config.ts) — open
                  # http://localhost:5173/dashboard/ (trailing slash)
npm run check    # svelte-check + tsc
```
