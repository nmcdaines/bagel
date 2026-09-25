# bagel

An [axum](https://github.com/tokio-rs/axum) server that serves a
[Vite](https://vite.dev) + [React Router](https://reactrouter.com) single-page app.

```
.
├── src/        # axum server (API under /api, SPA fallback for everything else)
├── web/        # Vite + React Router frontend
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
