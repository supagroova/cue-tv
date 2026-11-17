//! Conservative admission plus write-time guards; no eviction of engine-owned pieces.
use anyhow::{Context, Result, bail};
use librqbit::{
    ManagedTorrentShared, TorrentMetadata,
    storage::{
        BoxStorageFactory, StorageFactory, StorageFactoryExt, TorrentStorage,
        filesystem::FilesystemStorageFactory,
    },
};
use std::{
    ffi::CString,
    os::unix::{ffi::OsStrExt, fs::MetadataExt},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

#[derive(Clone, Copy)]
pub struct StorageBudget {
    pub cap_bytes: u64,
    pub reserve_bytes: u64,
    pub metadata_bytes: u64,
    pub max_files: usize,
}
impl Default for StorageBudget {
    fn default() -> Self {
        Self {
            cap_bytes: 4 * 1024 * 1024 * 1024,
            reserve_bytes: 1024 * 1024 * 1024,
            metadata_bytes: 16 * 1024 * 1024,
            max_files: 512,
        }
    }
}
#[derive(Clone, Copy)]
pub struct DiskUsage {
    pub available: u64,
    pub allocated: u64,
    pub metadata_allocated: u64,
    pub block_size: u64,
}
pub type DiskProbe = Arc<dyn Fn(&Path) -> Result<DiskUsage> + Send + Sync>;

fn allocated(path: &Path) -> Result<(u64, u64)> {
    let metadata = std::fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        bail!("cache contains symbolic link");
    }
    let blocks = metadata
        .blocks()
        .checked_mul(512)
        .context("allocation overflow")?;
    let mut bytes = if metadata.is_file() { blocks } else { 0 };
    let mut overhead = if metadata.is_dir() { blocks } else { 0 };
    if metadata.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let (data, meta) = allocated(&entry?.path())?;
            bytes = bytes.checked_add(data).context("allocation overflow")?;
            overhead = overhead.checked_add(meta).context("allocation overflow")?;
        }
    }
    Ok((bytes, overhead))
}
fn disk_usage(root: &Path) -> Result<DiskUsage> {
    let path = CString::new(root.as_os_str().as_bytes())?;
    let mut stats = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    if unsafe { libc::statvfs(path.as_ptr(), stats.as_mut_ptr()) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let stats = unsafe { stats.assume_init() };
    let block = (stats.f_frsize as u64).max(4096);
    let (data, overhead) = allocated(root)?;
    Ok(DiskUsage {
        available: (stats.f_bavail as u64).saturating_mul(stats.f_frsize as u64),
        allocated: data,
        metadata_allocated: overhead,
        block_size: block,
    })
}
fn rounded(bytes: u64, block: u64) -> Result<u64> {
    if block == 0 {
        bail!("invalid allocation unit");
    }
    bytes
        .checked_add(block - 1)
        .and_then(|n| (n / block).checked_mul(block))
        .context("size overflow")
}

#[derive(Clone)]
pub struct BudgetFactory {
    root: PathBuf,
    budget: StorageBudget,
    probe: DiskProbe,
    lock: Arc<Mutex<bool>>,
}
impl BudgetFactory {
    /// Wait for in-flight storage calls and permanently reject subsequent I/O.
    pub fn close(&self) {
        let mut closed = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        *closed = true;
    }
    fn enter(&self) -> Result<std::sync::MutexGuard<'_, bool>> {
        let guard = self
            .lock
            .lock()
            .map_err(|_| anyhow::anyhow!("storage lock failed"))?;
        if *guard {
            bail!("storage closed");
        }
        Ok(guard)
    }
    pub fn new(root: PathBuf, budget: StorageBudget, probe: Option<DiskProbe>) -> Self {
        Self {
            root,
            budget,
            probe: probe.unwrap_or_else(|| Arc::new(disk_usage)),
            lock: Arc::new(Mutex::new(false)),
        }
    }
}
impl StorageFactory for BudgetFactory {
    type Storage = BudgetStorage;
    fn create(
        &self,
        shared: &ManagedTorrentShared,
        metadata: &TorrentMetadata,
    ) -> Result<Self::Storage> {
        let _guard = self.enter()?;
        Ok(BudgetStorage {
            inner: Box::new(FilesystemStorageFactory::default().create(shared, metadata)?),
            policy: self.clone(),
            lengths: metadata.file_infos.iter().map(|f| f.len).collect(),
        })
    }
    fn clone_box(&self) -> BoxStorageFactory {
        self.clone().boxed()
    }
}
pub struct BudgetStorage {
    inner: Box<dyn TorrentStorage>,
    policy: BudgetFactory,
    lengths: Vec<u64>,
}
impl BudgetStorage {
    fn guard_write(&self, file: usize, offset: u64, len: usize) -> Result<()> {
        let expected = *self.lengths.get(file).context("invalid file index")?;
        let end = offset.checked_add(len as u64).context("write overflow")?;
        if end > expected {
            bail!("write exceeds declared file length");
        }
        let usage = (self.policy.probe)(&self.policy.root)?;
        let declared = self.lengths.iter().try_fold(0u64, |sum, len| {
            sum.checked_add(rounded(*len, usage.block_size)?)
                .context("torrent size overflow")
        })?;
        // A filesystem may allocate more than the incoming chunk (including full-file extents).
        // Preserve capacity for all remaining declared data before every mutating operation.
        let remaining = declared.saturating_sub(usage.allocated);
        let overhead = self
            .policy
            .budget
            .metadata_bytes
            .max(usage.metadata_allocated);
        let bound = usage
            .allocated
            .checked_add(remaining)
            .and_then(|n| n.checked_add(overhead))
            .context("budget overflow")?;
        if bound > self.policy.budget.cap_bytes {
            bail!("cache budget exhausted");
        }
        let headroom = remaining
            .checked_add(overhead.saturating_sub(usage.metadata_allocated))
            .context("budget overflow")?;
        if usage.available
            < self
                .policy
                .budget
                .reserve_bytes
                .checked_add(headroom)
                .context("reserve overflow")?
        {
            bail!("storage reserve reached");
        }
        Ok(())
    }
}
impl TorrentStorage for BudgetStorage {
    fn init(&mut self, shared: &ManagedTorrentShared, metadata: &TorrentMetadata) -> Result<()> {
        let _guard = self.policy.enter()?;
        if self.lengths.len() > self.policy.budget.max_files {
            bail!("too many torrent files");
        }
        std::fs::create_dir_all(&self.policy.root)?;
        let usage = (self.policy.probe)(&self.policy.root)?;
        let full = self.lengths.iter().try_fold(
            self.policy
                .budget
                .metadata_bytes
                .max(usage.metadata_allocated),
            |sum, len| {
                sum.checked_add(rounded(*len, usage.block_size)?)
                    .context("torrent size overflow")
            },
        )?;
        if full
            .checked_add(usage.allocated)
            .context("budget overflow")?
            > self.policy.budget.cap_bytes
        {
            bail!("torrent exceeds cache budget");
        }
        if usage.available
            < full
                .checked_add(self.policy.budget.reserve_bytes)
                .context("reserve overflow")?
        {
            bail!("insufficient space after reserve");
        }
        self.inner.init(shared, metadata)
    }
    fn pread_exact(&self, file: usize, offset: u64, buf: &mut [u8]) -> Result<()> {
        let _guard = self.policy.enter()?;
        self.inner.pread_exact(file, offset, buf)
    }
    fn pwrite_all(&self, file: usize, offset: u64, buf: &[u8]) -> Result<()> {
        if buf.is_empty() {
            return Ok(());
        }
        let _guard = self.policy.enter()?;
        self.guard_write(file, offset, buf.len())?;
        self.inner.pwrite_all(file, offset, buf)
    }
    fn ensure_file_length(&self, file: usize, length: u64) -> Result<()> {
        if length > *self.lengths.get(file).context("invalid file index")? {
            bail!("invalid file length");
        }
        let _guard = self.policy.enter()?;
        self.guard_write(
            file,
            0,
            usize::try_from(length).context("file too large for platform")?,
        )?;
        self.inner.ensure_file_length(file, length)
    }
    fn remove_file(&self, file: usize, path: &Path) -> Result<()> {
        let _guard = self.policy.enter()?;
        self.inner.remove_file(file, path)
    }
    fn remove_directory_if_empty(&self, path: &Path) -> Result<()> {
        let _guard = self.policy.enter()?;
        self.inner.remove_directory_if_empty(path)
    }
    fn take(&self) -> Result<Box<dyn TorrentStorage>> {
        let _guard = self.policy.enter()?;
        Ok(Box::new(Self {
            inner: self.inner.take()?,
            policy: self.policy.clone(),
            lengths: self.lengths.clone(),
        }))
    }
}
