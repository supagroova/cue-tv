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
    cancellation: CancellationToken,
    ticket: u64,
    reply: Option<Vec<u8>>,
}
impl Drop for Handoff {
    fn drop(&mut self) {
        self.cancellation.cancel();
    }
}
struct Entry {
    identity: Mutex<Option<String>>,
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
#[derive(Clone)]
enum Startup {
    Guest,
    NewAccount,
    Restore(String),
}
struct Persistence {
    startup: Startup,
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
    unsafe { persistent(path, path_length, key, key_length, Startup::Guest) }
}
/// Create a login-capable session in an empty dedicated encrypted account store.
/// # Safety
/// Both non-null inputs must be readable for their supplied lengths; inputs are copied.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cue_core_create_account(
    path: *const u8,
    path_length: usize,
    key: *const u8,
    key_length: usize,
) -> u64 {
    unsafe { persistent(path, path_length, key, key_length, Startup::NewAccount) }
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
        unsafe {
            persistent(
                path,
                path_length,
                key,
                key_length,
                Startup::Restore(user.to_owned()),
            )
        }
    }))
    .unwrap_or(0)
}
unsafe fn persistent(
    path: *const u8,
    path_length: usize,
    key: *const u8,
    key_length: usize,
    startup: Startup,
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
            startup,
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
            identity: Mutex::new(None),
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
    let startup = persistence
        .as_ref()
        .map(|config| config.startup.clone())
        .unwrap_or(Startup::Guest);
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
        match startup {
            Startup::Restore(user) => CoreSession::restore_account(env.clone(), &user).await,
            Startup::NewAccount => CoreSession::new_account(env.clone()).await,
            Startup::Guest => CoreSession::guest(env.clone()).await,
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
                            Command::Login { email, password } => session.login(&email, &password).await,
                            Command::Reauthenticate { email, password } => session.reauthenticate(&email, &password).await,
                            Command::Logout { .. } => session.logout().await,
                            Command::RetrySync { .. } => session.retry_sync().await,
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
                            Command::PrepareHttps { ticket, index, revision } => {
                                let result = session.prepare_https(index, revision).await;
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
                            Command::DiscoverSubtitles { ticket, identity, cancellation } => {
                                let result = tokio::select! {
                                    biased;
                                    _ = cancellation.cancelled() => Err("subtitle_discovery_cancelled"),
                                    result = session.discover_subtitles(&identity.meta_id, &identity.video_id, &identity.kind, cancellation.clone()) => result,
                                };
                                let value = match result {
                                    Ok(request) => json!({"version":1,"session":handle,"ticket":ticket,"request":request,"error":null}),
                                    Err(error) => json!({"version":1,"session":handle,"ticket":ticket,"request":null,"error":error}),
                                };
                                if let (Ok(bytes), Ok(mut handoff)) = (serde_json::to_vec(&value), entry.handoff.lock()) {
                                    if handoff.ticket == ticket && !entry.stop.is_cancelled() && !cancellation.is_cancelled() {
                                        handoff.reply = Some(bytes);
                                    }
                                }
                                Ok(())
                            },
                            Command::PlaybackControl { ticket, token, action, time, duration, cancellation } => {
                                let operation = async {
                                    use crate::player::PlaybackEvent;
                                    match action {
                                        0 => session.start_playback(token).await.map(|_| None),
                                        1 => session.playback_event(token, PlaybackEvent::Seek { time, duration }).await.map(Some),
                                        2 => session.playback_event(token, PlaybackEvent::TimeChanged { time, duration }).await.map(Some),
                                        3 | 4 => session.playback_event(token, PlaybackEvent::Paused(action == 3)).await.map(Some),
                                        5 => session.checkpoint_playback(token).await.map(|_| None),
                                        6 => session.close_playback(token).await.map(|_| None),
                                        7 => session.retire_playback(token).await.map(|_| None),
                                        _ => Err("invalid_playback_action"),
                                    }
                                };
                                let result = tokio::select! {
                                    biased;
                                    _ = cancellation.cancelled() => Err("playback_operation_cancelled"),
                                    result = tokio::time::timeout(std::time::Duration::from_secs(4), operation) => result.unwrap_or(Err("playback_operation_timeout")),
                                };
                                let value = match result {
                                    Ok(progress) => json!({"version":1,"session":handle,"ticket":ticket,"request":{"token":token,"action":action,"progress":progress},"error":null}),
                                    Err(error) => json!({"version":1,"session":handle,"ticket":ticket,"request":null,"error":error}),
                                };
                                if let (Ok(bytes), Ok(mut handoff)) = (serde_json::to_vec(&value), entry.handoff.lock()) {
                                    if handoff.ticket == ticket && !entry.stop.is_cancelled() && !cancellation.is_cancelled() {
                                        handoff.reply = Some(bytes);
                                    }
                                }
                                Ok(())
                            },
                            Command::SaveWatched { ticket, video_id, watched, revision, cancellation } => {
                                let save = async {
                                    let identity = session.snapshot().details.map(|details| (details.kind, details.id));
                                    session.set_details_watched(video_id.clone(), watched, revision).await?;
                                    let state = session.settled().await?;
                                    if let Some(error) = state.error { return Err(error); }
                                    if state.details.map(|details| (details.kind, details.id)) != identity {
                                        return Err("watched_selection_changed");
                                    }
                                    Ok(())
                                };
                                let result = tokio::select! {
                                    biased;
                                    _ = cancellation.cancelled() => Err("watched_save_cancelled"),
                                    result = tokio::time::timeout(std::time::Duration::from_secs(4), save) => result.unwrap_or(Err("watched_save_timeout")),
                                };
                                let value = match result {
                                    Ok(()) => json!({"version":1,"session":handle,"ticket":ticket,"request":{"watched":watched,"video_id":video_id},"error":null}),
                                    Err(error) => json!({"version":1,"session":handle,"ticket":ticket,"request":null,"error":error}),
                                };
                                if let (Ok(bytes), Ok(mut handoff)) = (serde_json::to_vec(&value), entry.handoff.lock()) {
                                    if handoff.ticket == ticket && !entry.stop.is_cancelled() && !cancellation.is_cancelled() {
                                        handoff.reply = Some(bytes);
                                    }
                                }
                                Ok(())
                            },
                            Command::SaveLibrary { ticket, member, revision, cancellation } => {
                                let save = async {
                                    session.set_details_library(member, revision).await?;
                                    let state = session.settled().await?;
                                    if let Some(error) = state.error { return Err(error); }
                                    if state.details_in_library != Some(member) { return Err("library_selection_changed"); }
                                    Ok(())
                                };
                                let result = tokio::select! {
                                    biased;
                                    _ = cancellation.cancelled() => Err("library_save_cancelled"),
                                    result = tokio::time::timeout(std::time::Duration::from_secs(4), save) => result.unwrap_or(Err("library_save_timeout")),
                                };
                                let value = match result {
                                    Ok(()) => json!({"version":1,"session":handle,"ticket":ticket,"request":{"member":member},"error":null}),
                                    Err(error) => json!({"version":1,"session":handle,"ticket":ticket,"request":null,"error":error}),
                                };
                                if let (Ok(bytes), Ok(mut handoff)) = (serde_json::to_vec(&value), entry.handoff.lock()) {
                                    if handoff.ticket == ticket && !entry.stop.is_cancelled() && !cancellation.is_cancelled() {
                                        handoff.reply = Some(bytes);
                                    }
                                }
                                Ok(())
                            },
                            Command::SetLanguage { language, .. } => session.set_interface_language(&language).await,
                            Command::LoadAddon { url, .. } => session.load_addon(&url).await,
                            Command::InstallAddon { revision, .. } => session.install_addon(revision).await,
                            Command::DetailsVideos { offset, revision, .. } => session.show_details_videos(offset, revision).await,
                            Command::DetailsSeason { season, revision, .. } => session.select_details_season(season, revision).await,
                            Command::LoadSources { index, revision, .. } => session.load_sources(index, revision).await,
                            Command::SourceWindow { offset, revision, .. } => session.show_sources(offset, revision).await,
                            Command::LibraryWindow { offset, revision, .. } => session.show_library(offset, revision).await,
                            Command::LibraryDetails { index, revision, .. } => session.load_library_details(index, revision).await,
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
                    if let Ok(mut identity) = entry.identity.lock() { *identity = session.account_identity(); }
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
    PlaybackControl {
        ticket: u64,
        token: u64,
        action: u8,
        time: u64,
        duration: u64,
        cancellation: CancellationToken,
    },
    #[serde(skip)]
    SaveWatched {
        ticket: u64,
        video_id: Option<String>,
        watched: bool,
        revision: u64,
        cancellation: CancellationToken,
    },
    #[serde(skip)]
    SaveLibrary {
        ticket: u64,
        member: bool,
        revision: u64,
        cancellation: CancellationToken,
    },
    #[serde(skip)]
    Login {
        email: String,
        password: String,
    },
    #[serde(skip)]
    Reauthenticate {
        email: String,
        password: String,
    },
    Logout {
        version: u32,
    },
    RetrySync {
        version: u32,
    },
    #[serde(skip)]
    PrepareTorrent {
        ticket: u64,
        index: usize,
        revision: u64,
    },
    #[serde(skip)]
    PrepareHttps {
        ticket: u64,
        index: usize,
        revision: u64,
    },
    #[serde(skip)]
    DiscoverSubtitles {
        ticket: u64,
        identity: SubtitleIdentity,
        cancellation: CancellationToken,
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
    DetailsSeason {
        version: u32,
        season: Option<u32>,
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
    LibraryWindow {
        version: u32,
        offset: usize,
        revision: u64,
    },
    LibraryDetails {
        version: u32,
        index: usize,
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
            Self::PlaybackControl { .. }
            | Self::SaveWatched { .. }
            | Self::SaveLibrary { .. }
            | Self::DiscoverSubtitles { .. }
            | Self::PrepareTorrent { .. }
            | Self::PrepareHttps { .. }
            | Self::Login { .. }
            | Self::Reauthenticate { .. } => false,
            Self::Logout { version } | Self::RetrySync { version } => *version == 1,
            Self::InstallAddon { version, .. }
            | Self::DetailsSeason { version, .. }
            | Self::DetailsVideos { version, .. }
            | Self::LoadSources { version, .. }
            | Self::SourceWindow { version, .. }
            | Self::LibraryWindow { version, .. }
            | Self::LibraryDetails { version, .. }
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
/// Copies credentials into the bounded command queue. Admission is not authentication.
/// # Safety
/// Non-null input pointers must be readable for their supplied lengths.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cue_core_login(
    handle: u64,
    email: *const u8,
    email_length: usize,
    password: *const u8,
    password_length: usize,
) -> i32 {
    unsafe {
        admit_credentials(
            handle,
            email,
            email_length,
            password,
            password_length,
            false,
        )
    }
}
/// Refresh an existing paused account. Completion is reported through snapshots.
/// # Safety
/// Non-null input pointers must be readable for their supplied lengths.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cue_core_reauthenticate(
    handle: u64,
    email: *const u8,
    email_length: usize,
    password: *const u8,
    password_length: usize,
) -> i32 {
    unsafe { admit_credentials(handle, email, email_length, password, password_length, true) }
}
unsafe fn admit_credentials(
    handle: u64,
    email: *const u8,
    email_length: usize,
    password: *const u8,
    password_length: usize,
    refresh: bool,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        if email.is_null()
            || password.is_null()
            || email_length == 0
            || email_length > 512
            || password_length == 0
            || password_length > 4096
        {
            return -2;
        }
        let Ok(email) =
            std::str::from_utf8(unsafe { std::slice::from_raw_parts(email, email_length) })
        else {
            return -2;
        };
        let Ok(password) =
            std::str::from_utf8(unsafe { std::slice::from_raw_parts(password, password_length) })
        else {
            return -2;
        };
        if email.chars().any(char::is_control)
            || password.contains('\0')
            || (refresh && password.chars().any(char::is_control))
        {
            return -2;
        }
        let Some(entry) = lookup(handle) else {
            return -1;
        };
        if entry.stop.is_cancelled() {
            return -1;
        }
        let command = if refresh {
            Command::Reauthenticate {
                email: email.to_owned(),
                password: password.to_owned(),
            }
        } else {
            Command::Login {
                email: email.to_owned(),
                password: password.to_owned(),
            }
        };
        match entry.commands.try_send(command) {
            Ok(()) => 0,
            Err(mpsc::error::TrySendError::Full(_)) => -3,
            Err(_) => -1,
        }
    }))
    .unwrap_or(-1)
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
/// Copies only the authenticated user ID for protected account selection.
/// # Safety
/// A non-null output must be writable for capacity bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cue_core_account_identity(
    handle: u64,
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
        if entry.stop.is_cancelled() {
            return -1;
        }
        let Ok(identity) = entry.identity.lock() else {
            return -1;
        };
        let Some(identity) = identity.as_ref() else {
            return 0;
        };
        if !output.is_null() && capacity >= identity.len() {
            unsafe {
                std::ptr::copy_nonoverlapping(identity.as_ptr(), output, identity.len());
            }
        }
        identity.len() as isize
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
    request_source(handle, index, revision, false)
}
/// Uses the same one-ticket handoff slot and take/cancel functions as torrents.
#[unsafe(no_mangle)]
pub extern "C" fn cue_core_request_https(handle: u64, index: usize, revision: u64) -> u64 {
    request_source(handle, index, revision, true)
}
fn request_source(handle: u64, index: usize, revision: u64, https: bool) -> u64 {
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
            .try_send(if https {
                Command::PrepareHttps {
                    ticket,
                    index,
                    revision,
                }
            } else {
                Command::PrepareTorrent {
                    ticket,
                    index,
                    revision,
                }
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SubtitleIdentity {
    version: u32,
    meta_id: String,
    video_id: String,
    kind: String,
}
/// Requests private subtitles in the shared handoff slot. Zero means invalid/busy.
/// # Safety
/// Input must be readable for length bytes; it is copied before returning.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cue_core_request_subtitles(
    handle: u64,
    input: *const u8,
    length: usize,
) -> u64 {
    catch_unwind(AssertUnwindSafe(|| {
        if input.is_null() || length == 0 || length > 16384 {
            return 0;
        }
        let bytes = unsafe { std::slice::from_raw_parts(input, length) };
        let Ok(identity) = serde_json::from_slice::<SubtitleIdentity>(bytes) else {
            return 0;
        };
        if identity.version != 1
            || identity.kind.len() > 64
            || [&identity.meta_id, &identity.video_id, &identity.kind]
                .iter()
                .any(|value| {
                    value.is_empty() || value.len() > 2048 || value.chars().any(char::is_control)
                })
        {
            return 0;
        }
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
        let cancellation = handoff.cancellation.clone();
        if entry
            .commands
            .try_send(Command::DiscoverSubtitles {
                ticket,
                identity,
                cancellation,
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

/// Playback actions share the private receipt slot; admission is not completion.
/// Cancel/timeout discards observation, not an operation already admitted by Core.
#[unsafe(no_mangle)]
pub extern "C" fn cue_core_request_playback(
    handle: u64,
    token: u64,
    action: u8,
    time: u64,
    duration: u64,
) -> u64 {
    catch_unwind(AssertUnwindSafe(|| {
        if token == 0
            || action > 7
            || time > i64::MAX as u64
            || duration > i64::MAX as u64
            || (!matches!(action, 1 | 2) && (time != 0 || duration != 0))
        {
            return 0;
        }
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
        let cancellation = handoff.cancellation.clone();
        if entry
            .commands
            .try_send(Command::PlaybackControl {
                ticket,
                token,
                action,
                time,
                duration,
                cancellation,
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

/// Saves selected-title membership using the exclusive reply slot. Member is 0 or 1.
/// A successful receipt requires settled Core effects. Discard is not a rollback.
#[unsafe(no_mangle)]
pub extern "C" fn cue_core_request_library(handle: u64, member: u8, revision: u64) -> u64 {
    catch_unwind(AssertUnwindSafe(|| {
        if member > 1 {
            return 0;
        }
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
        let cancellation = handoff.cancellation.clone();
        if entry
            .commands
            .try_send(Command::SaveLibrary {
                ticket,
                member: member == 1,
                revision,
                cancellation,
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

/// Requests a watched-state save, sharing the exclusive take/discard slot.
/// # Safety
/// For nonzero length, video must point to length readable bytes. Input is copied.
/// A zero length denotes a film; episode IDs are UTF-8 and at most 512 bytes.
/// Cancellation does not roll back an admitted mutation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cue_core_request_watched(
    handle: u64,
    video: *const u8,
    length: usize,
    watched: u8,
    revision: u64,
) -> u64 {
    catch_unwind(AssertUnwindSafe(|| {
        if watched > 1 || length > 512 || (length != 0 && video.is_null()) {
            return 0;
        }
        let video_id = if length == 0 {
            None
        } else {
            let Ok(value) =
                std::str::from_utf8(unsafe { std::slice::from_raw_parts(video, length) })
            else {
                return 0;
            };
            if value.chars().any(char::is_control) {
                return 0;
            }
            Some(value.to_owned())
        };
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
        let cancellation = handoff.cancellation.clone();
        if entry
            .commands
            .try_send(Command::SaveWatched {
                ticket,
                video_id,
                watched: watched == 1,
                revision,
                cancellation,
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
