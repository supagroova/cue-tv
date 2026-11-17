//! Versioned authenticated storage for an app-owned directory. Keys are supplied
//! by the platform credential store, never generated beside the encrypted file.
use ring::{
    aead,
    rand::{SecureRandom, SystemRandom},
};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
const HEADER: &[u8; 8] = b"CUECORE1";
const LIMIT: usize = 16 * 1024 * 1024;
const OVERHEAD: usize = 8 + 12 + 16;
pub type StorageValues = BTreeMap<String, Vec<u8>>;
#[derive(Debug, PartialEq, Eq)]
pub enum StoreError {
    Unavailable,
    Corrupt,
    UnsupportedVersion,
    TooLarge,
}
pub struct EncryptedStore {
    directory: PathBuf,
    key: aead::LessSafeKey,
}
impl EncryptedStore {
    /// Blocking: call off the UI actor. Directory must already exist and be owned
    /// exclusively by this application. Callers serialize all access to a store.
    pub fn open(directory: &Path, key: &[u8; 32]) -> Result<(Self, StorageValues), StoreError> {
        let metadata = fs::symlink_metadata(directory).map_err(|_| StoreError::Unavailable)?;
        if !metadata.file_type().is_dir() {
            return Err(StoreError::Unavailable);
        }
        let key =
            aead::UnboundKey::new(&aead::AES_256_GCM, key).map_err(|_| StoreError::Unavailable)?;
        let store = Self {
            directory: directory.to_owned(),
            key: aead::LessSafeKey::new(key),
        };
        let path = directory.join("state.bin");
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok((store, BTreeMap::new()));
            }
            Err(_) => return Err(StoreError::Unavailable),
        };
        if !metadata.file_type().is_file() {
            return Err(StoreError::Unavailable);
        }
        if metadata.len() > (LIMIT + OVERHEAD) as u64 {
            return Err(StoreError::TooLarge);
        }
        let mut bytes = Vec::new();
        File::open(path)
            .map_err(|_| StoreError::Unavailable)?
            .take((LIMIT + OVERHEAD + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| StoreError::Unavailable)?;
        if bytes.len() > LIMIT + OVERHEAD {
            return Err(StoreError::TooLarge);
        }
        if bytes.len() < OVERHEAD {
            return Err(StoreError::Corrupt);
        }
        if &bytes[..7] != b"CUECORE" {
            return Err(StoreError::Corrupt);
        }
        if &bytes[..8] != HEADER {
            return Err(StoreError::UnsupportedVersion);
        }
        let nonce: [u8; 12] = bytes[8..20].try_into().map_err(|_| StoreError::Corrupt)?;
        let plaintext = store
            .key
            .open_in_place(
                aead::Nonce::assume_unique_for_key(nonce),
                aead::Aad::from(HEADER),
                &mut bytes[20..],
            )
            .map_err(|_| StoreError::Corrupt)?;
        let json: BTreeMap<String, Value> =
            serde_json::from_slice(plaintext).map_err(|_| StoreError::Corrupt)?;
        let values: StorageValues = json
            .into_iter()
            .map(|(key, value)| {
                serde_json::to_vec(&value)
                    .map(|bytes| (key, bytes))
                    .map_err(|_| StoreError::Corrupt)
            })
            .collect::<Result<_, _>>()?;
        if values.iter().map(|(k, v)| k.len() + v.len()).sum::<usize>() > LIMIT {
            return Err(StoreError::TooLarge);
        }
        Ok((store, values))
    }
    pub fn save(&self, values: &StorageValues) -> Result<(), StoreError> {
        if values.iter().map(|(k, v)| k.len() + v.len()).sum::<usize>() > LIMIT {
            return Err(StoreError::TooLarge);
        }
        let json: BTreeMap<&str, Value> = values
            .iter()
            .map(|(key, value)| {
                serde_json::from_slice(value)
                    .map(|value| (key.as_str(), value))
                    .map_err(|_| StoreError::Corrupt)
            })
            .collect::<Result<_, _>>()?;
        let mut bytes = serde_json::to_vec(&json).map_err(|_| StoreError::Corrupt)?;
        if bytes.len() > LIMIT {
            return Err(StoreError::TooLarge);
        }
        let mut nonce = [0u8; 12];
        SystemRandom::new()
            .fill(&mut nonce)
            .map_err(|_| StoreError::Unavailable)?;
        self.key
            .seal_in_place_append_tag(
                aead::Nonce::assume_unique_for_key(nonce),
                aead::Aad::from(HEADER),
                &mut bytes,
            )
            .map_err(|_| StoreError::Unavailable)?;
        let suffix: String = nonce.iter().map(|byte| format!("{byte:02x}")).collect();
        let temporary = self.directory.join(format!("state-{suffix}.tmp"));
        let result = (|| {
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options
                .open(&temporary)
                .map_err(|_| StoreError::Unavailable)?;
            file.write_all(HEADER)
                .and_then(|_| file.write_all(&nonce))
                .and_then(|_| file.write_all(&bytes))
                .and_then(|_| file.sync_all())
                .map_err(|_| StoreError::Unavailable)?;
            fs::rename(&temporary, self.directory.join("state.bin"))
                .map_err(|_| StoreError::Unavailable)?;
            // Rename is the commit point. Best-effort directory sync does not
            // turn a committed write into a falsely reported rollback.
            if let Ok(directory) = File::open(&self.directory) {
                let _ = directory.sync_all();
            }
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(temporary);
        }
        result
    }
}
