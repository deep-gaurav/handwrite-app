use std::{convert::Infallible, error, process::Stdio, time::Duration};

use tokio::process::{self, Command};

use warp::{http::Response, reject::Reject, Filter, Rejection};

use tokio::fs::File;
use tokio::io::AsyncReadExt;

use serde::{Deserialize, Serialize};

use std::io::Error;
use std::process::Output;
use std::sync::Arc;
use tokio::sync::RwLock;

pub type Context = Arc<RwLock<Vec<Task>>>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub text: String,
    pub style: Option<u32>,
    pub bias: Option<f32>,
    pub color: Option<String>,
    pub width: Option<u32>,
    pub status: TaskStatus,
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Waiting,
    Completed(TaskCompleteTypes),
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum TaskCompleteTypes {
    Success(String),
    Failed(String),
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    let context = Context::default();
    let c2 = context.clone();
    let with_context = warp::any().map(move || c2.clone());
    let run = warp::path!("create")
        .and(warp::body::json::<HandParameters>())
        .and(with_context)
        .and_then(create);
    let fs_s = warp::path("files").and(warp::fs::dir("/"));
    let hello = warp::path!("hello" / String).map(|name| format!("Hello, {}!", name));

    let solver = async {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            let task: Option<Task> = {
                let c = context.write().await;
                let task_to_do = c.iter().find(|t| t.status == TaskStatus::Waiting);
                if let Some(task) = task_to_do {
                    Some(task.clone())
                } else {
                    None
                }
            };
            if let Some(task) = task {
                let taskc = task.clone();
                let filename = format!("/{}.svg", task.id);
                let child = Command::new("python")
                    .arg("/handwriter/demo.py")
                    .arg("-i")
                    .arg(format!("{}", task.text))
                    .arg("-o")
                    .arg(format!("{}", filename))
                    .arg("-s")
                    .arg(task.style.unwrap_or(0).to_string())
                    .arg("-b")
                    .arg(task.bias.unwrap_or(0.75).to_string())
                    .arg("-c")
                    .arg(task.color.unwrap_or("blue".to_string()))
                    .arg("-w")
                    .arg(task.width.unwrap_or(1).to_string())
                    .current_dir("/handwriter")
                    .stderr(Stdio::piped())
                    .stdout(Stdio::piped())
                    .spawn();
                match child {
                    Ok(child) => {
                        log::debug!("child created {:#?}", child);
                        let output = child.wait_with_output().await;
                        match output {
                            Ok(output) => {
                                let mut file = File::open(filename)
                                    .await
                                    .map_err(|e| warp::reject::custom(ServerError::from(e)));
                                if let Ok(mut file) = file {
                                    let mut contents = vec![];
                                    let f = file
                                        .read_to_end(&mut contents)
                                        .await
                                        .map_err(|e| warp::reject::custom(ServerError::from(e)));
                                    match f {
                                        Ok(_) => {
                                            let svg = std::str::from_utf8(&contents);
                                            match svg {
                                                Ok(svg) => {
                                                    let mut c = context.write().await;

                                                    let task =
                                                        c.iter_mut().find(|t| t.id == taskc.id);
                                                    if let Some(task) = task {
                                                        task.status = TaskStatus::Completed(
                                                            TaskCompleteTypes::Success(
                                                                svg.to_string(),
                                                            ),
                                                        );
                                                    }
                                                }
                                                Err(err) => {
                                                    let mut c = context.write().await;

                                                    let task =
                                                        c.iter_mut().find(|t| t.id == taskc.id);
                                                    if let Some(task) = task {
                                                        task.status = TaskStatus::Completed(
                                                            TaskCompleteTypes::Failed(format!(
                                                                "{:#?} {:#?}",
                                                                output, err
                                                            )),
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                        Err(err) => {
                                            let mut c = context.write().await;

                                            let task = c.iter_mut().find(|t| t.id == taskc.id);
                                            if let Some(task) = task {
                                                task.status = TaskStatus::Completed(
                                                    TaskCompleteTypes::Failed(format!(
                                                        "{:#?} {:#?}",
                                                        output, err
                                                    )),
                                                );
                                            }
                                        }
                                    }
                                } else {
                                    let mut c = context.write().await;

                                    let task = c.iter_mut().find(|t| t.id == taskc.id);
                                    if let Some(task) = task {
                                        task.status = TaskStatus::Completed(
                                            TaskCompleteTypes::Failed(format!("{:#?}", output)),
                                        );
                                    }
                                }
                            }
                            Err(err) => {
                                let mut c = context.write().await;

                                let task = c.iter_mut().find(|t| t.id == taskc.id);
                                if let Some(task) = task {
                                    task.status = TaskStatus::Completed(TaskCompleteTypes::Failed(
                                        format!("{:#?}", err),
                                    ));
                                }
                            }
                        }
                    }
                    Err(err) => {
                        let mut c = context.write().await;

                        let task = c.iter_mut().find(|t| t.id == taskc.id);
                        if let Some(task) = task {
                            task.status = TaskStatus::Completed(TaskCompleteTypes::Failed(
                                format!("{:#?}", err),
                            ));
                        }
                    }
                }
            }
        }
    };
    let warpav = warp::serve(hello.or(run).or(fs_s)).run((
        [0, 0, 0, 0],
        std::env::var("PORT")
            .unwrap_or_default()
            .parse()
            .unwrap_or_else(|x| 3030),
    ));
    tokio::join!(solver, warpav);
}

async fn load_file(filename: &str) -> Option<String> {
    let mut file = File::open(filename).await.ok()?;
    let mut contents = vec![];
    let f = file.read_to_end(&mut contents).await.ok()?;
    let st = std::str::from_utf8(&contents).ok()?.to_string();
    Some(st)
}

async fn create(
    params: HandParameters,
    context: Context,
) -> Result<impl warp::Reply, warp::reject::Rejection> {
    let id = format!("{:x}", md5::compute(format!("{:#?}", params).as_bytes()));
    let filename = format!("/{}.svg", id);
    if let Some(svg) = load_file(&filename).await {
        let task = Task {
            id: id,
            text: params.text,
            style: params.style,
            bias: params.bias,
            color: params.color,
            width: params.width,
            status: TaskStatus::Completed(TaskCompleteTypes::Success(svg)),
        };
        let body =
            serde_json::to_string(&task).map_err(|e| warp::reject::custom(ServerError::from(e)))?;
        let reply = Response::builder()
            .header("Content-Type", "application/json")
            .body(body)
            .map_err(|e| warp::reject::custom(ServerError::from(e)))?;

        Ok(reply)
    } else {
        let con = context.read().await;
        let task = con.iter().find(|t| t.id == id);
        if let Some(task) = task {
            let body = serde_json::to_string(task)
                .map_err(|e| warp::reject::custom(ServerError::from(e)))?;

            let reply = Response::builder()
                .header("Content-Type", "application/json")
                .body(body)
                .map_err(|e| warp::reject::custom(ServerError::from(e)))?;
            Ok(reply)
        } else {
            drop(con);
            let task = Task {
                id: id,
                text: params.text,
                style: params.style,
                bias: params.bias,
                color: params.color,
                width: params.width,
                status: TaskStatus::Waiting,
            };
            context.write().await.push(task.clone());
            let body = serde_json::to_string(&task)
                .map_err(|e| warp::reject::custom(ServerError::from(e)))?;

            let reply = Response::builder()
                .header("Content-Type", "application/json")
                .body(body)
                .map_err(|e| warp::reject::custom(ServerError::from(e)))?;
            Ok(reply)
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct HandParameters {
    text: String,
    style: Option<u32>,
    bias: Option<f32>,
    color: Option<String>,
    width: Option<u32>,
}

#[derive(Debug)]
struct ServerError {
    error: String,
}

impl Reject for ServerError {}

impl<T> From<T> for ServerError
where
    T: error::Error,
{
    fn from(e: T) -> Self {
        Self {
            error: format!("{:#?}", e),
        }
    }
}
