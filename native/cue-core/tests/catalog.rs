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
    let state = view(&catalog, 0).unwrap();
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

#[tokio::test]
async fn content_windows_fetch_remote_pages_without_gaps_or_duplicates() {
    use cue_core::{
        environment::{Environment, NativeEnv},
        session::CoreSession,
    };
    use stremio_core::{
        constants::{PROFILE_STORAGE_KEY, SCHEMA_VERSION, SCHEMA_VERSION_STORAGE_KEY},
        runtime::Env,
        types::profile::Profile,
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        for (request_path, range) in [
            ("/catalog/movie/fixture.json", 0..27),
            ("/catalog/movie/fixture/skip=27.json", 25..30),
            ("/catalog/movie/fixture/skip=32.json", 30..30),
        ] {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0; 4096];
            let count = socket.read(&mut request).await.unwrap();
            assert!(
                std::str::from_utf8(&request[..count])
                    .unwrap()
                    .starts_with(&format!("GET {request_path} "))
            );
            let metas: Vec<_> = range.map(|n| serde_json::json!({"id":format!("film-{n}"),"name":format!("Film {n}"),"type":"movie"})).collect();
            let body = serde_json::json!({"metas":metas}).to_string();
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        }
    });
    let env = Environment::new().unwrap();
    let mut profile = Profile::default();
    profile.addons=vec![serde_json::from_value(serde_json::json!({"transportUrl":format!("http://{address}/manifest.json"),"manifest":{"id":"fixture","name":"Fixture","version":"1.0.0","description":"Fixture","resources":["catalog"],"types":["movie"],"catalogs":[{"id":"fixture","type":"movie","extra":[{"name":"skip"}]}]}})).unwrap()];
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
    session
        .load_catalog(0, session.snapshot().revision)
        .await
        .unwrap();
    for (offset, ids) in [
        (0, (0..24).collect::<Vec<_>>()),
        (24, (24..27).collect()),
        (27, (27..30).collect()),
        (30, vec![]),
        (0, (0..24).collect()),
    ] {
        if offset != 0
            || session
                .snapshot()
                .catalog
                .as_ref()
                .is_some_and(|v| v.offset != 0)
        {
            session
                .show_catalog_items(offset, session.snapshot().revision)
                .await
                .unwrap();
        }
        let state = tokio::time::timeout(std::time::Duration::from_secs(2), session.settled())
            .await
            .unwrap()
            .unwrap();
        let view = state.catalog.unwrap();
        assert_eq!(view.status, "ready");
        assert_eq!(view.offset, offset);
        assert_eq!(
            view.items.iter().map(|i| i.id.clone()).collect::<Vec<_>>(),
            ids.into_iter()
                .map(|i| format!("film-{i}"))
                .collect::<Vec<_>>()
        );
        if offset == 30 {
            assert!(!view.has_more);
        }
    }
    assert!(
        session
            .show_catalog_items(31, session.snapshot().revision)
            .await
            .is_err()
    );
    server.await.unwrap();
    session.stop().await;
}
