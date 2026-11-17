use serde::Serialize;
use stremio_core::{
    models::{
        catalog_with_filters::{CatalogWithFilters, Selected},
        common::{Loadable, ResourceError},
        ctx::Ctx,
    },
    types::{
        addon::{ResourcePath, ResourceRequest},
        resource::MetaItemPreview,
    },
};
pub type Catalog = CatalogWithFilters<MetaItemPreview>;
#[derive(Clone, Debug, Serialize)]
pub struct Choice {
    pub index: usize,
    pub name: String,
    pub kind: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub kind: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct View {
    pub status: &'static str,
    pub items: Vec<Item>,
    pub has_more: bool,
}
pub fn choices(ctx: &Ctx, offset: usize) -> (Vec<Choice>, usize) {
    let all: Vec<_> = ctx
        .profile
        .addons
        .iter()
        .flat_map(|a| &a.manifest.catalogs)
        .enumerate()
        .collect();
    (
        all.iter()
            .skip(offset)
            .take(16)
            .map(|(index, c)| Choice {
                index: *index,
                name: c
                    .name
                    .as_deref()
                    .unwrap_or(&c.id)
                    .chars()
                    .take(128)
                    .collect(),
                kind: c.r#type.chars().take(32).collect(),
            })
            .collect(),
        all.len(),
    )
}
pub fn selection(ctx: &Ctx, index: usize) -> Option<Selected> {
    let (addon, catalog) = ctx
        .profile
        .addons
        .iter()
        .flat_map(|addon| {
            addon
                .manifest
                .catalogs
                .iter()
                .map(move |catalog| (addon, catalog))
        })
        .nth(index)?;
    if !catalog.is_extra_supported(&[]) {
        return None;
    }
    Some(Selected {
        request: ResourceRequest::new(
            addon.transport_url.clone(),
            ResourcePath::without_extra("catalog", &catalog.r#type, &catalog.id),
        ),
    })
}
pub fn view(catalog: &Catalog) -> Option<View> {
    catalog.selected.as_ref()?;
    let page = catalog.catalog.first()?;
    Some(match &page.content {
        None | Some(Loadable::Loading) => View {
            status: "loading",
            items: vec![],
            has_more: false,
        },
        Some(Loadable::Err(ResourceError::EmptyContent)) => View {
            status: "ready",
            items: vec![],
            has_more: false,
        },
        Some(Loadable::Err(_)) => View {
            status: "failed",
            items: vec![],
            has_more: false,
        },
        Some(Loadable::Ready(items)) => View {
            status: "ready",
            items: items
                .iter()
                .filter(|item| item.id.len() <= 512 && item.r#type.len() <= 64)
                .take(24)
                .map(|item| Item {
                    id: item.id.clone(),
                    name: item.name.chars().take(128).collect(),
                    kind: item.r#type.clone(),
                })
                .collect(),
            has_more: items.len() > 24 || catalog.selectable.next_page.is_some(),
        },
    })
}
