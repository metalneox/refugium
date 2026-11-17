#!/usr/bin/env bash
#
# Refugium — avvio in sviluppo con un solo comando.
#
# Lancia in parallelo:
#   - backend (Axum)   su http://localhost:3000
#   - frontend (Trunk) su http://localhost:8080
#
# Ctrl+C (o l'arresto di un servizio) ferma entrambi.

set -euo pipefail

cd "$(dirname "$0")"

BACKEND_PORT=3000
FRONTEND_PORT=8080

info() { printf '\033[1;34m▸\033[0m %s\n' "$1"; }
ok()   { printf '\033[1;32m✓\033[0m %s\n' "$1"; }
warn() { printf '\033[1;33m⚠\033[0m %s\n' "$1"; }
fail() { printf '\033[1;31m✗\033[0m %s\n' "$1" >&2; }

# --- Prerequisiti -----------------------------------------------------------

if ! command -v cargo >/dev/null 2>&1; then
    fail "cargo non trovato: installa Rust (https://rustup.rs)."
    exit 1
fi

if ! command -v trunk >/dev/null 2>&1; then
    fail "trunk non trovato."
    echo "   Installalo con: cargo install trunk --locked" >&2
    exit 1
fi

if ! rustup target list --installed 2>/dev/null | grep -q wasm32-unknown-unknown; then
    fail "Manca il target WASM per il frontend."
    echo "   Installalo con: rustup target add wasm32-unknown-unknown" >&2
    exit 1
fi

# --- Porte occupate ---------------------------------------------------------

pid_on_port() {
    ss -tlnp 2>/dev/null | grep ":$1 " | grep -o 'pid=[0-9]*' | head -1 | cut -d= -f2
}

free_port() {
    local port=$1 pid
    pid=$(pid_on_port "$port" || true)
    [[ -z "$pid" ]] && return 0

    warn "La porta $port è occupata (PID $pid — forse un servizio avviato prima)."
    read -rp "  La libero? [y/N] " confirm
    if [[ "$confirm" =~ ^[Yy]$ ]]; then
        kill "$pid" 2>/dev/null || true
        sleep 1
        ok "Porta $port liberata"
    else
        fail "Impossibile continuare con la porta $port occupata."
        exit 1
    fi
}

free_port "$BACKEND_PORT"
free_port "$FRONTEND_PORT"

# --- Avvio ------------------------------------------------------------------

PIDS=()
INTERRUPTED=0

cleanup() {
    INTERRUPTED=1
    trap - EXIT INT TERM
    local pid
    for pid in "${PIDS[@]:-}"; do
        [[ -n "$pid" ]] && kill "$pid" 2>/dev/null || true
    done
    wait 2>/dev/null || true
}
trap cleanup EXIT INT TERM

info "Avvio backend su http://localhost:$BACKEND_PORT …"
cargo run -p backend &
PIDS+=("$!")

info "Avvio frontend su http://localhost:$FRONTEND_PORT …"
cd frontend
trunk serve &
PIDS+=("$!")

ok "Refugium in esecuzione:"
echo "  • Sito:      http://localhost:$FRONTEND_PORT"
echo "  • API:       http://localhost:$BACKEND_PORT/api/announcements"
echo "  • Per uscire: Ctrl+C"

# Attende il PRIMO servizio che termina: se il backend crasha (o Ctrl+C),
# il trap pulisce l'altro processo e lo script esce.
wait -n
rc=$?

if [[ "$INTERRUPTED" -eq 1 ]]; then
    ok "Servizi fermati."
    exit 0
fi

fail "Un servizio si è fermato inaspettatamente (exit $rc)."
exit "$rc"
