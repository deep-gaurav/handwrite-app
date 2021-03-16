use std::{convert::Infallible, error, process::Stdio, time::Duration};

use tokio::process::Command;

use warp::{Filter, reject::Reject,http::Response};

use tokio::fs::File;
use tokio::io::AsyncReadExt; 

use serde::{Serialize,Deserialize};

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    let fs_s = warp::path("files").and(warp::fs::dir("/"));
    let hello = warp::path!("hello" / String).map(|name| format!("Hello, {}!", name));
    let runf = warp::path!("write"/ String).and_then(
        writer
    );
    let runf2 = warp::path!("write2"/ String).and_then(
        writer2
    );
    let runf3 = warp::path!("write3"/ String).and(warp::query::<HandParameters>()).and_then(
        writer3
    );
    warp::serve(hello.or(runf).or(runf2).or(runf3).or(fs_s)).run(([0, 0, 0, 0], std::env::var("PORT").unwrap_or_default().parse().unwrap_or_else(|x|3030))).await;
}

#[derive(Debug,Serialize,Deserialize)]
struct HandParameters{
    style:Option<u32>,
    bias:Option<f32>,
    color:Option<String>,
    width:Option<u32>,
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
async fn writer3(text:String,param:HandParameters) -> Result<impl warp::Reply, warp::reject::Rejection> {
    let filename = format!("/{}.svg",text);
    let mut child = Command::new("python")
        .arg("/handwriter/demo.py")
        .arg("-i")
        .arg(format!("{}",text))
        .arg("-o")
        .arg(format!("{}",filename))
        .arg("-s")
        .arg(param.style.unwrap_or(0).to_string())
        .arg("-b")
        .arg(param.bias.unwrap_or(0.75).to_string())
        .arg("-c")
        .arg(param.color.unwrap_or("blue".to_string()))
        .arg("-w")
        .arg(param.width.unwrap_or(1).to_string())
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
            let mut file = File::open(filename).await.map_err(|e|warp::reject::custom(ServerError::from(e)))?;
            let mut contents = vec![];
            file.read_to_end(&mut contents).await.map_err(|e|warp::reject::custom(ServerError::from(e)))?;
            let body = std::str::from_utf8(&contents).map_err(|e|warp::reject::custom(ServerError::from(e)))?.to_string();
            let reply = Response::builder().header("Content-Type", "image/svg+xml").body(body).map_err(|e|warp::reject::custom(ServerError::from(e)))?;
            Ok(reply)
        }
        Err(err) => {
            Err(warp::reject::custom(ServerError::from(err)))
        }
    }
}

async fn writer2(text:String) -> Result<impl warp::Reply, warp::reject::Rejection> {
    let filename = format!("/{}.svg",text);
    let mut child = Command::new("python")
        .arg("/handwriter/demo.py")
        .arg("-i")
        .arg(format!("{}",text))
        .arg("-o")
        .arg(format!("{}",filename))
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

            Ok(out.stdout)
        }
        Err(err) => {
            Err(warp::reject::custom(ServerError::from(err)))
        }
    }
}


async fn writer(text:String) -> Result<impl warp::Reply, warp::reject::Rejection> {
    let filename = format!("/{}.svg",text);
    let mut child = Command::new("python")
        .arg("/handwriter/demo.py")
        .arg("-i")
        .arg(format!("{}",text))
        .arg("-o")
        .arg(format!("{}",filename))
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