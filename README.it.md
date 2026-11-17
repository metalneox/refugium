# Refugium 🐕

[English](README.md) | **Italiano**

---

**Refugium** è una piattaforma per pubblicare **annunci di cani in cerca di una casa**.
Ogni annuncio contiene i dati del cane (nome, razza, età, città, descrizione, contatto) e una **foto**.

L'intero progetto è scritto in **Rust**, organizzato come monorepo:

- il **backend** è una API REST costruita con [Axum](https://github.com/tokio-rs/axum);
- il **frontend** è un sito SPA costruito con [Leptos](https://leptos.dev), compilato in WebAssembly;
- i dati sono salvati in **SQLite**, le immagini su disco.

---

## Indice

- [Architettura](#architettura)
- [Requisiti](#requisiti)
- [Avvio rapido (sviluppo)](#avvio-rapido-sviluppo)
- [Avvio con un solo comando (`run.sh`)](#avvio-con-un-solo-comando-runsh)
- [Build di produzione](#build-di-produzione)
- [API](#api)
- [Configurazione](#configurazione)
- [Comandi utili](#comandi-utili)

---

## Architettura

```
refugium/
├── Cargo.toml              # workspace Cargo (3 crate)
├── backend/                # 🖥️ API Axum
│   └── src/
│       ├── main.rs         #   avvio server, CORS, serving statico
│       ├── routes/         #   endpoint: elenco, dettaglio, creazione annunci
│       ├── db.rs           #   schema SQLite + query (sqlx)
│       └── error.rs        #   errori JSON ({ "error": "..." })
├── frontend/               # 🌐 SPA Leptos (WASM)
│   ├── Trunk.toml          #   dev server + proxy verso l'API
│   ├── index.html          #   entrypoint di Trunk
│   ├── style.css           #   stile dell'interfaccia
│   └── src/
│       ├── main.rs         #   mount dell'app
│       ├── app.rs          #   UI: galleria foto, menu «Inserisci», form con validazioni
│       └── api.rs          #   chiamate fetch all'API (via web-sys)
└── shared/                 # 📦 tipi condivisi (nativo + wasm32)
    └── src/lib.rs          #   struct Announcement usata da entrambi
```

**Come collaborano i tre crate:**

| Crate    | Target                  | Ruolo                                                             |
| -------- | ----------------------- | ----------------------------------------------------------------- |
| `backend`  | nativo (`x86_64`)       | API HTTP, validazione upload, SQLite, serving delle immagini      |
| `frontend` | `wasm32-unknown-unknown` | interfaccia nel browser: galleria, menu, form degli annunci         |
| `shared`   | entrambi                | il tipo `Announcement` identico su API e UI (niente duplicazioni) |

**Flusso di un annuncio:**
il browser compila il form → `fetch` POST multipart verso `/api/announcements` →
il backend valida i campi e l'immagine → salva la foto in `data/uploads/` e la riga in SQLite →
risponde con il JSON dell'annuncio → la UI torna alla galleria, dove la nuova foto è già visibile.

## Requisiti

- **Rust** stable (si usa l'edizione 2024)
- Target WASM e Trunk per il frontend:

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
```

## Avvio rapido (sviluppo)

Servono **due terminali**, entrambi dalla radice del repository.

**1. Backend** — API su `http://localhost:3000`:

```sh
cargo run -p backend
```

**2. Frontend** — sito su `http://localhost:8080`, con ricaricamento automatico:

```sh
cd frontend
trunk serve
```

Apri **http://localhost:8080**.

> Il dev server di Trunk inoltra automaticamente `/api/*` e `/uploads/*` al backend
> (configurazione in `frontend/Trunk.toml`): in sviluppo **non serve gestire il CORS**.
> La prima visita a `http://localhost:3000` funziona già, perché il backend serve
> l'ultima build presente in `frontend/dist` se esiste.

## Avvio con un solo comando (`run.sh`)

Lo script `run.sh` fa tutto quello descritto sopra con un comando:

```sh
./run.sh
```

Cosa fa:

1. **Verifica i prerequisiti** — `cargo`, `trunk` e il target WASM, con il comando
   di installazione da copiare se qualcosa manca;
2. **Controlla le porte** 3000 e 8080 — se sono occupate (per esempio da un
   servizio avviato in precedenza) propone di liberarle;
3. **Avvia in parallelo** backend e frontend, con i log di entrambi a video;
4. **Pulisce alla chiusura** — con `Ctrl+C` ferma backend *e* frontend insieme,
   senza lasciare processi orfani. Se un servizio crasha, lo script ferma
   anche l'altro ed esce segnalando l'errore.

Output atteso all'avvio:

```
▸ Avvio backend su http://localhost:3000 …
▸ Avvio frontend su http://localhost:8080 …
✓ Refugium in esecuzione:
  • Sito:      http://localhost:8080
  • API:       http://localhost:3000/api/announcements
  • Per uscire: Ctrl+C
```

## Build di produzione

```sh
# 1. Compila il frontend in frontend/dist/
cd frontend
trunk build --release

# 2. Compila e avvia il backend: serve sia l'API sia il sito su :3000
cd ..
cargo run --release -p backend
```

In produzione un **solo processo** espone tutto: l'API su `/api/*`, le immagini su
`/uploads/*` e il sito come fallback su `/`.

## API

Base URL: `http://localhost:3000`

| Metodo | Endpoint                   | Descrizione                             |
| ------ | -------------------------- | --------------------------------------- |
| GET    | `/api/announcements`       | Elenco di tutti gli annunci (dal più recente) |
| GET    | `/api/announcements/:id`   | Dettaglio di un singolo annuncio        |
| POST   | `/api/announcements`       | Crea un annuncio (`multipart/form-data`) |
| GET    | `/health`                  | Probe di stato: `{"status":"ok"}`       |

### Campi del POST (`multipart/form-data`)

| Campo         | Tipo     | Obbligatorio | Note                                  |
| ------------- | -------- | ------------ | ------------------------------------- |
| `dog_name`    | testo    | ✅           | nome del cane                         |
| `description` | testo    | ✅           | descrizione libera                    |
| `contact`     | testo    | ✅           | telefono o email                      |
| `image`       | file     | ✅           | png/jpeg/webp/gif, max **10 MB**      |
| `breed`       | testo    | —            | razza                                 |
| `age`         | testo    | —            | età                                   |
| `city`        | testo    | —            | città                                 |

### Esempi con `curl`

Creare un annuncio:

```sh
curl -X POST http://localhost:3000/api/announcements \
  -F 'dog_name=Rocky' \
  -F 'breed=Meticcio' \
  -F 'age=3 anni' \
  -F 'city=Roma' \
  -F 'description=Dolce e giocoso, cerca una famiglia.' \
  -F 'contact=info@example.com' \
  -F 'image=@foto.png;type=image/png'
```

Risposta `201 Created`:

```json
{
  "id": "1690ea42-3c5c-4c03-87af-bee02d98ed95",
  "dog_name": "Rocky",
  "breed": "Meticcio",
  "age": "3 anni",
  "description": "Dolce e giocoso, cerca una famiglia.",
  "city": "Roma",
  "contact": "info@example.com",
  "image_url": "/uploads/1690ea42-3c5c-4c03-87af-bee02d98ed95.png",
  "created_at": "2026-11-16T18:28:17.879081674+00:00"
}
```

Leggere gli annunci:

```sh
curl http://localhost:3000/api/announcements          # elenco
curl http://localhost:3000/api/announcements/<id>     # dettaglio
```

Errori tipici (HTTP 400, corpo JSON `{ "error": "..." }`):

```json
{"error":"Il nome del cane è obbligatorio"}
{"error":"Tipo di immagine non supportato: text/plain"}
{"error":"L'immagine supera i 10 MB"}
```

## Configurazione

Tutte le impostazioni sono variabili d'ambiente con valori predefiniti sensati:

| Variabile              | Predefinito                     | Descrizione                        |
| ---------------------- | ------------------------------- | ---------------------------------- |
| `REFUGIUM_ADDR`        | `0.0.0.0:3000`                  | indirizzo e porta del backend      |
| `REFUGIUM_DB_URL`      | `sqlite://data/refugium.db?mode=rwc` | connessione SQLite (crea il file se manca) |
| `REFUGIUM_UPLOADS_DIR` | `data/uploads`                  | cartella dove salvare le immagini  |

Esempio:

```sh
REFUGIUM_ADDR=0.0.0.0:8081 REFUGIUM_DB_URL=sqlite://./refugium.db?mode=rwc cargo run -p backend
```

Niente da configurare a mano: database e cartella upload vengono **creati automaticamente**
al primo avvio, con lo schema applicato da `backend/src/db.rs`.

## Comandi utili

Dalla radice del workspace:

```sh
./run.sh                          # avvia backend + frontend insieme (sviluppo)
cargo build                       # compila backend + shared
cargo run -p backend              # avvia solo l'API
cargo check -p frontend --target wasm32-unknown-unknown   # verifica rapida del frontend
cargo test -p frontend            # test unitari della validazione del form
cargo fmt --all                   # formatta tutto il codice
cargo clippy --all-targets        # linter
cargo clean                       # riparte da zero
```

Dal `frontend/`:

```sh
trunk serve                       # dev server su :8080 con hot reload
trunk build --release             # build di produzione in dist/
trunk clean                       # rimuove dist/
```

Pulizia dei dati di test (database e foto caricate):

```sh
rm -rf data/
```
