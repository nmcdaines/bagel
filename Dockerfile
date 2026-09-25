# syntax=docker/dockerfile:1

FROM node:24-alpine AS web
WORKDIR /app/web
COPY web/package.json web/package-lock.json ./
RUN --mount=type=cache,target=/root/.npm npm ci
COPY web/ ./
RUN npm run build

FROM rust:1-slim-trixie AS server
WORKDIR /app
# sqlx query macros read the committed .sqlx/ metadata instead of a live database.
ENV SQLX_OFFLINE=true
COPY Cargo.toml Cargo.lock build.rs ./
COPY .sqlx ./.sqlx
COPY migrations ./migrations
COPY src ./src
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release --locked --bin bagel \
    && cp target/release/bagel /usr/local/bin/bagel \
    && mkdir -p /data

FROM gcr.io/distroless/cc-debian13:nonroot
COPY --from=server /usr/local/bin/bagel /app/bagel
COPY --from=web /app/web/dist /app/static
# SQLite lives in /data (owned by the distroless nonroot user); mount a volume there to persist it.
COPY --from=server --chown=65532:65532 /data /data
ENV BAGEL_STATIC_DIR=/app/static \
    DATABASE_URL=sqlite:///data/bagel.db?mode=rwc \
    PORT=3000
VOLUME /data
EXPOSE 3000
ENTRYPOINT ["/app/bagel"]
