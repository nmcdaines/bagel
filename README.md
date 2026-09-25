# bagel

An [axum](https://github.com/tokio-rs/axum) server that serves a
[Vite](https://vite.dev) + [React Router](https://reactrouter.com) single-page app.

```
.
├── src/          # axum server (API under /api, SPA fallback for everything else)
├── web/          # Vite + React Router frontend
├── openapi.json  # generated API contract between the two
└── Dockerfile  # multi-stage build → distroless image
```

## Development

Run the API and the Vite dev server side by side (Vite proxies `/api` to `localhost:3000`):

```sh
cargo run                    # http://localhost:3000
cd web && npm install && npm run dev   # http://localhost:5173
```

To serve the built frontend from axum directly:

```sh
(cd web && npm run build) && cargo run   # serves web/dist
```

### API contract

The Rust handlers are the source of truth. Each `/api` handler is annotated with `#[utoipa::path]`,
its request/response types derive `ToSchema`, and it is registered with `routes!` in `src/lib.rs`.
From that, the server emits an OpenAPI document that is committed as `openapi.json`.
`openapi-typescript` generates `web/src/api.gen.ts` from it. The frontend calls the API through the
typed `openapi-fetch` client in `web/src/api.ts`, so a wrong path, parameter or field is a type error.

After changing the API, regenerate both files and commit them:

```sh
cd web && npm run gen:api
```

CI fails if either is stale: `cargo test` checks `openapi.json` and the web job checks `api.gen.ts`.

### Configuration

| Env var            | Default    | Description                        |
| ------------------ | ---------- | ---------------------------------- |
| `PORT`             | `3000`     | Port to listen on                  |
| `BAGEL_STATIC_DIR` | `web/dist` | Directory of built frontend assets |
| `RUST_LOG`         | `bagel=info,tower_http=info` | Log filter       |

## Docker

```sh
docker build -t bagel .
docker run --rm -p 3000:3000 bagel
```

## CI

`.github/workflows/ci.yml` runs fmt/clippy/tests for Rust and lint/build for the frontend, then builds the
Docker image. Images are pushed to `ghcr.io/<owner>/bagel` on pushes to `main` (`latest`, `sha-…`) and on
`v*` tags (`1.2.3`, `1.2`). Pull requests build the image without pushing.
