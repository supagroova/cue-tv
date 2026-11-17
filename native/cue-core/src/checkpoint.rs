//! One local checkpoint replaces both Core library buckets atomically. No network effect.
use crate::environment::NativeEnv;
use std::collections::BTreeMap;
use stremio_core::{
    constants::{LIBRARY_RECENT_STORAGE_KEY, LIBRARY_STORAGE_KEY},
    runtime::{EnvError, TryEnvFuture},
    types::library::{LibraryBucket, LibraryBucketRef},
};
pub(crate) fn write(library: &LibraryBucket) -> Result<TryEnvFuture<()>, EnvError> {
    let (recent, older) = library.split_items_by_recent();
    let encode = |items: &[_]| {
        serde_json::to_vec(&LibraryBucketRef::new(&library.uid, items))
            .map_err(|_| EnvError::Serde("Invalid library checkpoint".into()))
    };
    let recent = encode(&recent)?;
    let older = if older.is_empty() {
        None
    } else {
        Some(encode(&older)?)
    };
    Ok(NativeEnv::set_storage_batch(BTreeMap::from([
        (LIBRARY_RECENT_STORAGE_KEY.to_owned(), Some(recent)),
        (LIBRARY_STORAGE_KEY.to_owned(), older),
    ])))
}
