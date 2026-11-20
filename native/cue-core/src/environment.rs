use chrono::{DateTime, Utc};
use futures::{Future, FutureExt};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use stremio_core::{
    models::{ctx::Ctx, streaming_server::StreamingServer},
    runtime::{ConditionalSend, Env, EnvError, EnvFuture, TryEnvFuture},
    types::remote_config::ClientInfo,
};
use tokio_util::sync::CancellationToken;

pub(crate) const AUTH_REJECTED: &str = "HTTP authentication rejected";

const RESPONSE_LIMIT: usize = 4 * 1024 * 1024;
const STORAGE_LIMIT: usize = 16 * 1024 * 1024;
tokio::task_local! { static CONTEXT: Arc<Environment>; }

/// One context per Core session. The default is memory-only; optional persistence
/// encrypts the entire state before writing to the app-owned directory.
#[cfg(test)]
type TestHttp =
    Arc<dyn Fn(&http::request::Parts, &[u8]) -> Result<Vec<u8>, EnvError> + Send + Sync>;
#[cfg(test)]
type TestHttpAsync = Arc<
    dyn Fn(
            http::request::Parts,
            Vec<u8>,
        ) -> futures::future::BoxFuture<'static, Result<Vec<u8>, EnvError>>
        + Send
        + Sync,
>;
pub struct Environment {
    #[cfg(test)]
    pub(crate) test_http: Mutex<Option<TestHttp>>,
    #[cfg(test)]
    pub(crate) test_http_async: Mutex<Option<TestHttpAsync>>,
    storage: Mutex<BTreeMap<String, Vec<u8>>>,
    persistence: Option<crate::persistence::EncryptedStore>,
    canceled: CancellationToken,
    failed: AtomicBool,
    tasks: tokio_util::task::TaskTracker,
    runtime: tokio::runtime::Handle,
    sequential: tokio::sync::mpsc::Sender<futures::future::BoxFuture<'static, ()>>,
    client: reqwest::Client,
    clock: Arc<dyn Fn() -> DateTime<Utc> + Send + Sync>,
}
impl Environment {
    pub fn new() -> Result<Arc<Self>, EnvError> {
        Self::with_clock(Arc::new(Utc::now))
    }
    pub fn with_clock(
        clock: Arc<dyn Fn() -> DateTime<Utc> + Send + Sync>,
    ) -> Result<Arc<Self>, EnvError> {
        Self::configured(clock, BTreeMap::new(), None)
    }
    /// Blocking initialization for an app-owned encrypted directory, off the UI actor.
    pub fn persistent(directory: &std::path::Path, key: &[u8; 32]) -> Result<Arc<Self>, EnvError> {
        let (store, values) = crate::persistence::EncryptedStore::open(directory, key)
            .map_err(|_| EnvError::StorageUnavailable)?;
        Self::configured(Arc::new(Utc::now), values, Some(store))
    }
    fn configured(
        clock: Arc<dyn Fn() -> DateTime<Utc> + Send + Sync>,
        values: BTreeMap<String, Vec<u8>>,
        persistence: Option<crate::persistence::EncryptedStore>,
    ) -> Result<Arc<Self>, EnvError> {
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| EnvError::Other("Core runtime unavailable".into()))?;
        let (sequential, mut queue) =
            tokio::sync::mpsc::channel::<futures::future::BoxFuture<'static, ()>>(128);
        let env = Arc::new(Self {
            #[cfg(test)]
            test_http: Mutex::new(None),
            #[cfg(test)]
            test_http_async: Mutex::new(None),
            storage: Mutex::new(values),
            persistence,
            canceled: CancellationToken::new(),
            failed: AtomicBool::new(false),
            tasks: tokio_util::task::TaskTracker::new(),
            runtime: runtime.clone(),
            sequential,
            clock,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(15))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .map_err(|_| EnvError::Fetch("HTTP client unavailable".into()))?,
        });
        let weak = Arc::downgrade(&env);
        let canceled = env.canceled.clone();
        env.tasks.spawn_on(async move {
            loop {
                let future = tokio::select! { biased;
                    _ = canceled.cancelled() => break,
                    future = queue.recv() => match future { Some(future) => future, None => break },
                };
                let Some(env) = weak.upgrade() else {
                    break;
                };
                tokio::select! { biased;
                    _ = canceled.cancelled() => break,
                    result = std::panic::AssertUnwindSafe(env.scope(future)).catch_unwind() => {
                        if result.is_err() { env.fail(); break; }
                    },
                }
            }
        }, &runtime);
        Ok(env)
    }
    pub(crate) fn storage_is_empty(&self) -> Result<bool, EnvError> {
        self.storage
            .lock()
            .map(|values| values.is_empty())
            .map_err(|_| EnvError::StorageUnavailable)
    }
    pub async fn scope<T>(self: &Arc<Self>, future: impl Future<Output = T>) -> T {
        CONTEXT.scope(self.clone(), future).await
    }
    /// Serializes cancellation with storage access; subsequent writes cannot succeed.
    pub fn cancel(&self) {
        let _guard = self.storage.lock().unwrap_or_else(|e| e.into_inner());
        self.canceled.cancel();
        self.tasks.close();
    }
    pub(crate) fn fail(&self) {
        self.failed.store(true, Ordering::Release);
        self.cancel();
    }
    pub(crate) fn failure(&self) -> Option<&'static str> {
        self.failed
            .load(Ordering::Acquire)
            .then_some("core_task_failed")
    }
    pub async fn cancelled(&self) {
        self.canceled.cancelled().await;
    }
    pub async fn shutdown(&self) {
        self.cancel();
        self.tasks.wait().await;
    }
}
fn context() -> Result<Arc<Environment>, EnvError> {
    CONTEXT
        .try_with(Arc::clone)
        .map_err(|_| EnvError::Other("Core context unavailable".into()))
}
fn canceled() -> EnvError {
    EnvError::Other("Core session canceled".into())
}
fn decode<T: for<'de> Deserialize<'de>>(bytes: &[u8]) -> Result<T, EnvError> {
    serde_json::from_slice(bytes).map_err(|_| EnvError::Serde("Invalid JSON".into()))
}

