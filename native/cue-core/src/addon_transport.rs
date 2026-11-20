//! Add-on content is supported; unsolicited activity reporting is not.
use futures::FutureExt;
use stremio_core::{
    addon_transport::{AddonHTTPTransport, AddonTransport, UnsupportedTransport},
    constants::{LIBRARY_RESOURCE_NAME, PLAYER_RESOURCE_NAME},
    runtime::{EnvError, TryEnvFuture},
    types::addon::{Manifest, ResourcePath, ResourceResponse},
};

pub(crate) fn transport(url: &reqwest::Url) -> Box<dyn AddonTransport> {
    let inner: Box<dyn AddonTransport> = match url.scheme() {
        "http" | "https" => Box::new(AddonHTTPTransport::<crate::environment::NativeEnv>::new(
            url.clone(),
        )),
        _ => Box::new(UnsupportedTransport::new(url.clone())),
    };
    Box::new(ContentTransport { inner })
}

struct ContentTransport {
    inner: Box<dyn AddonTransport>,
}
impl AddonTransport for ContentTransport {
    fn manifest(&self) -> TryEnvFuture<Manifest> {
        self.inner.manifest()
    }
    fn resource(&self, path: &ResourcePath) -> TryEnvFuture<ResourceResponse> {
        if matches!(
            path.resource.as_str(),
            PLAYER_RESOURCE_NAME | LIBRARY_RESOURCE_NAME
        ) {
            // Return a local policy error; never claim that an external event was sent.
            // Core's AddonEventResult is informational, separate from library persistence.
            return async {
                Err(EnvError::AddonTransport(
                    "Activity reporting disabled".into(),
                ))
            }
            .boxed();
        }
        let request = self.inner.resource(path);
        if path.resource != "catalog" {
            return request;
        }
        // Capture the generation while Core constructs the effect, not when it is polled.
        let cancellation = crate::environment::catalog_request_cancellation();
        async move {
            let cancellation = cancellation?;
            tokio::select! { biased;
                _ = cancellation.cancelled() => Err(EnvError::Other("Catalogue request superseded".into())),
                result = request => result,
            }
        }.boxed()
    }
}
