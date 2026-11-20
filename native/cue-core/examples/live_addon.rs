//! Opt-in public catalogue/metadata probe. Never downloads or plays media.
use cue_core::{
    environment::{Environment, NativeEnv},
    session::{CoreSession, Snapshot},
};
use std::time::{Duration, Instant};
use stremio_core::{
    constants::{PROFILE_STORAGE_KEY, SCHEMA_VERSION, SCHEMA_VERSION_STORAGE_KEY},
    runtime::Env,
    types::profile::Profile,
};

async fn wait(
    session: &CoreSession,
    stage: &'static str,
    accepts: impl Fn(&Snapshot) -> bool,
) -> Result<Snapshot, &'static str> {
    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            let state = session.snapshot();
            if accepts(&state) {
                return state;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .map_err(|_| {
        let state = session.snapshot();
        eprintln!("{}", serde_json::json!({"stage":stage,"pending_effects":state.pending_effects,"addon_status":state.addon.as_ref().map(|a|a.status),"catalogue_status":state.catalog.as_ref().map(|c|c.status),"metadata_status":state.details.as_ref().map(|d|d.status),"error_category":state.error}));
        stage
    })
}

async fn probe(session: &CoreSession) -> Result<serde_json::Value, &'static str> {
    let started = Instant::now();
    session
        .load_addon("https://v3-cinemeta.strem.io/manifest.json")
        .await?;
    let manifest = wait(session, "manifest_timeout", |s| {
        s.pending_effects == 0 && s.addon.as_ref().is_some_and(|a| a.status == "ready")
    })
    .await?;
    if manifest.addon.as_ref().and_then(|a| a.id.as_deref()) != Some("com.linvo.cinemeta") {
        return Err("unexpected_live_addon");
    }
    session.install_addon(manifest.revision).await?;
    let installed = wait(session, "installed_timeout", |s| {
        s.pending_effects == 0 && s.addon_count == 1 && !s.catalogs.is_empty()
    })
    .await?;
    let choice = installed
        .catalogs
        .iter()
        .find(|c| c.kind == "movie")
        .ok_or("live_catalogue_missing")?;
    session
        .load_catalog(choice.index, installed.revision)
        .await?;
    let catalog = wait(session, "catalog_timeout", |s| {
        s.catalog
            .as_ref()
            .is_some_and(|c| c.status == "ready" && !c.items.is_empty())
    })
    .await?;
    let items = &catalog
        .catalog
        .as_ref()
        .ok_or("live_catalogue_missing")?
        .items;
    let identity = (items[0].id.clone(), items[0].kind.clone());
    let item_count = items.len();
    session.load_details(0, catalog.revision).await?;
    let details = wait(session, "details_timeout", |s| {
        s.details.as_ref().is_some_and(|d| d.status == "ready")
    })
    .await?;
    let detail = details.details.as_ref().ok_or("live_metadata_missing")?;
    if (detail.id.as_str(), detail.kind.as_str()) != (identity.0.as_str(), identity.1.as_str())
        || detail.name.as_ref().is_none_or(|s| s.is_empty())
    {
        return Err("live_metadata_identity_mismatch");
    }
    if details.authenticated {
        return Err("unexpected_authenticated_session");
    }
    Ok(
        serde_json::json!({"version":1,"addon":"com.linvo.cinemeta","transport":"https://v3-cinemeta.strem.io/manifest.json","scope":"native host Core HTTP/model integration; not Apple TV or playback qualification","manifest_ready":true,"installed_addons":installed.addon_count,"catalogue_items":item_count,"matching_metadata":true,"authenticated":false,"elapsed_ms":started.elapsed().as_millis()}),
    )
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("CUE_LIVE_ADDON_PROBE").as_deref() != Ok("1") {
        return Err("Set CUE_LIVE_ADDON_PROBE=1 to contact public Cinemeta".into());
    }
    let env = Environment::new().map_err(|_| "live_environment_unavailable")?;
    let mut profile = Profile::default();
    profile.addons.clear();
    env.scope(async {
        NativeEnv::set_storage(SCHEMA_VERSION_STORAGE_KEY, Some(&SCHEMA_VERSION)).await?;
        NativeEnv::set_storage(PROFILE_STORAGE_KEY, Some(&profile)).await
    })
    .await
    .map_err(|_| "live_guest_storage_unavailable")?;
    let mut session = CoreSession::guest(env)
        .await
        .map_err(|_| "live_guest_start_failed")?;
    let result = probe(&session).await;
    session.stop().await;
    println!("{}", serde_json::to_string_pretty(&result?)?);
    Ok(())
}