pub(crate) type StorageBatch = BTreeMap<String, Option<Vec<u8>>>;

fn write_storage_values(
    env: Result<Arc<Environment>, EnvError>,
    updates: Result<StorageBatch, EnvError>,
) -> TryEnvFuture<()> {
    async move {
        let env = env?;
        let updates = updates?;
        let mut store = env
            .storage
            .lock()
            .map_err(|_| EnvError::StorageUnavailable)?;
        if env.canceled.is_cancelled() {
            return Err(canceled());
        }
        let mut next = store.clone();
        for (key, value) in updates {
            if let Some(value) = value {
                next.insert(key, value);
            } else {
                next.remove(&key);
            }
        }
        let size = next.iter().try_fold(0usize, |sum, (key, value)| {
            sum.checked_add(key.len())
                .and_then(|sum| sum.checked_add(value.len()))
        });
        if size.is_none_or(|size| size > STORAGE_LIMIT) {
            return Err(EnvError::StorageWriteError(
                "Storage budget exceeded".into(),
            ));
        }
        if let Some(persistence) = &env.persistence {
            persistence.save(&next).map_err(|_| {
                EnvError::StorageWriteError("Encrypted storage write failed".into())
            })?;
        }
        // Publish all keys only after the single encrypted-store replacement succeeds.
        *store = next;
        Ok(())
    }
    .boxed()
}

pub struct NativeEnv;
impl NativeEnv {
    /// Atomic update of already-serialized Core values in this identity's store.
    pub(crate) fn set_storage_batch(updates: StorageBatch) -> TryEnvFuture<()> {
        write_storage_values(context(), Ok(updates))
    }
}

impl Env for NativeEnv {
    fn addon_transport(
        url: &reqwest::Url,
    ) -> Box<dyn stremio_core::addon_transport::AddonTransport> {
        crate::addon_transport::transport(url)
    }

