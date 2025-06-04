# Install Remails

1. Install cert-manager CRDs
```shell
# replace the version 1.12.17 with the newest available
kubectl apply -f https://github.com/cert-manager/cert-manager/releases/download/v1.12.17/cert-manager.crds.yaml
```
2. Create a namespace for cert-manager
```shell
kubectl create ns cert-manager
```

3. Create a namespace for remails
```shell
kubectl create ns remails
```

4. Create image pull secret
```shell
kubectl create secret generic regcred \
    --from-file=.dockerconfigjson=<path/to/.docker/config.json> \
    --type=kubernetes.io/dockerconfigjson
    --namespace remails
```

