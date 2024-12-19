#!/usr/bin/env bash

set -Eeuo pipefail
cd "$(dirname "$0")"

BACKEND_DIR="backend"
FRONTEND_DIR="frontend"

case "$1" in
    "f")
        cd $FRONTEND_DIR
        npm run dev
        ;;
    "b")
        cd $BACKEND_DIR
        cargo run
        ;;
    "run")
        cd $FRONTEND_DIR
        npm run build

        cd ../$BACKEND_DIR
        cargo run
        ;;
    "test")
        cd $FRONTEND_DIR
        npm run build
        npm run test

        cd ../$BACKEND_DIR
        cargo test
        ;;
    "coverage")
        cd $BACKEND_DIR
        cargo tarpaulin --out Html
        python3 -m http.server
        # xdg-open http://localhost:8000/tarpaulin-report.html
        ;;
    "seed")
        cd $BACKEND_DIR
        cat fixtures/*.sql | sqlite3 data.db
        ;;
    "cam")
        cd $BACKEND_DIR
        cargo run -p camera-impersonator $2 $3 $4
        ;;
    "cam1")
        cd $BACKEND_DIR
        cargo run -p camera-impersonator 80 40000 ./videos/1.mp4
        ;;
    "cam2")
        cd $BACKEND_DIR
        cargo run -p camera-impersonator 80 40001 ./videos/2.mp4
        ;;
    *)
        echo "Invalid argument"
        exit 1
        ;;
esac
