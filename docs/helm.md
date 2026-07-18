# Kubernetes / Helm Deployment Guide: Agent Governance Console

## Overview

`helm/agent-governance-console` deploys `agc-api` to Kubernetes:
Deployment, Service, and optionally Ingress, a HorizontalPodAutoscaler,
a PersistentVolumeClaim for the audit log, a ConfigMap for governance
policies, RBAC env wiring for both HMAC and Entra ID modes, and Azure
Workload Identity annotations for AKS. The root `Dockerfile` builds the
image the chart deploys.

No public image is published by this project yet -- build and push your
own from the repo root:

```bash
docker build -t <your-registry>/agent-governance-console:0.12.0 .
docker push <your-registry>/agent-governance-console:0.12.0
```

---

## Quick start

```bash
helm install agc helm/agent-governance-console \
  --set image.repository=<your-registry>/agent-governance-console \
  --set image.tag=0.12.0

kubectl port-forward svc/agc-agent-governance-console 8080:80
open http://127.0.0.1:8080/dashboard
```

## A real bug this chart's own build found

`agc-api`'s default bind address is `127.0.0.1:8080` -- correct for the
"run it yourself locally" model the README describes, but **unreachable
through Docker's port mapping or a Kubernetes Service/probe**, both of
which connect to the container's external interface, not its loopback.
Fixed with a new `AGC_BIND` env var (`agc-api/src/main.rs`), set to
`0.0.0.0:8080` by default inside the container image (`Dockerfile`'s
`ENV AGC_BIND=0.0.0.0:8080`). Found by actually building the image,
running it, and hitting it through a real port mapping -- not by reading
the code.

---

## Key values

See `helm/agent-governance-console/values.yaml` for the full set with
inline comments; the ones worth knowing about:

| Value | Default | Purpose |
|-------|---------|---------|
| `image.repository` / `image.tag` | `agent-governance-console` / chart `appVersion` | Your pushed image |
| `service.type` | `ClusterIP` | `LoadBalancer`/`NodePort` for direct external access |
| `ingress.enabled` | `false` | Set `true` + `ingress.hosts` for an Ingress resource |
| `persistence.enabled` | `false` | `true` mounts a PVC at `AGC_AUDIT_DB_DIR` -- without it, every tenant's audit log is lost on pod restart |
| `policies` | `{}` | Policy YAML files as inline map values -- mounted read-only, wired to `AGC_POLICY_DIR` (hot-reloads on ConfigMap update + a kubelet sync, same as any ConfigMap volume) |
| `rbac.mode` | `""` (disabled) | `"hmac"` (+ `rbac.hmac.existingSecret`, a Secret you create yourself with a `jwt-secret` key) or `"aad"` (+ `rbac.aad.tenantId`) |
| `telemetry.enabled` | `false` | OTLP export; `telemetry.managedIdentity` / `telemetry.managedIdentityClientId` for Managed-Identity-authenticated export, see `docs/azure_integration.md` |
| `workloadIdentity.clientId` | `""` | Set for AKS Azure Workload Identity: annotates the ServiceAccount and labels the pod so Managed Identity token requests resolve with no client secret, see [Microsoft's Workload Identity docs](https://learn.microsoft.com/azure/aks/workload-identity-overview) |
| `autoscaling.enabled` | `false` | HorizontalPodAutoscaler on CPU utilization |

Both liveness and readiness probes hit `GET /health` (lightweight,
in-process, no DB/network round-trip).

---

## What's verified vs. not

This chart was verified for real, not just written and assumed correct
-- every step below actually ran in this environment (Colima + k3s +
Docker, installed for this purpose; none of docker/colima/helm/kubectl
were present beforehand):

1. `helm lint` -- clean (one informational "icon is recommended" note).
2. `helm template` with every conditional path exercised at once (ingress, autoscaling, persistence, the policy ConfigMap, both RBAC modes, Azure Workload Identity, `extraEnv`) -- confirmed every resource renders.
3. `kubectl apply --dry-run=server` against a real k3s API server, both with default values and with every feature enabled -- the API server itself accepted every generated manifest (ServiceAccount, ConfigMap, PVC, Service, Deployment, HPA, Ingress).
4. A genuine `docker build` of the root `Dockerfile` -- this is what caught the `AGC_BIND` bug above; the first build attempt also failed outright (a dependency required a newer Cargo edition than `rust:1.82-bookworm` shipped), fixed by bumping the base image to `rust:1.90-bookworm`.
5. A real `helm install` of that image into a local k3s cluster: the pod reached `1/1 Ready` (both probes passing against real `/health` responses), and `kubectl port-forward` through the real Service confirmed `/health` and `/dashboard` both work end-to-end through the whole chain (Service -> pod -> container -> `agc-api`).

Not verified: a real cloud Kubernetes cluster (AKS, EKS, GKE) end-to-end, a real Ingress controller routing actual DNS traffic, real Azure Workload Identity token issuance (the annotation/label shape is correct per Microsoft's documented contract, but no AKS cluster with a federated identity was available to test the full token exchange), and image registry push/pull (no image was actually pushed to a remote registry in this verification -- the local k3s cluster used the Docker-built image directly via a `ctr images import`).
