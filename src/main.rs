use std::{convert::Infallible, error, process::Stdio, time::Duration};

use tokio::process::Command;

use warp::{Filter, reject::Reject};

use tokio::fs::File;
use tokio::io::AsyncReadExt; 

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    let fs_s = warp::path("files").and(warp::fs::dir("/"));
    let hello = warp::path!("hello" / String).map(|name| format!("Hello, {}!", name));
    let runf = warp::path!("write"/ String).and_then(
        writer
    );

    warp::serve(hello.or(runf).or(fs_s)).run(([0, 0, 0, 0], std::env::var("PORT").unwrap_or_default().parse().unwrap_or_else(|x|3030))).await;
}

#[derive(Debug)]
struct ServerError{
    error:String
}

impl Reject for ServerError {}

impl<T> From<T> for ServerError
    where T:error::Error
{

fn from(e: T) -> Self { Self{ error:format!("{:#?}",e)} }
}

async fn writer(text:String) -> Result<impl warp::Reply, warp::reject::Rejection> {
    let filename = format!("/{}.svg",text);
    let mut child = Command::new("python")
        .arg("/handwriter/demo.py")
        .arg(format!("-i {}",text))
        .arg(format!("-o {}",filename))
        .current_dir("/handwriter")
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to spawn");
    log::debug!("child created {:#?}", child);

    let status = child.wait_with_output().await.map(|o|format!("{:#?}",o));

    log::debug!("Output {:#?}",status);

    match status {
        Ok(status) => {
            let mut child = Command::new("cat")
            .arg(filename)
            .current_dir("/handwriter")
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("failed to spawn");

            log::debug!("cat child {:#?}",child);
            let out = child.wait_with_output().await.map_err(|e|warp::reject::custom(ServerError::from(e)))?;
            log::debug!("cat out {:#?}",out);

            Ok(format!("{:#?}",out))
        }
        Err(err) => {
            Err(warp::reject::custom(ServerError::from(err)))
        }
    }
}