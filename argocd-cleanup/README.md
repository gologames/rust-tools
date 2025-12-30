# argocd-cleanup

A small Rust CLI utility to force-remove Argo CD custom resources from a Kubernetes cluster and delete a target namespace.  
Useful when Argo CD resources or namespaces are stuck in `Terminating` due to finalizers.

## What it does

Targets Argo CD CRDs:

- Application (`argoproj.io/v1alpha1`)
- ApplicationSet (`argoproj.io/v1alpha1`)
- AppProject (`argoproj.io/v1alpha1`)

For each resource kind, the tool:

1. Lists objects across all namespaces
2. Removes `metadata.finalizers` (best-effort)
3. Deletes the object using foreground propagation

After that, it deletes the namespace provided as an argument.

Resource types that are not installed in the cluster are skipped.

## Requirements

- Access to a Kubernetes cluster (via kubeconfig or in-cluster config)
- RBAC permissions for:
  - list / patch / delete on `applications`, `applicationsets`, `appprojects`
  - delete on `namespaces`

## Install

### Build from source

    cd argocd-cleanup
    cargo build --release

Binary will be located at:

    target/release/argocd-cleanup

## Usage

    argocd-cleanup <namespace>

Example:

    argocd-cleanup argocd

## Notes / Safety

- This tool is destructive.
- Cleanup is currently cluster-wide for Argo CD resources.
- Finalizer removal is best-effort.
- Deletion uses foreground propagation.
