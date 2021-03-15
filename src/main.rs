use warp::Filter;
#[tokio::main]
async fn main() {
    let fs_s = warp::path("files").and(warp::fs::dir("/src/files"));
    let hello = warp::path!("hello" / String).map(|name| format!("Hello, {}!", name));

    warp::serve(hello).run(([0, 0, 0, 0], std::env::var("PORT").unwrap_or_default().parse().unwrap_or_else(|x|3030))).await;
}
