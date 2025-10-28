#!/bin/bash

set -e

if minikube status | grep -q "host: Running"; then
  echo "✅ Minikube host is running."
else
  echo "❌ Minikube host is not running or is in another state. Starting now"
  minikube start
fi

eval "$(minikube docker-env)"
minikube addons enable ingress

cargo build --release

docker build ./target/release -f deploy/ci.Dockerfile -t ghcr.io/tweedegolf/remails/inbound:dev --target inbound
docker build ./target/release -f deploy/ci.Dockerfile -t ghcr.io/tweedegolf/remails/outbound:dev --target outbound
docker build ./target/release -f deploy/ci.Dockerfile -t ghcr.io/tweedegolf/remails/management:dev --target management
docker build ./target/release -f deploy/ci.Dockerfile -t ghcr.io/tweedegolf/remails/periodic:dev --target periodic
docker build ./target/release -f deploy/ci.Dockerfile -t ghcr.io/tweedegolf/remails/migrate_db:dev --target migrate-db
docker build ./target/release -f deploy/ci.Dockerfile -t ghcr.io/tweedegolf/remails/message_bus:dev --target message-bus

helm upgrade --install -f ./deploy/cert-manager-values.yaml -n cert-manager --create-namespace cert-manager jetstack/cert-manager

kubectl apply -f - <<EOF
apiVersion: cert-manager.io/v1
kind: ClusterIssuer
metadata:
  name: letsencrypt-production-scaleway-dns
spec:
  selfSigned: {}
EOF

kubectl patch node minikube -p '{"spec": {"providerID": "minikube:////localhost"}}' || true

DEFAULT_INTERFACE_IP=$(ip route list | awk ' /^default/ {print $9}' | head -n 1)

helm upgrade --install remails ./deploy/remails \
  --set images.inbound.pullPolicy="Never" \
  --set images.outbound.pullPolicy="Never" \
  --set images.management.pullPolicy="Never" \
  --set images.periodic.pullPolicy="Never" \
  --set images.migrate_db.pullPolicy="Never" \
  --set images.message_bus.pullPolicy="Never" \
  --set backup.enabled="false" \
  --set environment="development" \
  --set database_url="postgresql://remails@${DEFAULT_INTERFACE_IP}/remails" \
  --set management.server_name="local.remails.net" \
  --set smtp.server_name="smtp.local.remails.net" \
  --set moneybird.administration="" \
  --set moneybird.api_key="" \
  --set moneybird.webhook_url="" \
  --set smtp.dkim_selector="remails-local" \
  --set "smtp.ports={25}" \
  --namespace default


echo
echo "Please add the following to your /etc/hosts file to be able to communicate with the remails application"
echo "----"
echo "$(minikube ip) local.remails.net"
echo "$(minikube ip) smtp.local.remails.net"
echo "----"
echo
echo "The SMTP port is $(minikube service inbound --format '{{.Port}}' --url)"