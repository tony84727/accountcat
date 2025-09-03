use std::{fs::File, io::Write, path::PathBuf};

use accountcat::{csp::NonceLayer, serve_dist::ServeDist, testing::cwd::ChangeCwd};
use axum::body::Body;
use http::Request;
use http_body_util::BodyExt;
use temp_dir::TempDir;
use tower::{ServiceBuilder, ServiceExt};

fn create_dummy_file(directory: PathBuf, name: &str) {
    let mut file = File::create(directory.join(name)).unwrap();
    file.write_all(format!("content of {name}").as_bytes())
        .unwrap();
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
    assert_eq!(
        axum::body::Bytes::from_static(b"content of a.txt"),
        response
    );
}