    fn fetch<
        IN: Serialize + ConditionalSend + 'static,
        OUT: for<'de> Deserialize<'de> + ConditionalSend + 'static,
    >(
        request: http::Request<IN>,
    ) -> TryEnvFuture<OUT> {
        let context = context();
        async move {
            let env = context?;
            let (parts, body) = request.into_parts();
            #[cfg(test)]
            {
                let hook = env.test_http_async.lock().unwrap().clone();
                if let Some(hook) = hook {
                    let bytes = serde_json::to_vec(&body)
                        .map_err(|_| EnvError::Serde("Invalid request".into()))?;
                    let response = tokio::select! { biased;
                        _ = env.canceled.cancelled() => return Err(canceled()),
                        response = hook(parts, bytes) => response?,
                    };
                    return decode(&response);
                }
            }

            #[cfg(test)]
            if let Some(http) = env.test_http.lock().unwrap().clone() {
                if env.canceled.is_cancelled() {
                    return Err(canceled());
                }
                let body = serde_json::to_vec(&body)
                    .map_err(|_| EnvError::Serde("Invalid request".into()))?;
                return decode(&http(&parts, &body)?);
            }
            let uri = parts.uri.to_string();
            if !matches!(parts.uri.scheme_str(), Some("http" | "https")) {
                return Err(EnvError::Fetch("Unsupported URL scheme".into()));
            }
            let needs_json_type = !parts.headers.contains_key(http::header::CONTENT_TYPE);
            let mut request = env
                .client
                .request(parts.method.clone(), uri)
                .headers(parts.headers);
            if parts.method != http::Method::GET && parts.method != http::Method::HEAD {
                let body = serde_json::to_vec(&body)
                    .map_err(|_| EnvError::Serde("Invalid request".into()))?;
                if body.len() > RESPONSE_LIMIT {
                    return Err(EnvError::Fetch("Request too large".into()));
                }
                if needs_json_type {
                    request = request.header(http::header::CONTENT_TYPE, "application/json");
                }
                request = request.body(body);
            }
            let fetch = async {
                let mut response = request
                    .send()
                    .await
                    .map_err(|_| EnvError::Fetch("Request failed".into()))?;
                if matches!(response.status().as_u16(), 401 | 403) {
                    return Err(EnvError::Fetch(AUTH_REJECTED.into()));
                }
                if !response.status().is_success() {
                    return Err(EnvError::Fetch("HTTP status rejected".into()));
                }
                if response
                    .content_length()
                    .is_some_and(|n| n > RESPONSE_LIMIT as u64)
                {
                    return Err(EnvError::Fetch("Response too large".into()));
                }
                let mut bytes = Vec::new();
                while let Some(chunk) = response
                    .chunk()
                    .await
                    .map_err(|_| EnvError::Fetch("Response interrupted".into()))?
                {
                    if chunk.len() > RESPONSE_LIMIT.saturating_sub(bytes.len()) {
                        return Err(EnvError::Fetch("Response too large".into()));
                    }
                    bytes.extend_from_slice(&chunk);
                }
                decode(&bytes)
            };
            tokio::select! { biased;
                _ = env.canceled.cancelled() => Err(canceled()),
                result = fetch => result,
            }
        }
        .boxed()
    }
    fn get_storage<T: for<'de> Deserialize<'de> + ConditionalSend + 'static>(
        key: &str,
    ) -> TryEnvFuture<Option<T>> {
        let key = key.to_owned();
        let context = context();
        async move {
            let env = context?;
            let store = env
                .storage
                .lock()
                .map_err(|_| EnvError::StorageUnavailable)?;
            if env.canceled.is_cancelled() {
                return Err(canceled());
            }
            store.get(&key).map(|bytes| decode(bytes)).transpose()
        }
        .boxed()
    }
    fn set_storage<T: Serialize>(key: &str, value: Option<&T>) -> TryEnvFuture<()> {
        let values = value
            .map(serde_json::to_vec)
            .transpose()
            .map(|value| BTreeMap::from([(key.to_owned(), value)]))
            .map_err(|_| EnvError::Serde("Invalid storage value".into()));
        write_storage_values(context(), values)
    }
    fn exec_concurrent<F: Future<Output = ()> + ConditionalSend + 'static>(future: F) {
        spawn(future, false);
    }
    fn exec_sequential<F: Future<Output = ()> + ConditionalSend + 'static>(future: F) {
        spawn(future, true);
    }
    fn now() -> DateTime<Utc> {
        CONTEXT
            .try_with(|env| (env.clock)())
            .unwrap_or_else(|_| Utc::now())
    }
    fn flush_analytics() -> EnvFuture<'static, ()> {
        async {}.boxed()
    }
    fn client_info() -> ClientInfo {
        ClientInfo {
            platform: "tvos".into(),
            os: Some("tvos".into()),
            app_version: Some("0.1.0".into()),
            shell_version: None,
            installation_id: None,
            notification_templates: vec![],
        }
    }
    fn analytics_context(_: &Ctx, _: &StreamingServer, _: &str) -> serde_json::Value {
        serde_json::Value::Null
    }
    #[cfg(debug_assertions)]
    fn log(_: String) {} // Core logs can contain account/source data; do not forward them.
}
fn spawn(future: impl Future<Output = ()> + Send + 'static, sequential: bool) {
    let Ok(env) = context() else {
        return;
    };
    let Ok(guard) = env.storage.lock() else {
        return;
    };
    if env.canceled.is_cancelled() {
        return;
    }
    if sequential {
        let overflow = env.sequential.try_send(future.boxed()).is_err();
        drop(guard);
        if overflow {
            env.cancel();
        }
    } else {
        let scope = env.clone();
        env.tasks.spawn_on(
            async move {
                tokio::select! { biased;
                    _ = scope.canceled.cancelled() => {},
                    result = std::panic::AssertUnwindSafe(scope.scope(future)).catch_unwind() => {
                        if result.is_err() { scope.fail(); }
                    }
                }
            },
            &env.runtime,
        );
    }
}

