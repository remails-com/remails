#!/bin/bash

set -e

# kill running docker containers
if [ "$(docker ps -q)" ]; then docker kill $(docker ps -q); fi

# startup the database
docker compose up -d --wait

# install frontend deps and build
pushd frontend
    npm ci
    npm run build
popd

# initialize database
cargo sqlx database reset -y
cargo sqlx migrate run

# run  the full application
cargo run --bin app --features load-fixtures
