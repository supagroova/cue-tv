//! Redacted title details from Core's metadata model.
use serde::Serialize;
use stremio_core::models::{common::Loadable, meta_details::MetaDetails};
#[derive(Clone, Debug, Serialize)]
pub struct View {
    pub status: &'static str,
    pub id: String,
    pub kind: String,
    pub name: Option<String>,
    pub description: Option<String>,
}
pub fn view(model: &MetaDetails) -> Option<View> {
    let selected = model.selected.as_ref()?;
    let ready = model
        .meta_items
        .iter()
        .filter_map(|page| page.content.as_ref()?.ready())
        .find(|item| {
            item.preview.id == selected.meta_path.id
                && item.preview.r#type == selected.meta_path.r#type
        });
    let loading = model
        .meta_items
        .iter()
        .any(|page| matches!(page.content, None | Some(Loadable::Loading)));
    Some(View {
        status: if ready.is_some() {
            "ready"
        } else if loading {
            "loading"
        } else {
            "failed"
        },
        id: selected.meta_path.id.clone(),
        kind: selected.meta_path.r#type.clone(),
        name: ready.map(|item| item.preview.name.chars().take(256).collect()),
        description: ready
            .and_then(|item| item.preview.description.as_ref())
            .map(|text| text.chars().take(4096).collect()),
    })
}
