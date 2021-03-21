use std::{error, process::Stdio};

use tokio::{process::Command, task::spawn_blocking};

use warp::{http::Response, reject::Reject, Filter};

use tokio::fs::File;
use tokio::io::AsyncReadExt;

use handwriter_shared::*;
use std::sync::Arc;
use tokio::sync::RwLock;

pub type Context = Arc<RwLock<Vec<Task>>>;

async fn complete_task(context: Context, id: &str, status: TaskStatus) {
    let mut c = context.write().await;

    let task = c.iter_mut().find(|t| t.id == id);
    if let Some(task) = task {
        task.status = status;
        c.iter_mut()
            .filter(|t| t.status.is_waiting())
            .enumerate()
            .for_each(|(i, f)| {
                if let TaskStatus::Waiting(p) = &mut f.status {
                    *p = (i + 1) as u32;
                }
            });
    }
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    let context = Context::default();
    let c2 = context.clone();
    let with_context = warp::any().map(move || c2.clone());
    let create_task = warp::path!("create")
        .and(warp::body::json::<HandParameters>())
        .and(with_context.clone())
        .and_then(create);
    let status_task = warp::path!("status" / String)
        .and(with_context)
        .and_then(status);
    let fs_s = warp::path("files").and(warp::fs::dir("/"));
    let hello = warp::path!("hello" / String).map(|name| format!("Hello, {}!", name));

    let cors = warp::cors()
        .allow_any_origin()
        .allow_headers(vec![
            "User-Agent",
            "Sec-Fetch-Mode",
            "Referer",
            "Origin",
            "Access-Control-Request-Method",
            "Access-Control-Request-Headers",
            "Content-Type",
        ])
        .allow_methods(vec!["POST", "GET"]);

    let solver = async {
        let hgen = tokio::task::spawn_blocking(|| {
            let hgen = handwriter::HandWritingGen::new(true, true);
            hgen
        })
        .await;
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            let task: Option<Task> = {
                let c = context.write().await;
                let task_to_do = c.iter().find(|t| t.status.is_waiting());
                if let Some(task) = task_to_do {
                    Some(task.clone())
                } else {
                    None
                }
            };
            if let Some(task) = task {
                let taskc = task.clone();
                let filename = format!("/{}.svg", task.id);
                match hgen {
                    Ok(ref hgen) => match hgen {
                        Ok(hgen) => {
                            let hgenf = hgen.clone();
                            let taskc = task.clone();
                            let svg = tokio::task::spawn_blocking(move|| {
                                hgenf.gen_svg(
                                    &taskc.text,
                                    taskc.style.unwrap_or(0),
                                    taskc.bias.unwrap_or(0.75),
                                    &taskc.color.unwrap_or("blue".to_string()),
                                    taskc.width.unwrap_or(1) as f32,
                                )
                            })
                            .await;
                            match svg {
                                Ok(svg) => match svg {
                                    Ok(svg) => {
                                        complete_task( context.clone(), &task.id, TaskStatus::Completed(
                                        TaskCompleteTypes::Success(
                                            SuccessResult{
                                                url:format!("https://handwrite.herokuapp.com/image/{}.svg",task.id.clone()),
                                                svg:svg,
                                            }
                                        ),
                                    )).await;
                                    }
                                    Err(err) => {
                                        complete_task(
                                            context.clone(),
                                            &task.id,
                                            TaskStatus::Completed(TaskCompleteTypes::Failed(
                                                format!("child spawn failed {:#?}", err),
                                            )),
                                        )
                                        .await;
                                    }
                                },
                                Err(err) => {
                                    complete_task(
                                        context.clone(),
                                        &task.id,
                                        TaskStatus::Completed(TaskCompleteTypes::Failed(format!(
                                            "child spawn failed {:#?}",
                                            err
                                        ))),
                                    )
                                    .await;
                                }
                            }
                        }
                        Err(err) => {
                            complete_task(
                                context.clone(),
                                &taskc.id,
                                TaskStatus::Completed(TaskCompleteTypes::Failed(format!(
                                    "child spawn failed {:#?}",
                                    err
                                ))),
                            )
                            .await;
                        }
                    },
                    Err(ref err) => {
                        complete_task(
                            context.clone(),
                            &taskc.id,
                            TaskStatus::Completed(TaskCompleteTypes::Failed(format!(
                                "child spawn failed {:#?}",
                                err
                            ))),
                        )
                        .await;
                    }
                }
            }
        }
    };
    let route = (hello.or(create_task).or(status_task).or(fs_s)).with(cors);
    let warpav = warp::serve(route).run((
        [0, 0, 0, 0],
        std::env::var("PORT")
            .unwrap_or_default()
            .parse()
            .unwrap_or_else(|_x| 3030),
    ));
    tokio::join!(solver, warpav);
}

async fn load_file(filename: &str) -> Option<String> {
    let mut file = File::open(filename).await.ok()?;
    let mut contents = vec![];
    let _f = file.read_to_end(&mut contents).await.ok()?;
    let st = std::str::from_utf8(&contents).ok()?.to_string();
    Some(st)
}

async fn status(id: String, context: Context) -> Result<impl warp::Reply, warp::reject::Rejection> {
    let con = context.read().await;
    let task = con.iter().find(|t| t.id == id);
    if let Some(task) = task {
        let body =
            serde_json::to_string(task).map_err(|e| warp::reject::custom(ServerError::from(e)))?;

        let reply = Response::builder()
            .header("Content-Type", "application/json")
            .body(body)
            .map_err(|e| warp::reject::custom(ServerError::from(e)))?;
        Ok(reply)
    } else {
        Err(warp::reject::not_found())
    }
}

async fn create(
    params: HandParameters,
    context: Context,
) -> Result<impl warp::Reply, warp::reject::Rejection> {
    let id = format!("{:x}", md5::compute(format!("{:#?}", params).as_bytes()));
    let filename = format!("/{}.svg", id);
    if let Some(svg) = load_file(&filename).await {
        let task = Task {
            id: id.clone(),
            text: params.text,
            style: params.style,
            bias: params.bias,
            color: params.color,
            width: params.width,
            status: TaskStatus::Completed(TaskCompleteTypes::Success(SuccessResult {
                url: format!("https://handwrite.herokuapp.com/files/{}.svg", id),
                svg: svg.to_string(),
            })),
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
            let openq = con.iter().filter(|t| t.status.is_waiting()).count();

            drop(con);
            let task = Task {
                id: id,
                text: params.text,
                style: params.style,
                bias: params.bias,
                color: params.color,
                width: params.width,
                status: TaskStatus::Waiting((openq + 1) as u32),
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
