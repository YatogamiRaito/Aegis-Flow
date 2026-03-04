# Specification: Production Deployment Hardening (v0.39.0)

## Overview
Fills the gaps identified in the "Production-Ready Deployment" audit by genuinely implementing External Secrets Operator (ESO) templates and adding Multi-Cluster/Service Mesh support to the Aegis-Flow Helm chart.

## Functional Requirements

### 1. External Secrets Operator (ESO) Integration
- Create `SecretStore` and `ClusterSecretStore` Custom Resource Definitions (CRDs) templates configurable via `values.yaml`.
- Create `ExternalSecret` templates to fetch TLS certificates and configuration secrets automatically from AWS Secrets Manager, GCP Secret Manager, or HashiCorp Vault.
- Update `deployment.yaml` to ensure it can mount the dynamically created Kubernetes Secrets seamlessly.

### 2. Multi-Cluster & Service Mesh Support
- Add Multi-Cluster Ingress (MCI) configuration options to `values.yaml`.
- Add explicit Service Mesh sidecar injection options (e.g., `podAnnotations: { "sidecar.istio.io/inject": "true" }` or Linkerd equivalents).
- Add support for cross-cluster DNS and global load balancing annotations on Services.

## Acceptance Criteria
- [ ] `values.yaml` explicitly disables ESO by default but provides a robust structure to enable it for different providers (AWS, GCP, Vault).
- [ ] When `externalSecrets.enabled = true`, the chart renders valid `SecretStore` and `ExternalSecret` resources instead of standard `Secret` resources.
- [ ] Helm tests or linting process confirms the CRDs render correctly without errors (`helm template` works as expected).
- [ ] Service Mesh and Edge/Global Load Balancer configuration documentation and values are verified.
