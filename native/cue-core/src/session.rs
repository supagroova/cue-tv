//! Owns real Core models and effects; published snapshots contain no account secrets.
use crate::environment::{Environment, NativeEnv};
use serde::Serialize;
use std::{collections::VecDeque, sync::Arc};
use stremio_core::{
    constants::PROFILE_STORAGE_KEY,
    models::ctx::Ctx,
    runtime::{
        Effect, EffectFuture, Env, EnvError, Update,
        msg::{Action, ActionCtx, Event, Msg},
    },
    types::{notifications::NotificationsBucket, profile::Profile, server_urls::ServerUrlsBucket},
};
use tokio::sync::{mpsc, oneshot, watch};

#[derive(Clone, Debug, Serialize)]
pub struct Snapshot {
    pub revision: u64,
    pub stopped: bool,
    pub interface_language: String,
    pub addon_count: usize,
    pub pending_effects: usize,
    pub error: Option<&'static str>,
}
enum Command {
    Language(String, oneshot::Sender<Result<(), &'static str>>),
    Effect(Msg),
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
        let initial = snapshot(&ctx, 0, 0, false, None);
        let (publish, state) = watch::channel(initial);
        let env = environment.clone();
        let outgoing = commands.clone();
        let owner = tokio::spawn(async move {
            env.scope(run(ctx, inbox, outgoing, publish, env.clone()))
                .await;
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
    let mut revision = 0;
    let mut pending = 0usize;
    let mut error = None;
    loop {
        let command = tokio::select! { biased;
            _ = env.cancelled() => break,
            command = inbox.recv() => match command { Some(command) => command, None => break },
        };
        let (message, reply) = match command {
            Command::Language(language, reply) => {
                error = None;
                let mut settings = ctx.profile.settings.clone();
                settings.interface_language = language;
                (
                    Msg::Action(Action::Ctx(ActionCtx::UpdateSettings(settings))),
                    Some(reply),
                )
            }
            Command::Effect(message) => {
                pending = pending.saturating_sub(1);
                (message, None)
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
            let effects = <Ctx as Update<NativeEnv>>::update(&mut ctx, &message);
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
                                    let _ = sender.send(Command::Effect(future.await)).await;
                                })
                            }
                            EffectFuture::Sequential(future) => {
                                NativeEnv::exec_sequential(async move {
                                    let _ = sender.send(Command::Effect(future.await)).await;
                                })
                            }
                        }
                    }
                }
            }
        }
        publish.send_replace(snapshot(&ctx, revision, pending, false, error));
        if let Some(reply) = reply {
            let _ = reply.send(if error.is_none() {
                Ok(())
            } else {
                Err("core_effect_failed")
            });
        }
    }
    inbox.close();
    publish.send_replace(snapshot(&ctx, revision, 0, true, error));
}
