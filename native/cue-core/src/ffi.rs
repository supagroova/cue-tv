//! Copy-only C boundary; no Rust allocations or borrowed model objects cross it.
use crate::{environment::Environment, session::CoreSession};
use futures::FutureExt;
use serde::Deserialize;
use serde_json::json;
use std::{
    collections::BTreeMap,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

#[derive(Default)]
struct Handoff {
    ticket: u64,
    reply: Option<Vec<u8>>,
}
struct Entry {
    handoff: Mutex<Handoff>,
    commands: mpsc::Sender<Command>,
    snapshot: Mutex<Vec<u8>>,
    stop: CancellationToken,
    stopped: AtomicBool,
}
static ENTRIES: OnceLock<Mutex<BTreeMap<u64, Arc<Entry>>>> = OnceLock::new();
static NEXT_TICKET: AtomicU64 = AtomicU64::new(1);
static NEXT: AtomicU64 = AtomicU64::new(1);
fn entries() -> &'static Mutex<BTreeMap<u64, Arc<Entry>>> {
    ENTRIES.get_or_init(Default::default)
}
fn lookup(handle: u64) -> Option<Arc<Entry>> {
    entries().lock().ok()?.get(&handle).cloned()
}
fn runtime() -> Option<&'static tokio::runtime::Runtime> {
    static RUNTIME: OnceLock<Option<tokio::runtime::Runtime>> = OnceLock::new();
    RUNTIME
        .get_or_init(|| {
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .ok()
        })
        .as_ref()
}
fn publish(
    entry: &Entry,
    handle: u64,
    status: &str,
    model: Option<crate::session::Snapshot>,
    error: Option<&str>,
) {
    let value = json!({"version":1,"session":handle,"status":status,"model":model,"error":error});
    if let (Ok(bytes), Ok(mut snapshot)) = (serde_json::to_vec(&value), entry.snapshot.lock()) {
        *snapshot = bytes;
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn cue_core_abi_version() -> u32 {
    1
}
#[unsafe(no_mangle)]
pub extern "C" fn cue_core_create() -> u64 {
    create(None)
}
struct Persistence {
    account: Option<String>,
    directory: std::path::PathBuf,
    key: [u8; 32],
}
/// # Safety
/// Both non-null inputs must be readable for the supplied lengths. Inputs are copied.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cue_core_create_persistent(
    path: *const u8,
    path_length: usize,
    key: *const u8,
    key_length: usize,
) -> u64 {
    unsafe { persistent(path, path_length, key, key_length, None) }
}
/// Restore an identity selected by protected app storage. All inputs are copied.
/// # Safety
/// Non-null inputs must be readable for their supplied lengths.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cue_core_restore_account(
    path: *const u8,
    path_length: usize,
    key: *const u8,
    key_length: usize,
    user: *const u8,
    user_length: usize,
) -> u64 {
    catch_unwind(AssertUnwindSafe(|| {
        if user.is_null() || user_length == 0 || user_length > 512 {
            return 0;
        }
        let Ok(user) =
            std::str::from_utf8(unsafe { std::slice::from_raw_parts(user, user_length) })
        else {
            return 0;
        };
        if user.chars().any(char::is_control) {
            return 0;
        }
        unsafe { persistent(path, path_length, key, key_length, Some(user.to_owned())) }
    }))
    .unwrap_or(0)
}
unsafe fn persistent(
    path: *const u8,
    path_length: usize,
    key: *const u8,
    key_length: usize,
    account: Option<String>,
) -> u64 {
    catch_unwind(AssertUnwindSafe(|| {
        if path.is_null()
            || key.is_null()
            || path_length == 0
            || path_length > 4096
            || key_length != 32
        {
            return 0;
        }
        let Ok(path) =
            std::str::from_utf8(unsafe { std::slice::from_raw_parts(path, path_length) })
        else {
            return 0;
        };
        let directory = std::path::PathBuf::from(path);
        if path.contains('\0')
            || !directory.is_absolute()
            || directory
                .components()
                .any(|c| c == std::path::Component::ParentDir)
        {
            return 0;
        }
        let Ok(key) = <[u8; 32]>::try_from(unsafe { std::slice::from_raw_parts(key, key_length) })
        else {
            return 0;
        };
        create(Some(Persistence {
            directory,
            key,
            account,
        }))
    }))
    .unwrap_or(0)
}
fn create(persistence: Option<Persistence>) -> u64 {
    catch_unwind(AssertUnwindSafe(|| {
        let Some(runtime) = runtime() else {
            return 0;
        };
        let Ok(mut all) = entries().lock() else {
            return 0;
        };
        if all.len() >= 8 {
            return 0;
        }
        let Ok(handle) =
            NEXT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
        else {
            return 0;
        };
        let (commands, inbox) = mpsc::channel(64);
        let entry = Arc::new(Entry {
            commands,
            handoff: Mutex::new(Handoff::default()),
            snapshot: Mutex::new(Vec::new()),
            stop: CancellationToken::new(),
            stopped: AtomicBool::new(false),
        });
        publish(&entry, handle, "starting", None, None);
        all.insert(handle, entry.clone());
        runtime.spawn(async move {
            own(handle, entry, inbox, persistence).await;
        });
        handle
    }))
    .unwrap_or(0)
}
async fn own(
    handle: u64,
    entry: Arc<Entry>,
    mut inbox: mpsc::Receiver<Command>,
    persistence: Option<Persistence>,
) {
    let expected_user = persistence
        .as_ref()
        .and_then(|config| config.account.clone());
    let environment = match persistence {
        Some(config) => Environment::persistent(&config.directory, &config.key),
        None => Environment::new(),
    };
    let env = match environment {
        Ok(env) => env,
        Err(_) => {
            publish(&entry, handle, "stopped", None, Some("core_start_failed"));
            entry.stopped.store(true, Ordering::Release);
            return;
        }
    };
    let setup = AssertUnwindSafe(async {
        match expected_user {
            Some(user) => CoreSession::restore_account(env.clone(), &user).await,
            None => CoreSession::guest(env.clone()).await,
        }
    })
    .catch_unwind()
    .await;
    let mut session = match setup {
        Ok(Ok(session)) => session,
        _ => {
            env.shutdown().await;
            drop(env);
            publish(&entry, handle, "stopped", None, Some("core_start_failed"));
            entry.stop.cancel();
            entry.stopped.store(true, Ordering::Release);
            return;
        }
    };
    let result = AssertUnwindSafe(async {
        let mut command_error = None;
        let mut tick = tokio::time::interval(std::time::Duration::from_millis(20));
        loop {
            tokio::select! { biased;
                _ = entry.stop.cancelled() => break,
                _ = env.cancelled() => break,
                command = inbox.recv() => {
                    let Some(command) = command else { break; };
                    tokio::select! { biased;
                        _ = entry.stop.cancelled() => break,
                        result = async { match command {
                            Command::PrepareTorrent { ticket, index, revision } => {
                                let result = session.prepare_torrent(index, revision).await;
                                let value = match result {
                                    Ok(request) => json!({"version":1,"session":handle,"ticket":ticket,"request":request,"error":null}),
                                    Err(error) => json!({"version":1,"session":handle,"ticket":ticket,"request":null,"error":error}),
                                };
                                if let (Ok(bytes), Ok(mut handoff)) = (serde_json::to_vec(&value), entry.handoff.lock()) {
                                    if handoff.ticket == ticket && !entry.stop.is_cancelled() {
                                        handoff.reply = Some(bytes);
                                    }
                                }
                                Ok(())
                            },
                            Command::SetLanguage { language, .. } => session.set_interface_language(&language).await,
                            Command::LoadAddon { url, .. } => session.load_addon(&url).await,
                            Command::InstallAddon { revision, .. } => session.install_addon(revision).await,
                            Command::DetailsVideos { offset, revision, .. } => session.show_details_videos(offset, revision).await,
                            Command::LoadSources { index, revision, .. } => session.load_sources(index, revision).await,
                            Command::SourceWindow { offset, revision, .. } => session.show_sources(offset, revision).await,
                            Command::LoadDetails { index, revision, .. } => session.load_details(index, revision).await,
                            Command::LoadCatalog { index, revision, query, .. } => match query { Some(query) => session.search_catalog(index, &query, revision).await, None => session.load_catalog(index, revision).await },
                            Command::CatalogChoices { offset, revision, .. } => session.show_catalog_choices(offset, revision).await,
                            Command::CatalogItems { offset, revision, .. } => session.show_catalog_items(offset, revision).await,
                        }} => {
                            command_error = result.err();
                        }
                    }
                },
                _ = tick.tick() => {
                    publish(&entry, handle, "ready", Some(session.snapshot()), command_error);
                }
            }
        }
    })
    .catch_unwind()
    .await;
    inbox.close();
    if result.is_err() {
        env.fail();
    }
    // Keep the session outside the unwind boundary so its model owner is joined
    // even when bridge command processing fails.
    session.stop().await;
    publish(
        &entry,
        handle,
        "stopped",
        Some(session.snapshot()),
        env.failure(),
    );
    drop(session);
    drop(env);
    entry.stop.cancel();
    if let Ok(mut handoff) = entry.handoff.lock() {
        *handoff = Handoff::default();
    }
    entry.stopped.store(true, Ordering::Release);
}
#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
enum Command {
    #[serde(skip)]
    PrepareTorrent {
        ticket: u64,
        index: usize,
        revision: u64,
    },
    SetLanguage {
        version: u32,
        language: String,
    },
    LoadAddon {
        version: u32,
        url: String,
    },
    InstallAddon {
        version: u32,
        revision: u64,
    },
    DetailsVideos {
        version: u32,
        offset: usize,
        revision: u64,
    },
    LoadSources {
        version: u32,
        index: Option<usize>,
        revision: u64,
    },
    SourceWindow {
        version: u32,
        offset: usize,
        revision: u64,
    },
    LoadDetails {
        version: u32,
        index: usize,
        revision: u64,
    },
    LoadCatalog {
        query: Option<String>,
        version: u32,
        index: usize,
        revision: u64,
    },
    CatalogChoices {
        version: u32,
        offset: usize,
        revision: u64,
    },
    CatalogItems {
        version: u32,
        offset: usize,
        revision: u64,
    },
}
impl Command {
    fn valid(&self) -> bool {
        match self {
            Self::PrepareTorrent { .. } => false,
            Self::InstallAddon { version, .. }
            | Self::DetailsVideos { version, .. }
            | Self::LoadSources { version, .. }
            | Self::SourceWindow { version, .. }
            | Self::LoadDetails { version, .. }
            | Self::LoadCatalog { version, .. }
            | Self::CatalogChoices { version, .. }
            | Self::CatalogItems { version, .. } => *version == 1,
            Self::SetLanguage { version, language } => {
                *version == 1
                    && language.len() == 3
                    && language.bytes().all(|b| b.is_ascii_lowercase())
            }
            Self::LoadAddon { version, url } => {
                *version == 1 && crate::session::validate_addon_url(url).is_ok()
            }
        }
    }
}
/// # Safety
/// A non-null input must be readable for `length` bytes for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cue_core_dispatch(handle: u64, bytes: *const u8, length: usize) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        if bytes.is_null() || length == 0 || length > 4096 {
            return -2;
        }
        let Ok(command) =
            serde_json::from_slice::<Command>(unsafe { std::slice::from_raw_parts(bytes, length) })
        else {
            return -2;
        };
        if !command.valid() {
            return -2;
        }
        let Some(entry) = lookup(handle) else {
            return -1;
        };
        if entry.stop.is_cancelled() {
            return -1;
        }
        match entry.commands.try_send(command) {
            Ok(()) => 0,
            Err(mpsc::error::TrySendError::Full(_)) => -3,
            Err(_) => -1,
        }
    }))
    .unwrap_or(-1)
}
/// # Safety
/// If capacity is nonzero, output must be writable for capacity bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cue_core_snapshot(handle: u64, output: *mut u8, capacity: usize) -> isize {
    catch_unwind(AssertUnwindSafe(|| {
        if output.is_null() && capacity != 0 {
            return -1;
        }
        let Some(entry) = lookup(handle) else {
            return -1;
        };
        let Ok(bytes) = entry.snapshot.lock() else {
            return -1;
        };
        if capacity >= bytes.len() && !output.is_null() {
            unsafe {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), output, bytes.len());
            }
        }
        bytes.len() as isize
    }))
    .unwrap_or(-1)
}
#[unsafe(no_mangle)]
pub extern "C" fn cue_core_stop(handle: u64) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        let Some(entry) = lookup(handle) else {
            return -1;
        };
        entry.stop.cancel();
        if let Ok(mut handoff) = entry.handoff.lock() {
            *handoff = Handoff::default();
        }
        i32::from(entry.stopped.load(Ordering::Acquire))
    }))
    .unwrap_or(-1)
}
#[unsafe(no_mangle)]
pub extern "C" fn cue_core_stop_status(handle: u64) -> i32 {
    lookup(handle)
        .map(|entry| i32::from(entry.stopped.load(Ordering::Acquire)))
        .unwrap_or(-1)
}
#[unsafe(no_mangle)]
pub extern "C" fn cue_core_release(handle: u64) -> i32 {
    let Ok(mut all) = entries().lock() else {
        return -1;
    };
    let Some(entry) = all.get(&handle) else {
        return -1;
    };
    if !entry.stopped.load(Ordering::Acquire) {
        return 0;
    }
    all.remove(&handle);
    1
}

