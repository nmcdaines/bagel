# syntax=docker/dockerfile:1

FROM node:24-alpine AS web
WORKDIR /app/web
COPY web/package.json web/package-lock.json ./
RUN --mount=type=cache,target=/root/.npm npm ci
COPY web/ ./
RUN npm run build

FROM rust:1-slim-trixie AS server
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release --locked --bin bagel \
    && cp target/release/bagel /usr/local/bin/bagel

FROM gcr.io/distroless/cc-debian13:nonroot
COPY --from=server /usr/local/bin/bagel /app/bagel
COPY --from=web /app/web/dist /app/static
ENV BAGEL_STATIC_DIR=/app/static \
    PORT=3000
EXPOSE 3000
ENTRYPOINT ["/app/bagel"]
