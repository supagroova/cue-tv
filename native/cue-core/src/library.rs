//! Bounded presentation of Core library members; no streams or configured URLs.
use serde::Serialize;
use stremio_core::{
    models::meta_details::Selected,
    types::{
        addon::ResourcePath,
        library::{LibraryBucket, LibraryItem},
    },
};
const PAGE_SIZE: usize = 24;
#[derive(Clone, Debug, Serialize)]
pub struct Item {
    pub id: String,
    pub kind: String,
    pub name: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct View {
    pub items: Vec<Item>,
    pub offset: usize,
    pub total: usize,
}
fn members(library: &LibraryBucket) -> Vec<&LibraryItem> {
    let mut items: Vec<_> = library
        .items
        .values()
        .filter(|item| {
            !item.removed
                && !item.temp
                && !item.id.is_empty()
                && item.id.len() <= 512
                && !item.r#type.is_empty()
                && item.r#type.len() <= 64
                && !item.id.chars().any(char::is_control)
                && !item.r#type.chars().any(char::is_control)
        })
        .collect();
    items.sort_by(|a, b| (&a.name, &a.r#type, &a.id).cmp(&(&b.name, &b.r#type, &b.id)));
    items
}
pub fn view(library: &LibraryBucket, offset: usize) -> View {
    let items = members(library);
    let total = items.len();
    let offset = offset.min(total.saturating_sub(1) / PAGE_SIZE * PAGE_SIZE);
    View {
        offset,
        total,
        items: items
            .into_iter()
            .skip(offset)
            .take(PAGE_SIZE)
            .map(|item| Item {
                id: item.id.clone(),
                kind: item.r#type.clone(),
                name: item
                    .name
                    .chars()
                    .filter(|c| !c.is_control())
                    .take(256)
                    .collect(),
            })
            .collect(),
    }
}
pub fn selection(library: &LibraryBucket, offset: usize, index: usize) -> Option<Selected> {
    let item = view(library, offset).items.into_iter().nth(index)?;
    Some(Selected {
        meta_path: ResourcePath::without_extra("meta", &item.kind, &item.id),
        stream_path: None,
        guess_stream: false,
    })
}
