# Prepare the cluster

The following commands are all executed in the `deploy` folder

1. Install cert-manager for the whole cluster with the configured helm values
```shell
helm upgrade --install -f cert-manager-values.yaml -n cert-manager --create-namespace cert-manager jetstack/cert-manager
```

2. Create a namespace for remails (or possible multiple for different environments)
```shell
kubectl create ns remails-staging
```

3. Create image pull secret (one for each namespace)
```shell
kubectl create secret generic regcred \
    --from-file=.dockerconfigjson=<path/to/.docker/config.json> \
    --type=kubernetes.io/dockerconfigjson \
    --namespace remails-staging
```

4. Create certificate issuers
```shell
kubectl apply -f cert-issuers.yaml -n cert-manager
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
            --set scaleway-certmanager-webhook.secret.accessKey="${{ secrets.SCW_ACCESS_KEY }}" \
            --set scaleway-certmanager-webhook.secret.secretKey="${{ secrets.SCW_SECRET_KEY }}" \
            --set github_oauth.client_secret="${{ secrets.OAUTH_CLIENT_SECRET_GITHUB }}" \
            --namespace ${{ vars.namespace }}
```