#[cfg(test)]
mod storage_batch_tests {
    use super::*;

    #[tokio::test]
    async fn failed_batch_preserves_both_keys_and_retry_survives_reopen() {
        let directory = std::env::temp_dir().join(format!(
            "cue-storage-batch-{}-{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap()
        ));
        std::fs::create_dir(&directory).unwrap();
        let unavailable = directory.with_extension("unavailable");
        let updates = |first: u32, second: u32| {
            BTreeMap::from([
                ("recent".into(), Some(serde_json::to_vec(&first).unwrap())),
                ("older".into(), Some(serde_json::to_vec(&second).unwrap())),
            ])
        };
        let env = Environment::persistent(&directory, &[8; 32]).unwrap();
        env.scope(async {
            NativeEnv::set_storage_batch(updates(11, 22)).await.unwrap();
            std::fs::rename(&directory, &unavailable).unwrap();
            assert!(NativeEnv::set_storage_batch(updates(33, 44)).await.is_err());
            assert_eq!(
                NativeEnv::get_storage::<u32>("recent").await.unwrap(),
                Some(11)
            );
            assert_eq!(
                NativeEnv::get_storage::<u32>("older").await.unwrap(),
                Some(22)
            );
            std::fs::rename(&unavailable, &directory).unwrap();
        })
        .await;
        env.shutdown().await;
        drop(env);
        let env = Environment::persistent(&directory, &[8; 32]).unwrap();
        env.scope(async {
            assert_eq!(
                NativeEnv::get_storage::<u32>("recent").await.unwrap(),
                Some(11)
            );
            assert_eq!(
                NativeEnv::get_storage::<u32>("older").await.unwrap(),
                Some(22)
            );
            NativeEnv::set_storage_batch(updates(33, 44)).await.unwrap();
        })
        .await;
        env.shutdown().await;
        drop(env);
        let env = Environment::persistent(&directory, &[8; 32]).unwrap();
        env.scope(async {
            assert_eq!(
                NativeEnv::get_storage::<u32>("recent").await.unwrap(),
                Some(33)
            );
            assert_eq!(
                NativeEnv::get_storage::<u32>("older").await.unwrap(),
                Some(44)
            );
            // A rejected oversized batch must not partially delete an existing key.
            assert!(
                NativeEnv::set_storage_batch(BTreeMap::from([
                    ("recent".into(), Some(vec![0; STORAGE_LIMIT])),
                    ("older".into(), None),
                ]))
                .await
                .is_err()
            );
            assert_eq!(
                NativeEnv::get_storage::<u32>("recent").await.unwrap(),
                Some(33)
            );
            assert_eq!(
                NativeEnv::get_storage::<u32>("older").await.unwrap(),
                Some(44)
            );
            NativeEnv::set_storage_batch(BTreeMap::from([
                ("recent".into(), Some(b"55".to_vec())),
                ("older".into(), None),
            ]))
            .await
            .unwrap();
        })
        .await;
        env.shutdown().await;
        drop(env);
        let env = Environment::persistent(&directory, &[8; 32]).unwrap();
        env.scope(async {
            assert_eq!(
                NativeEnv::get_storage::<u32>("recent").await.unwrap(),
                Some(55)
            );
            assert_eq!(NativeEnv::get_storage::<u32>("older").await.unwrap(), None);
        })
        .await;
        env.shutdown().await;
        drop(env);
        std::fs::remove_dir_all(directory).unwrap();
    }
}
