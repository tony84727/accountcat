use http::{HeaderName, HeaderValue, Request, Response};
use pin_project_lite::pin_project;
use std::task::{Poll, ready};
use tower::{Layer, Service};

pub fn build_csp(nonce: Option<&str>) -> HeaderValue {
    let nonce = nonce
        .map(|nonce| format!("'nonce-{nonce}'"))
        .unwrap_or_default();
    let directives = [
        format!("script-src 'self' https://accounts.google.com/gsi/client {nonce}"),
        format!(
            "style-src 'self' https://accounts.google.com/gsi/style 'sha256-ZAdCRDnStGum6I/Iqtz5uunKn4HysAVC/9iXTVObQr8=' {nonce}"
        ),
        String::from("connect-src 'self' https://accounts.google.com/gsi/status"),
        format!("frame-src https://accounts.google.com/gsi {nonce}"),
        String::from("default-src 'self' 'unsafe-inline'"),
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

pub struct CspLayer;

impl<S> Layer<S> for CspLayer {
    type Service = CspService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CspService {
            inner,
            header_name: HeaderName::from_static("content-security-policy"),
        }
    }
}

#[derive(Clone)]
pub struct CspService<S> {
    inner: S,
    header_name: HeaderName,
}

impl<S, T, O> Service<Request<T>> for CspService<S>
where
    S: Service<Request<T>, Response = Response<O>>,
{
    type Response = S::Response;

    type Error = S::Error;

    type Future = AttachPolicy<S::Future>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<T>) -> Self::Future {
        let nonce = req.extensions().get::<Nonce>();
        let nonce = nonce.cloned().map(|Nonce(n)| n.to_string());
        AttachPolicy {
            inner: self.inner.call(req),
            header_name: self.header_name.clone(),
            value: build_csp(nonce.as_deref()),
        }
    }
}

pin_project! {
    pub struct AttachPolicy<F> {
        #[pin]
        inner: F,
        header_name: HeaderName,
        value: HeaderValue,
    }
}

impl<F, T, E> Future for AttachPolicy<F>
where
    F: Future<Output = Result<Response<T>, E>>,
{
    type Output = F::Output;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.project();
        let mut response = ready!(this.inner.poll(cx)?);
        response
            .headers_mut()
            .insert(this.header_name.clone(), this.value.clone());
        Poll::Ready(Ok(response))
    }
}
