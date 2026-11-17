//! Owns real Core models and effects; published snapshots contain no account secrets.
use crate::environment::{Environment, NativeEnv};
use futures::FutureExt;
use serde::Serialize;
use std::{collections::VecDeque, sync::Arc};
use stremio_core::{
    constants::PROFILE_STORAGE_KEY,
    models::{
        addon_details::{AddonDetails, Selected as AddonSelected},
        common::Loadable,
        ctx::Ctx,
    },
    runtime::{
        Effect, EffectFuture, Effects, Env, EnvError, Update, UpdateWithCtx,
        msg::{Action, ActionCtx, ActionLoad, Event, Internal, Msg},
    },
    types::{notifications::NotificationsBucket, profile::Profile, server_urls::ServerUrlsBucket},
};
use tokio::sync::{mpsc, oneshot, watch};

#[derive(Clone, Debug, Serialize)]
pub struct AddonSnapshot {
    pub status: &'static str,
    pub installed: bool,
    pub id: Option<String>,
    pub name: Option<String>,
}
pub(crate) fn validate_addon_url(url: &str) -> Result<AddonSelected, &'static str> {
    if url.len() > 2048 {
        return Err("invalid_addon_url");
    }
    let selected: AddonSelected = serde_json::from_value(serde_json::json!({"transportUrl":url}))
        .map_err(|_| "invalid_addon_url")?;
    let url = &selected.transport_url;
    if !matches!(url.scheme(), "https" | "http")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || !url.path().ends_with("/manifest.json")
    {
        return Err("invalid_addon_url");
    }
    Ok(selected)
}
#[derive(Clone, Debug, Serialize)]
pub struct Snapshot {
    pub revision: u64,
    pub stopped: bool,
    pub interface_language: String,
    pub addon_count: usize,
    pub addon: Option<AddonSnapshot>,
    pub pending_effects: usize,
    pub error: Option<&'static str>,
}
enum Command {
    Language(String, oneshot::Sender<Result<(), &'static str>>),
    Addon(AddonSelected, oneshot::Sender<Result<(), &'static str>>),
    Install(u64, oneshot::Sender<Result<(), &'static str>>),
    Effect(Msg, u64),
}
pub struct CoreSession {
    environment: Arc<Environment>,
    commands: mpsc::Sender<Command>,
    state: watch::Receiver<Snapshot>,
    owner: Option<tokio::task::JoinHandle<()>>,
}
impl CoreSession {
    /// Guest-only entry until protected account storage and authentication are integrated.
    pub async fn guest(environment: Arc<Environment>) -> Result<Self, EnvError> {
        let setup = environment
            .scope(async {
                NativeEnv::migrate_storage_schema().await?;
                let profile = NativeEnv::get_storage::<Profile>(PROFILE_STORAGE_KEY)
                    .await?
                    .unwrap_or_default();
                if profile.auth.is_some() {
                    return Err(EnvError::Other("Account storage is not configured".into()));
                }
                Ok(Ctx::new(
                    profile,
                    Default::default(),
                    Default::default(),
                    ServerUrlsBucket::new::<NativeEnv>(None),
                    NotificationsBucket::new::<NativeEnv>(None, vec![]),
                    Default::default(),
                    Default::default(),
                ))
            })
            .await;
        let ctx = match setup {
            Ok(ctx) => ctx,
            Err(error) => {
                environment.shutdown().await;
                return Err(error);
            }
        };
        let (commands, inbox) = mpsc::channel(64);
        let initial = snapshot(&ctx, &AddonDetails::default(), 0, 0, false, None);
        let (publish, state) = watch::channel(initial);
        let env = environment.clone();
        let outgoing = commands.clone();
        let owner = tokio::spawn(async move {
            let result = std::panic::AssertUnwindSafe(env.scope(run(
                ctx,
                inbox,
                outgoing,
                publish.clone(),
                env.clone(),
            )))
            .catch_unwind()
            .await;
            if result.is_err() {
                env.fail();
                let mut state = publish.borrow().clone();
                state.stopped = true;
                state.pending_effects = 0;
                state.error = env.failure();
                publish.send_replace(state);
            }
        });
        Ok(Self {
            environment,
            commands,
            state,
            owner: Some(owner),
        })
    }
    pub fn snapshot(&self) -> Snapshot {
        self.state.borrow().clone()
    }
    pub async fn set_interface_language(&self, language: &str) -> Result<(), &'static str> {
        if language.len() != 3 || !language.bytes().all(|b| b.is_ascii_lowercase()) {
            return Err("invalid_language");
        }
        let (reply, ack) = oneshot::channel();
        self.commands
            .try_send(Command::Language(language.to_owned(), reply))
            .map_err(|_| "session_unavailable")?;
        ack.await.map_err(|_| "session_stopped")?
    }
    pub async fn load_addon(&self, url: &str) -> Result<(), &'static str> {
        let selected = validate_addon_url(url)?;
        let (reply, ack) = oneshot::channel();
        self.commands
            .try_send(Command::Addon(selected, reply))
            .map_err(|_| "session_unavailable")?;
        ack.await.map_err(|_| "session_stopped")?
    }
    pub async fn install_addon(&self, expected_revision: u64) -> Result<(), &'static str> {
        let (reply, ack) = oneshot::channel();
        self.commands
            .try_send(Command::Install(expected_revision, reply))
            .map_err(|_| "session_unavailable")?;
        ack.await.map_err(|_| "session_stopped")?
    }
    pub async fn settled(&mut self) -> Result<Snapshot, &'static str> {
        loop {
            let state = self.snapshot();
            if state.stopped {
                return Err("session_stopped");
            }
            if state.pending_effects == 0 {
                return Ok(state);
            }
            self.state.changed().await.map_err(|_| "session_stopped")?;
        }
    }
    pub async fn stop(&mut self) {
        self.environment.cancel();
        if let Some(owner) = self.owner.take() {
            let _ = owner.await;
        }
        self.environment.shutdown().await;
    }
}
impl Drop for CoreSession {
    fn drop(&mut self) {
        self.environment.cancel();
    }
}
fn snapshot(
    ctx: &Ctx,
    addon: &AddonDetails,
    revision: u64,
    pending_effects: usize,
    stopped: bool,
    error: Option<&'static str>,
) -> Snapshot {
    Snapshot {
        revision,
        stopped,
        interface_language: ctx.profile.settings.interface_language.clone(),
        addon_count: ctx.profile.addons.len(),
        addon: addon
            .remote_addon
            .as_ref()
            .map(|addon| match &addon.content {
                Loadable::Loading => AddonSnapshot {
                    status: "loading",
                    installed: false,
                    id: None,
                    name: None,
                },
                Loadable::Err(_) => AddonSnapshot {
                    status: "failed",
                    installed: false,
                    id: None,
                    name: None,
                },
                Loadable::Ready(value) => AddonSnapshot {
                    status: "ready",
                    installed: ctx.profile.addons.contains(value),
                    id: Some(value.manifest.id.chars().take(256).collect()),
                    name: Some(value.manifest.name.chars().take(256).collect()),
                },
            }),
        pending_effects,
        error,
    }
}
async fn run(
    mut ctx: Ctx,
    mut inbox: mpsc::Receiver<Command>,
    outgoing: mpsc::Sender<Command>,
    publish: watch::Sender<Snapshot>,
    env: Arc<Environment>,
) {
    let mut addon = AddonDetails::default();
    let mut addon_generation = 0u64;
    let mut revision = 0;
    let mut pending = 0usize;
    let mut error = None;
    loop {
        let command = tokio::select! { biased;
            _ = env.cancelled() => break,
            command = inbox.recv() => match command { Some(command) => command, None => break },
        };
        let (message, reply, origin) = match command {
            Command::Language(language, reply) => {
                error = None;
                let mut settings = ctx.profile.settings.clone();
                settings.interface_language = language;
                (
                    Msg::Action(Action::Ctx(ActionCtx::UpdateSettings(settings))),
                    Some(reply),
                    addon_generation,
                )
            }
            Command::Addon(selected, reply) => {
                error = None;
                if addon.selected.as_ref() != Some(&selected)
                    || addon
                        .remote_addon
                        .as_ref()
                        .is_some_and(|value| value.content.is_err())
                {
                    let Some(next) = addon_generation.checked_add(1) else {
                        env.fail();
                        break;
                    };
                    addon_generation = next;
                }
                (
                    Msg::Action(Action::Load(ActionLoad::AddonDetails(selected))),
                    Some(reply),
                    addon_generation,
                )
            }
            Command::Install(expected_revision, reply) => {
                if expected_revision != revision {
                    let _ = reply.send(Err("stale_preview"));
                    continue;
                }
                let Some(descriptor) = addon
                    .remote_addon
                    .as_ref()
                    .and_then(|value| value.content.ready())
                    .cloned()
                else {
                    let _ = reply.send(Err("addon_not_ready"));
                    continue;
                };
                error = None;
                (
                    Msg::Action(Action::Ctx(ActionCtx::InstallAddon(descriptor))),
                    Some(reply),
                    addon_generation,
                )
            }
            Command::Effect(message, generation) => {
                pending = pending.saturating_sub(1);
                (message, None, generation)
            }
        };
        let mut messages = VecDeque::from([message]);
        let mut handled = 0;
        while let Some(message) = messages.pop_front() {
            handled += 1;
            if handled > 1024 {
                error = Some("core_effect_limit");
                env.cancel();
                break;
            }
            if let Msg::Event(event) = message {
                if matches!(event, Event::Error { .. }) {
                    error = Some("core_effect_failed");
                }
                continue;
            }
            // Core caches a failed descriptor by URL. Explicit reselection permits
            // retry without unloading an in-flight request or a successful preview.
            let retry = if matches!(
                &message,
                Msg::Action(Action::Load(ActionLoad::AddonDetails(_)))
            ) && addon
                .remote_addon
                .as_ref()
                .is_some_and(|value| value.content.is_err())
            {
                <AddonDetails as UpdateWithCtx<NativeEnv>>::update(
                    &mut addon,
                    &Msg::Action(Action::Unload),
                    &ctx,
                )
            } else {
                Effects::none().unchanged()
            };
            let context_effects = <Ctx as Update<NativeEnv>>::update(&mut ctx, &message);
            let stale_manifest = origin != addon_generation
                && matches!(&message, Msg::Internal(Internal::ManifestRequestResult(..)));
            let addon_effects = if stale_manifest {
                Effects::none().unchanged()
            } else {
                <AddonDetails as UpdateWithCtx<NativeEnv>>::update(&mut addon, &message, &ctx)
            };
            let effects = retry.join(context_effects).join(addon_effects);
            if effects.has_changed {
                revision += 1;
            }
            for effect in effects {
                match effect {
                    Effect::Msg(message) => messages.push_back(*message),
                    Effect::Future(future) => {
                        pending += 1;
                        let sender = outgoing.clone();
                        match future {
                            EffectFuture::Concurrent(future) => {
                                NativeEnv::exec_concurrent(async move {
                                    let _ = sender
                                        .send(Command::Effect(future.await, addon_generation))
                                        .await;
                                })
                            }
                            EffectFuture::Sequential(future) => {
                                NativeEnv::exec_sequential(async move {
                                    let _ = sender
                                        .send(Command::Effect(future.await, addon_generation))
                                        .await;
                                })
                            }
                        }
                    }
                }
            }
        }
        publish.send_replace(snapshot(&ctx, &addon, revision, pending, false, error));
        if let Some(reply) = reply {
            let _ = reply.send(if error.is_none() {
                Ok(())
            } else {
                Err("core_effect_failed")
            });
        }
    }
    inbox.close();
    publish.send_replace(snapshot(
        &ctx,
        &addon,
        revision,
        0,
        true,
        env.failure().or(error),
    ));
}
