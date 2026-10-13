# syntax=docker/dockerfile:1.7

# ─── Frontend build stage (Vite & npm) ──────────────────────────────────────
FROM node:22-alpine AS assets
WORKDIR /build

COPY package.json package-lock.json ./
RUN npm ci

COPY vite.config.js ./
COPY frontend ./frontend
COPY src ./src
COPY static ./static

RUN npm run build

# ─── Rust build stage (Alpine) ──────────────────────────────────────────────
FROM rust:1-alpine AS builder
WORKDIR /build

# musl-dev is required for linking C dependencies like ring/aws-lc-rs or mimalloc
RUN apk add --no-cache musl-dev

COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src \
 && echo 'fn main() {}' > src/main.rs \
 && echo 'pub fn dummy() {}' > src/lib.rs \
 && cargo fetch \
 && rm -rf src

COPY src ./src
COPY i18n ./i18n
RUN cargo build --release --bin mailer

# ─── Runtime stage (Alpine) ─────────────────────────────────────────────────
FROM alpine:latest
WORKDIR /app

RUN apk add --no-cache ca-certificates tzdata

COPY --from=builder /build/target/release/mailer /usr/local/bin/mailer
COPY --from=assets  /build/static                /app/static
COPY i18n                                        /app/i18n

ENV BIND_ADDR=0.0.0.0:3000 \
    APP_URL=http://localhost:3000 \
    RUST_LOG=info,mailer=info

EXPOSE 3000

# Persist uploaded brand logos and local attachments between container restarts.
# Mount your GeoLite2 .mmdb at /app/data/GeoLite2-Country.mmdb (or set
# GEOIP_DB) for country-by-IP enrichment on subscribe.
VOLUME ["/app/static/uploads", "/app/data"]

CMD ["/usr/local/bin/mailer"]
