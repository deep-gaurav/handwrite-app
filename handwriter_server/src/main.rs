pub mod server;
pub mod worker;

use std::{error, process::Stdio};

use tokio::{process::Command, task::spawn_blocking};

use warp::{http::Response, reject::Reject, Filter};

use tokio::fs::File;
use tokio::io::AsyncReadExt;

use handwriter_shared::*;
use std::sync::Arc;
use tokio::sync::RwLock;

pub type Context = Arc<RwLock<Vec<Task>>>;

pub async fn complete_task(context: Context, id: &str, status: TaskStatus) {
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
    let worker_status = warp::path!("worker")
        .and(with_context.clone())
        .and_then(worker_status);
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

    let solver = tokio::spawn(async move {
        if let Some(workers) = std::env::var("WORKERS").ok() {
            let mut servers = workers.split(",").collect::<Vec<_>>();
            let servers = servers.into_iter().map(|s| s.to_string()).collect();
            server::server(context, servers).await
        } else {
            worker::worker(context).await;
        }
    });
    let route = (hello
        .or(create_task)
        .or(status_task)
        .or(fs_s)
        .or(worker_status))
    .with(cors);
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

async fn worker_status(context: Context) -> Result<impl warp::Reply, warp::reject::Rejection> {
    let con = context.read().await;
    let task = con.iter().find(|t| t.status.is_working());
    if let Some(task) = task {
        let body = serde_json::to_string(&WorkerStatus::Working(task.clone()))
            .map_err(|e| warp::reject::custom(ServerError::from(e)))?;

        let reply = Response::builder()
            .header("Content-Type", "application/json")
            .body(body)
            .map_err(|e| warp::reject::custom(ServerError::from(e)))?;
        Ok(reply)
    } else {
        let body = serde_json::to_string(&WorkerStatus::Available)
            .map_err(|e| warp::reject::custom(ServerError::from(e)))?;

        let reply = Response::builder()
            .header("Content-Type", "application/json")
            .body(body)
            .map_err(|e| warp::reject::custom(ServerError::from(e)))?;
        Ok(reply)
    }
}

async fn create(
    params: HandParameters,
    context: Context,
) -> Result<impl warp::Reply, warp::reject::Rejection> {
    let id = format!("{:x}", md5::compute(format!("{:#?}", params).as_bytes()));
    let filename = format!("/{}.svg", id);
    {
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
