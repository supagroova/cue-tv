use serde::Serialize;
use std::collections::HashSet;
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
    pub offset: usize,
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
fn valid_items(catalog: &Catalog) -> impl Iterator<Item = &MetaItemPreview> {
    // Deduplicate the presentation only: Core needs raw page lengths for remote skip.
    let mut seen = HashSet::new();
    catalog
        .catalog
        .iter()
        .flat_map(|page| {
            page.content
                .as_ref()
                .and_then(|content| content.ready())
                .into_iter()
                .flatten()
        })
        .filter(|item| item.id.len() <= 512 && item.r#type.len() <= 64)
        .filter(move |item| seen.insert((item.r#type.as_str(), item.id.as_str())))
}
pub fn loaded_count(catalog: &Catalog) -> usize {
    valid_items(catalog).count()
}
pub fn view(catalog: &Catalog, offset: usize) -> Option<View> {
    catalog.selected.as_ref()?;
    let tail = catalog.catalog.last()?;
    let total = loaded_count(catalog);
    let status = if offset < total {
        "ready"
    } else {
        match &tail.content {
            None | Some(Loadable::Loading) => "loading",
            Some(Loadable::Err(ResourceError::EmptyContent)) | Some(Loadable::Ready(_)) => "ready",
            Some(Loadable::Err(_)) => "failed",
        }
    };
    let items: Vec<_> = valid_items(catalog)
        .skip(offset)
        .take(24)
        .map(|item| Item {
            id: item.id.clone(),
            name: item.name.chars().take(128).collect(),
            kind: item.r#type.clone(),
        })
        .collect();
    Some(View {
        status,
        has_more: offset.saturating_add(items.len()) < total
            || catalog.selectable.next_page.is_some(),
        items,
        offset,
    })
}
