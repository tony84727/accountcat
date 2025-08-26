use std::{convert::Infallible, io, path::PathBuf, task::Poll};

use axum::body::Body;
use http::{HeaderValue, Request, Response, StatusCode, header};
use mime_guess::mime;
use tower::Service;

use crate::csp::Nonce;

#[derive(Clone)]
pub struct ServeDist {
    root: PathBuf,
}

impl ServeDist {
    pub fn new(root: PathBuf) -> io::Result<Self> {
        Ok(Self {
            root: root.canonicalize()?,
        })
    }
}

impl<B> Service<Request<B>> for ServeDist {
    type Response = Response<Body>;

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
    let joined = match joined.canonicalize() {
        Ok(joined) => joined,
        Err(_err) => {
            return root.join("index.html");
        }
    };
    if !joined.starts_with(root) || !joined.is_file() {
        return root.join("index.html");
    }
    joined
}

const ASSET_CSP_NONCE_PLACEHOLDER: &str = "__CSP_NONCE__";

fn inject_nonce(Nonce(nonce): &Nonce, source: Vec<u8>) -> Vec<u8> {
    match String::from_utf8(source) {
        Ok(file_string) => file_string
            .replace(ASSET_CSP_NONCE_PLACEHOLDER, nonce)
            .into_bytes(),
        Err(err) => err.into_bytes(),
    }
}

impl<B> Future for ServeDistFuture<B> {
    type Output = Result<Response<Body>, Infallible>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let mut f = Box::pin(async {
            let Some(nonce) = self.req.extensions().get::<Nonce>() else {
                return Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(String::from("500 internal server error").into())
                    .unwrap());
            };
            let path = build_path(&self.root, self.req.uri().path());
            let mime_type = mime_guess::from_path(&path)
                .first_raw()
                .map(HeaderValue::from_static)
                .unwrap_or_else(|| {
                    HeaderValue::from_static(mime::APPLICATION_OCTET_STREAM.as_ref())
                });
            let Ok(output) = std::fs::read(path) else {
                return Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(String::from("404 not found").into())
                    .unwrap());
            };
            let origin_len = output.len();
            let output = inject_nonce(nonce, output);
            let vary_by_nonce = origin_len != output.len();
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_LENGTH, output.len())
                .header(header::CONTENT_TYPE, mime_type)
                .header(
                    header::CACHE_CONTROL,
                    if vary_by_nonce {
                        "private, no-store"
                    } else {
                        "public, max-age=31536000, immutable"
                    },
                )
                .body(output.into())
                .unwrap())
        });
        f.as_mut().poll(cx)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs::File,
        io::{Read, Write},
        path::PathBuf,
    };

    use axum::body::{Body, Bytes};
    use http::Request;
    use http_body_util::BodyExt;
    use temp_dir::TempDir;
    use tower::{ServiceBuilder, ServiceExt};

    use crate::{
        csp::NonceLayer,
        serve_dist::{ServeDist, build_path},
    };

    #[test]
    fn test_build_path_not_exist_to_index() {
        assert_eq!(
            PathBuf::from("index.html"),
            build_path(&PathBuf::new(), "/notexistfile.jpg")
        );
    }

    #[test]
    fn test_cannot_access_parent() {
        let test_dir = TempDir::new().unwrap();
        let dist_dir_path = test_dir.path().join("dist");
        std::fs::create_dir(&dist_dir_path).unwrap();
        let outside_file_path = test_dir.path().join("outside.html");
        File::create(&outside_file_path).unwrap();
        assert_eq!(
            dist_dir_path.join("index.html"),
            build_path(&dist_dir_path, "/../outside.html")
        );
    }

    fn create_dummy_file(directory: PathBuf, name: &str) {
        let mut file = File::create(directory.join(name)).unwrap();
        file.write_all(format!("content of {name}").as_bytes())
            .unwrap();
    }

    struct ChangeCwd {
        original: PathBuf,
    }

    impl ChangeCwd {
        fn new(to: PathBuf) -> Self {
            let instance = Self {
                original: std::env::current_dir().unwrap(),
            };
            std::env::set_current_dir(to).unwrap();
            instance
        }
    }

    impl Drop for ChangeCwd {
        fn drop(&mut self) {
            std::env::set_current_dir(&self.original).unwrap();
        }
    }

    #[tokio::test]
    async fn test_serve_dist_allow_specifying_relative_path() {
        let test_dir = TempDir::new().unwrap();
        let assert_dir_path = test_dir.path().join("assets");
        std::fs::create_dir(&assert_dir_path).unwrap();
        create_dummy_file(assert_dir_path.clone(), "index.html");
        create_dummy_file(assert_dir_path, "a.txt");
        let _change_cwd = ChangeCwd::new(test_dir.path().into());
        let serve_dist = ServeDist::new(PathBuf::from("assets")).unwrap();
        let service = ServiceBuilder::new().layer(NonceLayer).service(serve_dist);
        let router = axum::Router::new().fallback_service(service);
        let response = router
            .oneshot(
                Request::builder()
                    .uri("/a.txt")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let response = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(Bytes::from_static(b"content of a.txt"), response);
    }

    #[tokio::test]
    async fn test_serve_binary() {
        let image = "ui/src/logo.png";
        let test_dir = TempDir::new().unwrap();
        std::fs::copy(image, test_dir.path().join("logo.png")).unwrap();
        let serve_dist = ServeDist::new(test_dir.path().into()).unwrap();
        let service = ServiceBuilder::new().layer(NonceLayer).service(serve_dist);
        let router = axum::Router::new().fallback_service(service);
        let response = router
            .oneshot(
                Request::builder()
                    .uri("/logo.png")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let response = response.into_body().collect().await.unwrap().to_bytes();
        let expected_bytes = {
            let mut f = File::open(image).unwrap();
            let mut buf = Vec::new();
            f.read_to_end(&mut buf).unwrap();
            buf
        };
        assert_eq!(expected_bytes, response);
    }
}
