# Prepare the cluster

The following commands are all executed in the `deploy` folder

1. Install cert-manager for the whole cluster with the configured helm values

```shell
helm upgrade --install -f cert-manager-values.yaml -n cert-manager --create-namespace cert-manager jetstack/cert-manager
```

2. Install remaining one-time cluster resources (needs to execute in "cert-manager" namespace, see https://github.com/scaleway/helm-charts/issues/37)
```shell
helm upgrade --install cluster-setup ./cluster-setup \
  --namespace cert-manager \
  --set scaleway-certmanager-webhook.secret.accessKey=$SCW_ACCESS_KEY \
  --set scaleway-certmanager-webhook.secret.secretKey=$SCW_SECRET_KEY \
  --set-json 'docker=[{"username": "$GITHUB_USERNAME", "password": "$GITHUB_TOKEN", "url": "ghcr.io"}]'
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

Please check out the GitHub CI to see the currently used Helm install command: [`deploy-k8s.yml`](../.github/workflows/deploy-k8s.yml).

# Install Grafana
We are using Grafana Cloud (https://remails.grafana.net), so we need to push logs and metrics to their cloud infrastructure.
`grafana-cloud-values.yaml` holds the values we are currently using but this [webpage](https://remails.grafana.net/a/grafana-k8s-app/configuration/cluster-config?from=now-1h&to=now&timezone=browser&var-cluster=remails&var-cardinalityDs=grafanacloud-cardinality-management&var-cardinalityCluster=%24__all&var-cardinalityNamespace=%24__all&var-nonamespaced=%7C%28%29)
is extremely helpful to generate a new configuration in case we ever want to change something.
Tokens can be listed and generated [here](https://remails.grafana.net/a/grafana-auth-app)

```shell
helm repo add grafana https://grafana.github.io/helm-charts
helm repo update
export ACCESS_POLICY_TOKEN=<your token>
helm upgrade --install --atomic --timeout 300s grafana-k8s-monitoring grafana/k8s-monitoring \
    --namespace "metrics" \
    --create-namespace \
    -f grafana-cloud-values.yaml \
    --set destinations[0].auth.password=$ACCESS_POLICY_TOKEN \
    --set destinations[1].auth.password=$ACCESS_POLICY_TOKEN \
    --set alloy-metrics.remoteConfig.auth.password=$ACCESS_POLICY_TOKEN \
    --set alloy-singleton.remoteConfig.auth.password=$ACCESS_POLICY_TOKEN \
    --set alloy-logs.remoteConfig.auth.password=$ACCESS_POLICY_TOKEN
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
