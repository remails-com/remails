#!/bin/bash

# install frontend deps and build
pushd frontend
    npm ci
    npm run build
popd

# startup and initialize the database
docker compose up -d --wait
cargo sqlx database reset -y
cargo sqlx migrate run

# run  the full application
cargo run --bin app