/// Starts one bounded handoff per session. Zero means unavailable or busy.
#[unsafe(no_mangle)]
pub extern "C" fn cue_core_request_torrent(handle: u64, index: usize, revision: u64) -> u64 {
    catch_unwind(AssertUnwindSafe(|| {
        let Some(entry) = lookup(handle) else {
            return 0;
        };
        let Ok(mut handoff) = entry.handoff.lock() else {
            return 0;
        };
        if entry.stop.is_cancelled() || entry.stopped.load(Ordering::Acquire) || handoff.ticket != 0
        {
            return 0;
        }
        let Ok(ticket) =
            NEXT_TICKET.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
        else {
            return 0;
        };
        handoff.ticket = ticket;
        handoff.reply = None;
        if entry
            .commands
            .try_send(Command::PrepareTorrent {
                ticket,
                index,
                revision,
            })
            .is_err()
        {
            *handoff = Handoff::default();
            return 0;
        }
        ticket
    }))
    .unwrap_or(0)
}
/// # Safety
/// Non-null output must be writable for capacity bytes. A successful full copy consumes
/// the reply. Length queries/undersized buffers retain it. Zero is pending; -1 invalid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cue_core_take_torrent(
    handle: u64,
    ticket: u64,
    output: *mut u8,
    capacity: usize,
) -> isize {
    catch_unwind(AssertUnwindSafe(|| {
        if output.is_null() && capacity != 0 {
            return -1;
        }
        let Some(entry) = lookup(handle) else {
            return -1;
        };
        let Ok(mut handoff) = entry.handoff.lock() else {
            return -1;
        };
        if ticket == 0
            || handoff.ticket != ticket
            || entry.stop.is_cancelled()
            || entry.stopped.load(Ordering::Acquire)
        {
            return -1;
        }
        let Some(bytes) = &handoff.reply else {
            return 0;
        };
        let length = bytes.len();
        if capacity >= length && !output.is_null() {
            unsafe {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), output, length);
            }
            *handoff = Handoff::default();
        }
        length as isize
    }))
    .unwrap_or(-1)
}
#[unsafe(no_mangle)]
pub extern "C" fn cue_core_discard_torrent(handle: u64, ticket: u64) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        let Some(entry) = lookup(handle) else {
            return -1;
        };
        let Ok(mut handoff) = entry.handoff.lock() else {
            return -1;
        };
        if ticket == 0 || handoff.ticket != ticket {
            return -1;
        }
        *handoff = Handoff::default();
        1
    }))
    .unwrap_or(-1)
}
