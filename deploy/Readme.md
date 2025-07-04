# Prepare the cluster

The following commands are all executed in the `deploy` folder

1. Install cert-manager for the whole cluster with the configured helm values

```shell
helm upgrade --install -f cert-manager-values.yaml -n cert-manager --create-namespace cert-manager jetstack/cert-manager
```

2. Create certificate issuers

```shell
kubectl apply -f cert-issuers.yaml -n cert-manager
```

3. Install the scaleway-certmanager-webhook to allow creating Let's Encrypt certificates using the `DNS-01` ACME
   challenge.
   This is required for generating certificates for the SMTP interface.

```shell
helm repo add scaleway https://helm.scw.cloud/
helm upgrade --install -f scaleway-certmanager-webhook-values.yaml -n cert-manager scaleway-certmanager-webhook scaleway/scaleway-certmanager-webhook \
  --set secret.accessKey=$SCW_ACCESS_KEY \
  --set secret.secretKey=$SCW_SECRET_KEY
```

2. Create a namespace for remails (or possible multiple for different environments)

```shell
kubectl create ns remails-staging
```

3. Create image pull secret (one for each namespace)

```shell
kubectl create secret docker-registry regcred \
  --docker-server=ghcr.io \
  --docker-username=<usename> \
  --docker-password=<access token> \
  --namespace remails-production
```

5. Setup Database

```postgresql
CREATE USER remails_staging WITH PASSWORD 'super-secret' CONNECTION LIMIT 20;
CREATE DATABASE "remails_staging" OWNER "remails_staging";

CREATE USER remails_production WITH PASSWORD 'super-secret' CONNECTION LIMIT 80;
CREATE DATABASE "remails_production" OWNER "remails_production";
```

```shell
cargo sqlx migrate run --database-url=postgres://...
```

# Install Remails

```shell
helm upgrade --install remails ./remails \
            --set images.mta.tag=<defaults to 'dev'> \
            --set images.management.tag=<defaults to 'dev'> \
            --set database_url="${{ secrets.DB_URL }}" \
            --set session_key=<has a default for development> \
            --set github_oauth.client_secret="${{ secrets.OAUTH_CLIENT_SECRET_GITHUB }}" \
            --namespace ${{ vars.namespace }}
```

# Back up and restore Database

1. Set up an S3-compatible bucket in Scaleway
2. Create a restic repository

```shell
export AWS_ACCESS_KEY_ID=$SCW_ACCESS_KEY
export AWS_SECRET_ACCESS_KEY=$SCW_SECRET_KEY
export RESTIC_REPOSITORY=s3:https://s3.fr-par.scw.cloud/remails-backup
restic -o s3.storage-class=ONEZONE_IA init
```

3. Backup

The cluster runs a regular cronjob to automatically create the backups.
See [backup.sh](./backup.sh) and [the job definition](./remails/templates/database-backup-cron-job.yaml) for details.

4. Restore

Keep in mind that the "messages" table does not get backed up, and therefore,
you have to create it manually with the currently applicable schema.

```postgresql
-- on a fresh database, you might need to create the user first
CREATE USER remails_production WITH PASSWORD 'super-secret' CONNECTION LIMIT 80;
CREATE DATABASE "remails_production" OWNER "remails_production";
```

```shell
export AWS_ACCESS_KEY_ID=$SCW_ACCESS_KEY
export AWS_SECRET_ACCESS_KEY=$SCW_SECRET_KEY
export RESTIC_REPOSITORY=s3:https://s3.fr-par.scw.cloud/remails-backup
export RESTIC_PASSWORD=<stored in 1Password>

restic restore latest --target ./backup-restore
pg_restore --dbname="$DATABASE_URL" "$PWD/backup-restore/production.backup"
```