use std::{convert::Infallible, path::PathBuf, task::Poll};

use http::{HeaderValue, Request, Response, StatusCode, header};
use mime_guess::mime;
use tower::Service;

use crate::csp::Nonce;

#[derive(Clone)]
pub struct ServeDist {
    root: PathBuf,
}

impl ServeDist {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

impl<B> Service<Request<B>> for ServeDist {
    type Response = Response<String>;

    type Error = Infallible;

    type Future = ServeDistFuture<B>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        ServeDistFuture {
            root: self.root.clone(),
            req,
        }
    }
}

pub struct ServeDistFuture<B> {
    root: PathBuf,
    req: Request<B>,
}

fn build_path(root: &PathBuf, path: &str) -> PathBuf {
    let joined = root.clone().join(path.strip_prefix("/").unwrap());
    if !joined.starts_with(root) || !joined.is_file() {
        return root.join("index.html");
    }
    joined
}

fn inject_nonce(Nonce(nonce): &Nonce, source: &str) -> String {
    source.replace("__CSP_NONCE__", nonce)
}

impl<B> Future for ServeDistFuture<B> {
    type Output = Result<Response<String>, Infallible>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let mut f = Box::pin(async {
            let Some(nonce) = self.req.extensions().get::<Nonce>() else {
                return Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(String::from("500 internal server error"))
                    .unwrap());
            };
            let path = build_path(&self.root, self.req.uri().path());
            let mime_type = mime_guess::from_path(&path)
                .first_raw()
                .map(HeaderValue::from_static)
                .unwrap_or_else(|| {
                    HeaderValue::from_static(mime::APPLICATION_OCTET_STREAM.as_ref())
                });
            let Ok(output) = std::fs::read_to_string(path) else {
                return Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(String::from("404 not found"))
                    .unwrap());
            };
            let output = inject_nonce(nonce, &output);
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_LENGTH, output.len())
                .header(header::CONTENT_TYPE, mime_type)
                .body(output)
                .unwrap())
        });
        f.as_mut().poll(cx)
    }
}
