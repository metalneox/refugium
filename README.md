# Refugium 🐕

**English** | [Italiano](README.it.md)

---

**Refugium** is a platform for publishing **listings of dogs looking for a home**.
Each listing contains the dog's details (name, breed, age, city, description, contact) and a **photo**.

The whole project is written in **Rust**, organised as a monorepo:

- the **backend** is a REST API built with [Axum](https://github.com/tokio-rs/axum);
- the **frontend** is an SPA site built with [Leptos](https://leptos.dev), compiled to WebAssembly;
- data is stored in **SQLite**, images on disk.

---

## Table of contents

- [Architecture](#architecture)
- [Requirements](#requirements)
- [Quick start (development)](#quick-start-development)
- [One-command start (`run.sh`)](#one-command-start-runsh)
- [Production build](#production-build)
- [API](#api)
- [Configuration](#configuration)
- [Useful commands](#useful-commands)

---

## Architecture

```
refugium/
├── Cargo.toml              # Cargo workspace (3 crates)
├── backend/                # 🖥️ Axum API
│   └── src/
│       ├── main.rs         #   server startup, CORS, static file serving
│       ├── routes/         #   endpoints: list, detail, creation of listings
│       ├── db.rs           #   SQLite schema + queries (sqlx)
│       └── error.rs        #   JSON errors ({ "error": "..." })
├── frontend/               # 🌐 Leptos SPA (WASM)
│   ├── Trunk.toml          #   dev server + proxy to the API
│   ├── index.html          #   Trunk entrypoint
│   ├── style.css           #   UI stylesheet
│   └── src/
│       ├── main.rs         #   app mount
│       ├── app.rs          #   UI: photo gallery, «Insert» menu, form with validation
│       └── api.rs          #   fetch calls to the API (via web-sys)
└── shared/                 # 📦 shared types (native + wasm32)
    └── src/lib.rs          #   the Announcement struct used by both
```

**How the three crates cooperate:**

| Crate      | Target                   | Role                                                                 |
| ---------- | ------------------------ | -------------------------------------------------------------------- |
| `backend`  | native (`x86_64`)        | HTTP API, upload validation, SQLite, image serving                   |
| `frontend` | `wasm32-unknown-unknown` | browser interface: gallery, menu, listing form                       |
| `shared`   | both                     | the `Announcement` type shared by API and UI (no duplication)        |

**Lifecycle of a listing:**
the browser fills in the form → multipart POST `fetch` to `/api/announcements` →
the backend validates the fields and the image → saves the photo in `data/uploads/` and the row in SQLite →
replies with the listing JSON → the UI returns to the gallery, where the new photo is already visible.

## Requirements

- **Rust** stable (edition 2024)
- WASM target and Trunk for the frontend:

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
```

## Quick start (development)

**Two terminals** are needed, both from the repository root.

**1. Backend** — API on `http://localhost:3000`:

```sh
cargo run -p backend
```

**2. Frontend** — site on `http://localhost:8080`, with hot reload:

```sh
cd frontend
trunk serve
```

Open **http://localhost:8080**.

> The Trunk dev server automatically forwards `/api/*` and `/uploads/*` to the backend
> (configuration in `frontend/Trunk.toml`): in development **there is no CORS to handle**.
> Visiting `http://localhost:3000` directly already works, because the backend serves
> the last build in `frontend/dist`, if it exists.

## One-command start (`run.sh`)

The `run.sh` script does everything described above with a single command:

```sh
./run.sh
```

What it does:

1. **Checks prerequisites** — `cargo`, `trunk` and the WASM target, printing the
   install command to copy if something is missing;
2. **Checks ports** 3000 and 8080 — if they are busy (e.g. by a service started
   earlier) it offers to free them;
3. **Starts in parallel** the backend and the frontend, showing both logs on screen;
4. **Cleans up on exit** — `Ctrl+C` stops the backend *and* the frontend together,
   leaving no orphan processes. If one service crashes, the script stops the other
   one too and exits reporting the error.

Expected startup output (messages are in Italian):

```
▸ Avvio backend su http://localhost:3000 …
▸ Avvio frontend su http://localhost:8080 …
✓ Refugium in esecuzione:
  • Sito:      http://localhost:8080
  • API:       http://localhost:3000/api/announcements
  • Per uscire: Ctrl+C
```

## Production build

```sh
# 1. Build the frontend into frontend/dist/
cd frontend
trunk build --release

# 2. Build and run the backend: it serves both the API and the site on :3000
cd ..
cargo run --release -p backend
```

In production a **single process** exposes everything: the API on `/api/*`, the images
on `/uploads/*` and the site as the fallback on `/`.

## API

Base URL: `http://localhost:3000`

| Method | Endpoint                  | Description                                      |
| ------ | ------------------------- | ------------------------------------------------ |
| GET    | `/api/announcements`      | List of all listings (newest first)              |
| GET    | `/api/announcements/:id`  | Detail of a single listing                       |
| POST   | `/api/announcements`      | Create a listing (`multipart/form-data`)         |
| GET    | `/health`                 | Health probe: `{"status":"ok"}`                  |

### POST fields (`multipart/form-data`)

| Field         | Type   | Required | Notes                                 |
| ------------- | ------ | -------- | ------------------------------------- |
| `dog_name`    | text   | ✅       | dog's name                            |
| `description` | text   | ✅       | free-text description                 |
| `contact`     | text   | ✅       | phone number or email                 |
| `image`       | file   | ✅       | png/jpeg/webp/gif, max **10 MB**      |
| `breed`       | text   | —        | breed                                 |
| `age`         | text   | —        | age                                   |
| `city`        | text   | —        | city                                  |

### `curl` examples

Creating a listing:

```sh
curl -X POST http://localhost:3000/api/announcements \
  -F 'dog_name=Rocky' \
  -F 'breed=Mutt' \
  -F 'age=3 years' \
  -F 'city=Rome' \
  -F 'description=Sweet and playful, looking for a family.' \
  -F 'contact=info@example.com' \
  -F 'image=@photo.png;type=image/png'
```

`201 Created` response:

```json
{
  "id": "1690ea42-3c5c-4c03-87af-bee02d98ed95",
  "dog_name": "Rocky",
  "breed": "Mutt",
  "age": "3 years",
  "description": "Sweet and playful, looking for a family.",
  "city": "Rome",
  "contact": "info@example.com",
  "image_url": "/uploads/1690ea42-3c5c-4c03-87af-bee02d98ed95.png",
  "created_at": "2026-11-16T18:28:17.879081674+00:00"
}
```

Reading listings:

```sh
curl http://localhost:3000/api/announcements          # list
curl http://localhost:3000/api/announcements/<id>     # detail
```

Typical errors (HTTP 400, JSON body `{ "error": "..." }`; backend messages are in Italian):

```json
{"error":"Il nome del cane è obbligatorio"}
{"error":"Tipo di immagine non supportato: text/plain"}
{"error":"L'immagine supera i 10 MB"}
```

## Configuration

All settings are environment variables with sensible defaults:

| Variable               | Default                              | Description                                  |
| ---------------------- | ------------------------------------ | -------------------------------------------- |
| `REFUGIUM_ADDR`        | `0.0.0.0:3000`                       | backend address and port                     |
| `REFUGIUM_DB_URL`      | `sqlite://data/refugium.db?mode=rwc` | SQLite connection (creates the file if missing) |
| `REFUGIUM_UPLOADS_DIR` | `data/uploads`                       | folder where images are saved                |

Example:

```sh
REFUGIUM_ADDR=0.0.0.0:8081 REFUGIUM_DB_URL=sqlite://./refugium.db?mode=rwc cargo run -p backend
```

Nothing to set up by hand: the database and the upload folder are **created automatically**
on first start, with the schema applied by `backend/src/db.rs`.

## Useful commands

From the workspace root:

```sh
./run.sh                          # start backend + frontend together (development)
cargo build                       # build backend + shared
cargo run -p backend              # run only the API
cargo check -p frontend --target wasm32-unknown-unknown   # quick frontend check
cargo test -p frontend            # unit tests of the form validation
cargo fmt --all                   # format all the code
cargo clippy --all-targets        # linter
cargo clean                       # start from scratch
```

From `frontend/`:

```sh
trunk serve                       # dev server on :8080 with hot reload
trunk build --release             # production build into dist/
trunk clean                       # removes dist/
```

Cleaning test data (database and uploaded photos):

```sh
rm -rf data/
```
