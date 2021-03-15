use std::{convert::Infallible, os::linux::raw::stat, process::Stdio, time::Duration};

use tokio::process::Command;

use warp::Filter;
#[tokio::main]
async fn main() {
    let fs_s = warp::path("files").and(warp::fs::dir("/"));
    let hello = warp::path!("hello" / String).map(|name| format!("Hello, {}!", name));
    let runf = warp::path!("write"/ String).and_then(
        writer
    );

    warp::serve(hello.or(runf).or(fs_s)).run(([0, 0, 0, 0], std::env::var("PORT").unwrap_or_default().parse().unwrap_or_else(|x|3030))).await;
}

async fn writer(text:String) -> Result<impl warp::Reply, Infallible> {
    let mut child = Command::new("python")
        .arg("/handwriter/demo.py")
        .arg(format!("-i {}",text))
        .arg(format!("-o {}",text))
        .current_dir("/handwriter")
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to spawn");
    let status = child.wait_with_output().await.map(|o|format!("{:#?}",o)).unwrap_or_default();
    Ok(format!("Exited with code {:#?}", status))
}