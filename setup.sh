#!/bin/bash

set -e

# kill running docker containers
if [ "$(docker ps -q)" ]; then docker kill $(docker ps -q); fi

# startup the database
docker compose up -d --wait

# install frontend deps and build
pushd frontend
    npm ci
    npm run build || true
popd

# initialize database
cargo sqlx database reset -y
cargo sqlx migrate run

# load fixtures
psql -h localhost -U remails -w remails < src/fixtures/organizations.sql
psql -h localhost -U remails -w remails < src/fixtures/api_users.sql
psql -h localhost -U remails -w remails < src/fixtures/projects.sql
psql -h localhost -U remails -w remails < src/fixtures/runtime_config.sql

# run the application
cargo run --bin app
