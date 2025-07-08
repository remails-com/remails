#!/usr/bin/env bash

set -eo pipefail

echo "--- create new backup"
restic backup -o s3.storage-class=ONEZONE_IA --group-by '' --stdin-filename="$ENVIRONMENT.backup" --stdin-from-command -- /usr/bin/pg_dump --dbname="$DATABASE_URL" --format=custom --exclude-table="messages" --compress=4 --no-password

echo -e "\n--- remove outdated backups"
restic forget --prune --keep-daily 10 --keep-monthly 3 --group-by ''

echo -e "\n--- check repository state"
restic check
