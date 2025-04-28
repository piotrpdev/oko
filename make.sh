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
        NODE_ENV=development npm run build -- --mode development

        cd ../$BACKEND_DIR
        mkdir -p static
        cp -r ../$FRONTEND_DIR/dist/* ./static
        cargo run
        ;;
    "t")
        cd $FRONTEND_DIR
        NODE_ENV=development npm run build -- --mode development
        NODE_ENV=development npm run test -- --mode development

        cd ../$BACKEND_DIR
        mkdir -p static
        cp -r ../$FRONTEND_DIR/dist/* ./static
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
        # https://github.com/launchbadge/sqlx/blob/main/sqlx-cli/README.md
        # sqlx database drop
        # sqlx database create
        # sqlx migrate run
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
    "dry_pub")
        cd $BACKEND_DIR
        cargo publish --dry-run --allow-dirty
        ;;
    *)
        echo "Invalid argument"
        exit 1
        ;;
esac
