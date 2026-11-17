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

const RESPONSE_LIMIT: usize = 4 * 1024 * 1024;
const STORAGE_LIMIT: usize = 16 * 1024 * 1024;
tokio::task_local! { static CONTEXT: Arc<Environment>; }

/// One context per Core session. The default is memory-only; optional persistence
/// encrypts the entire state before writing to the app-owned directory.
pub struct Environment {
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

pub struct NativeEnv;
impl Env for NativeEnv {
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
            let uri = parts.uri.to_string();
            if !matches!(parts.uri.scheme_str(), Some("http" | "https")) {
                return Err(EnvError::Fetch("Unsupported URL scheme".into()));
            }
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
                request = request.body(body);
            }
            let fetch = async {
                let mut response = request
                    .send()
                    .await
                    .map_err(|_| EnvError::Fetch("Request failed".into()))?;
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
        let key = key.to_owned();
        let context = context();
        let value = value
            .map(serde_json::to_vec)
            .transpose()
            .map_err(|_| EnvError::Serde("Invalid storage value".into()));
        async move {
            let env = context?;
            let value = value?;
            let mut store = env
                .storage
                .lock()
                .map_err(|_| EnvError::StorageUnavailable)?;
            if env.canceled.is_cancelled() {
                return Err(canceled());
            }
            let remaining: usize = store
                .iter()
                .filter(|(k, _)| *k != &key)
                .map(|(k, v)| k.len() + v.len())
                .sum();
            if value
                .as_ref()
                .is_some_and(|v| key.len() + v.len() > STORAGE_LIMIT.saturating_sub(remaining))
            {
                return Err(EnvError::StorageWriteError(
                    "Storage budget exceeded".into(),
                ));
            }
            if let Some(persistence) = &env.persistence {
                let mut next = store.clone();
                if let Some(value) = value {
                    next.insert(key, value);
                } else {
                    next.remove(&key);
                }
                persistence.save(&next).map_err(|_| {
                    EnvError::StorageWriteError("Encrypted storage write failed".into())
                })?;
                *store = next;
            } else if let Some(value) = value {
                store.insert(key, value);
            } else {
                store.remove(&key);
            }
            Ok(())
        }
        .boxed()
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
