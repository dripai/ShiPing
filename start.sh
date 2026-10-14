#!/usr/bin/env bash

set -euo pipefail

usage() {
    echo "Usage: ./start.sh [dev|build] [shiping] (default: dev)" >&2
}

if (( $# > 2 )); then
    usage
    exit 2
fi

action="${1:-dev}"
app="${2:-shiping}"

case "$app" in
    shiping)
        bin="ShiPing"
        ;;
    *)
        echo "Unknown application: $app" >&2
        usage
        exit 2
        ;;
esac

cd -- "$(dirname -- "${BASH_SOURCE[0]}")"

case "$action" in
    dev | build)
        ;;
    *)
        echo "Unknown action: $action" >&2
        usage
        exit 2
        ;;
esac

case "$action" in
    dev)
        exec cargo run --bin "$bin"
        ;;
    build)
        exec cargo build --release --bin "$bin"
        ;;
esac
