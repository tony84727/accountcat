use http::{HeaderValue, Request};
use tower::{Layer, Service};

pub fn build_csp() -> HeaderValue {
    let directives = [
        "script-src 'self' https://accounts.google.com/gsi/client",
        "style-src 'self' https://accounts.google.com/gsi/style 'sha256-ZAdCRDnStGum6I/Iqtz5uunKn4HysAVC/9iXTVObQr8='",
        "connect-src 'self' https://accounts.google.com/gsi/status",
        "frame-src https://accounts.google.com/gsi",
        "default-src 'self' 'unsafe-inline'",
    ];
    HeaderValue::from_str(&(directives.join("; ") + ";")).unwrap()
}

pub struct NonceLayer;

impl<S> Layer<S> for NonceLayer {
    type Service = NonceService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        NonceService { inner }
    }
}
#[derive(Clone)]
pub struct NonceService<S> {
    inner: S,
}

#[derive(Clone)]
struct Nonce(uuid::Uuid);

impl<S, T> Service<Request<T>> for NonceService<S>
where
    S: Service<Request<T>>,
{
    type Response = S::Response;

    type Error = S::Error;

    type Future = S::Future;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<T>) -> Self::Future {
        let extensions = req.extensions_mut();
        extensions.insert(Nonce(uuid::Uuid::new_v4()));
        self.inner.call(req)
    }
}
