use cue_core::catalog::{Catalog, view};
use stremio_core::models::{
    catalog_with_filters::Selected,
    common::{Loadable, ResourceError, ResourceLoadable},
};

#[test]
fn an_empty_catalogue_is_a_successful_empty_result() {
    let selected: Selected = serde_json::from_value(serde_json::json!({"request":{"base":"https://example.test/manifest.json","path":{"resource":"catalog","type":"movie","id":"fixture","extra":[]}}})).unwrap();
    let mut catalog = Catalog::default();
    catalog.catalog.push(ResourceLoadable {
        request: selected.request.clone(),
        content: Some(Loadable::Err(ResourceError::EmptyContent)),
    });
    catalog.selected = Some(selected);
    let state = view(&catalog).unwrap();
    assert_eq!(state.status, "ready");
    assert!(state.items.is_empty());
    assert!(!state.has_more);
}

#[tokio::test]
async fn all_installed_catalogues_are_reachable_through_bounded_choice_windows() {
    use cue_core::{
        environment::{Environment, NativeEnv},
        session::CoreSession,
    };
    use stremio_core::{
        constants::{PROFILE_STORAGE_KEY, SCHEMA_VERSION, SCHEMA_VERSION_STORAGE_KEY},
        runtime::Env,
        types::profile::Profile,
    };
    let env = Environment::new().unwrap();
    let mut profile = Profile::default();
    profile.addons.truncate(1);
    let template = profile.addons[0].manifest.catalogs[0].clone();
    profile.addons[0].manifest.catalogs = (0..35)
        .map(|index| {
            let mut catalog = template.clone();
            catalog.id = format!("catalog-{index}");
            catalog.name = Some(format!("Catalogue {index}"));
            catalog
        })
        .collect();
    env.scope(async {
        NativeEnv::set_storage(SCHEMA_VERSION_STORAGE_KEY, Some(&SCHEMA_VERSION))
            .await
            .unwrap();
        NativeEnv::set_storage(PROFILE_STORAGE_KEY, Some(&profile))
            .await
            .unwrap();
    })
    .await;
    let mut session = CoreSession::guest(env).await.unwrap();
    let first = session.snapshot();
    assert_eq!(first.catalog_total, 35);
    assert_eq!(first.catalogs.len(), 16);
    session
        .show_catalog_choices(16, first.revision)
        .await
        .unwrap();
    let second = session.snapshot();
    assert_eq!(second.catalog_choices_offset, 16);
    assert_eq!(second.catalogs[0].index, 16);
    assert_eq!(second.catalogs[0].name, "Catalogue 16");
    session
        .show_catalog_choices(32, second.revision)
        .await
        .unwrap();
    let last = session.snapshot();
    assert_eq!(last.catalogs.len(), 3);
    assert_eq!(last.catalogs[2].index, 34);
    assert!(
        session
            .show_catalog_choices(35, last.revision)
            .await
            .is_err()
    );
    assert!(
        session
            .show_catalog_choices(0, first.revision)
            .await
            .is_err()
    );
    session
        .show_catalog_choices(0, last.revision)
        .await
        .unwrap();
    assert_eq!(session.snapshot().catalogs[0].index, 0);
    session.stop().await;
}
