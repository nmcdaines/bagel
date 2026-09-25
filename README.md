# bagel

An [axum](https://github.com/tokio-rs/axum) server that serves a
[Vite](https://vite.dev) + [React Router](https://reactrouter.com) single-page app.

```
.
├── src/          # axum server (API under /api, SPA fallback for everything else)
├── migrations/   # sqlx SQL migrations, embedded and run at startup
├── .sqlx/        # sqlx offline query metadata (generated, committed)
├── tests/        # API integration tests (in-memory SQLite)
├── web/          # Vite + React Router frontend
├── openapi.json  # generated API contract between the two
└── Dockerfile    # multi-stage build → distroless image
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
typed [TanStack Query](https://tanstack.com/query) hooks in `web/src/api.ts` (built with
`openapi-react-query` on top of `openapi-fetch`), e.g. `api.useQuery('get', '/api/health')`, so a wrong
path, parameter or field is a type error.

After changing the API, regenerate both files and commit them:

```sh
cd web && npm run gen:api
```

CI fails if either is stale: `cargo test` checks `openapi.json` and the web job checks `api.gen.ts`.

### Configuration

| Env var            | Default                      | Description                                       |
| ------------------ | ---------------------------- | ------------------------------------------------- |
| `PORT`             | `3000`                       | Port to listen on                                 |
| `BAGEL_STATIC_DIR` | `web/dist`                   | Directory of built frontend assets                |
| `DATABASE_URL`     | `sqlite://bagel.db?mode=rwc` | SQLite database (file is created if missing)      |
| `RUST_LOG`         | `bagel=info,tower_http=info` | Log filter                                        |

## Notes API

The notes endpoints are part of the OpenAPI contract (`openapi.json`, schemas `Note`, `NoteInput` and
`ErrorBody`); the `/notes` page uses them through the generated client.

| Method   | Path              | Body                  | Response                            |
| -------- | ----------------- | --------------------- | ----------------------------------- |
| `GET`    | `/api/notes`      |                       | `200` notes, most recently updated first |
| `POST`   | `/api/notes`      | `{ "title", "body"? }` | `201` note, `422` if title is empty |
| `GET`    | `/api/notes/{id}` |                       | `200` note, `404` if missing        |
| `PUT`    | `/api/notes/{id}` | `{ "title", "body"? }` | `200` note, `404`, `422`            |
| `DELETE` | `/api/notes/{id}` |                       | `204`, `404` if missing             |

## Database & migrations

Persistence is SQLite via [sqlx](https://github.com/launchbadge/sqlx) (no ORM). SQLite is bundled into the
binary, so no system `libsqlite3` is needed. Connections use WAL mode with foreign keys enabled.

**Migrations** are plain SQL files in `migrations/` (`<timestamp>_<name>.sql`). They are embedded into the
binary with `sqlx::migrate!()` and applied automatically at server startup; applied versions are tracked in the
`_sqlx_migrations` table. Migrations are forward-only: never edit one that has already been applied, add a new
one instead.

**Queries** use sqlx's compile-time checked macros (`query!` / `query_as!`). The macros read the committed
`.sqlx/` directory when no `DATABASE_URL` is set at build time (or when `SQLX_OFFLINE=true`), so builds, CI and
Docker need no database. If you do export `DATABASE_URL` while building, the macros check against that
database instead, so it must be migrated; set `SQLX_OFFLINE=true` to force the committed metadata.

Install the CLI (a dev tool only — the build does not need it):

```sh
cargo install sqlx-cli --no-default-features --features sqlite
```

Add a migration:

```sh
sqlx migrate add <name>   # creates migrations/<timestamp>_<name>.sql
```

After adding a migration or changing a query, regenerate `.sqlx/` against a migrated scratch database and
commit the result:

```sh
export DATABASE_URL=sqlite://dev.db
sqlx database create
sqlx migrate run
cargo sqlx prepare -- --all-targets
cargo sqlx prepare --check -- --all-targets   # verify .sqlx/ is up to date
```

Tests create an in-memory database (`sqlite::memory:`) and run the migrations, so `cargo test` needs no setup.

## Docker

```sh
docker build -t bagel .
docker run --rm -p 3000:3000 -v bagel-data:/data bagel
```

The image stores the database at `/data/bagel.db` (`DATABASE_URL=sqlite:///data/bagel.db?mode=rwc`); mount a
volume at `/data` to keep notes across container restarts. The directory is owned by the distroless `nonroot`
user (uid 65532), so a bind mount must be writable by that uid.

## CI

`.github/workflows/ci.yml` runs fmt/clippy/tests for Rust (with `SQLX_OFFLINE=true`, using the committed `.sqlx/`) and lint/build for the frontend, then builds the
Docker image. Images are pushed to `ghcr.io/<owner>/bagel` on pushes to `main` (`latest`, `sha-…`) and on
`v*` tags (`1.2.3`, `1.2`). Pull requests build the image without pushing.
