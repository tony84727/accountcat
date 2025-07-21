#[tokio::main]
async fn main() {
    warp::serve(warp::fs::dir("ui/dist"))
        .run(([0, 0, 0, 0], 3030))
        .await;
}
