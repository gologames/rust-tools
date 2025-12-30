use anyhow::Result;
use kube::{
    api::{Api, DeleteParams, ListParams, Patch, PatchParams, DynamicObject, ResourceExt},
    core::{ApiResource, GroupVersion, GroupVersionKind},
    Client, Error as KubeError,
};
use serde_json::json;
use std::env;

/// Wipe all objects of a given CRD kind:
/// 1) remove finalizers
/// 2) delete the object
async fn wipe_kind(client: Client, gvk: GroupVersionKind, label: &str) -> Result<()> {
    let ar = ApiResource::from_gvk(&gvk);

    let api_all: Api<DynamicObject> = Api::all_with(client.clone(), &ar);
    let list = match api_all.list(&ListParams::default()).await {
        Ok(list) => list,
        // If API returns 404 (CRD or endpoint missing) - ignore and continue
        Err(KubeError::Api(e)) if e.code == 404 => {
            println!("{label}: API 404 (probably CRD or API missing), skipping");
            return Ok(());
        }
        Err(e) => return Err(e.into()),
    };

    if list.items.is_empty() {
        println!("{label}: none");
        return Ok(());
    }

    for obj in list.items {
        let name = obj.name_any();
        let ns = obj
            .metadata
            .namespace
            .clone()
            .unwrap_or_else(|| "default".to_string());

        let api_ns: Api<DynamicObject> =
            Api::namespaced_with(client.clone(), &ns, &ar);

        // Remove finalizers (best-effort)
        let _ = api_ns
            .patch(
                &name,
                &PatchParams::default(),
                &Patch::Merge(json!({
                    "metadata": { "finalizers": [] }
                })),
            )
            .await;

        // Delete the object (foreground so children are cleaned up)
        let _ = api_ns
            .delete(&name, &DeleteParams::foreground())
            .await;

        println!("{label}: {ns}/{name}");
    }

    Ok(())
}

/// Delete a namespace by name
async fn delete_namespace(client: Client, ns: &str) -> Result<()> {
    let gvk = GroupVersionKind::gvk("", "v1", "Namespace");
    let ar = ApiResource::from_gvk(&gvk);

    let api: Api<DynamicObject> = Api::all_with(client, &ar);
    match api.delete(ns, &DeleteParams::default()).await {
        Ok(_) => println!("Deleted namespace {ns}"),
        // If ns already gone - ok
        Err(KubeError::Api(e)) if e.code == 404 => {
            println!("Namespace {ns} not found (404), skipping");
        }
        Err(e) => return Err(e.into()),
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let ns = env::args()
        .nth(1)
        .expect("USAGE: argocd-cleanup <namespace>");

    let client = Client::try_default().await?;

    let gvk_app =
        GroupVersion::gv("argoproj.io", "v1alpha1").with_kind("Application");
    let gvk_app_set =
        GroupVersion::gv("argoproj.io", "v1alpha1").with_kind("ApplicationSet");
    let gvk_proj =
        GroupVersion::gv("argoproj.io", "v1alpha1").with_kind("AppProject");

    wipe_kind(client.clone(), gvk_app, "Application").await?;
    wipe_kind(client.clone(), gvk_app_set, "ApplicationSet").await?;
    wipe_kind(client.clone(), gvk_proj, "AppProject").await?;

    delete_namespace(client.clone(), &ns).await?;

    println!("Done.");
    Ok(())
}