# syntax=docker/dockerfile:1

### ---- Frontend builder ----
FROM node:22-slim AS frontend-builder
WORKDIR /app/frontend

COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci

COPY frontend ./
RUN npm run build

### ---- Builder ----
FROM rust:1-slim-bookworm AS builder
WORKDIR /app

# Layer 1: build dependencies only, so this layer stays cached as long as
# Cargo.toml/Cargo.lock don't change (independent of source code edits).
COPY Cargo.toml Cargo.lock ./
RUN mkdir src \
    && echo "fn main() {}" > src/main.rs \
    && cargo build --release \
    && rm -rf src

# Layer 2: build the actual source, reusing the cached dependency layer
# above. The dashboard's assets are embedded into the binary at compile
# time (rust-embed), so frontend/dist must exist before this build.
COPY src ./src
COPY templates ./templates
COPY --from=frontend-builder /app/frontend/dist ./frontend/dist
RUN touch src/main.rs && cargo build --release

### ---- Runtime ----
FROM debian:bookworm-slim AS runtime

RUN useradd --system --no-create-home --uid 1000 botcamp
COPY --from=builder /app/target/release/bot-camp /usr/local/bin/bot-camp

USER botcamp
EXPOSE 3000

ENTRYPOINT ["/usr/local/bin/bot-camp"]
