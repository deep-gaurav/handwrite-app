use std::{error, fmt::format, process::Stdio};

use handwriter::{HandWritingGen, Stroke};
use tokio::{io::split, process::Command, task::spawn_blocking};

use warp::{http::Response, reject::Reject, Filter};

use tokio::fs::File;
use tokio::io::AsyncReadExt;

use futures::FutureExt;
use futures::TryFutureExt;
use handwriter_shared::*;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::complete_task;
use crate::Context;

use std::collections::HashMap;

enum LineStatus {
    NotStarted,
    Assigned(String, Task),
    Completed(String, Task),
}

impl LineStatus {
    /// Returns `true` if the line_status is [`NotStarted`].
    fn is_not_started(&self) -> bool {
        matches!(self, Self::NotStarted)
    }

    /// Returns `true` if the line_status is [`Assigned`].
    fn is_assigned(&self) -> bool {
        matches!(self, Self::Assigned(..))
    }

    /// Returns `true` if the line_status is [`Completed`].
    fn is_completed(&self) -> bool {
        matches!(self, Self::Completed(..))
    }
}

fn accomodate_list_to_character_limit(content: &str) -> Vec<String> {
    let mut final_lines = vec![];
    let max_length = 70;
    for line in content.lines() {
        if line.len() > max_length {
            fn split_line(line: &str, max_size: usize) -> Vec<String> {
                if line.len() < max_size {
                    vec![line.to_string()]
                } else {
                    let ptosplit = line[..max_size].rfind(' ');
                    if let Some(ptosplit) = ptosplit {
                        if ptosplit < line.len() - 1 {
                            let mut splits = vec![line[..ptosplit + 1].to_string()];
                            splits.append(&mut split_line(&line[ptosplit + 1..], max_size));
                            splits
                        } else {
                            let mut splits = vec![line[..max_size].to_string()];
                            splits.append(&mut split_line(&line[max_size..], max_size));
                            splits
                        }
                    } else {
                        let mut splits = vec![line[..max_size].to_string()];
                        splits.append(&mut split_line(&line[max_size..], max_size));
                        splits
                    }
                }
            }
            final_lines.append(&mut split_line(line, max_length));
        } else {
            final_lines.push(line.to_string())
        }
    }
    final_lines
}

async fn complete_line(
    workers: Arc<RwLock<HashMap<String, WorkerStatus>>>,
    task: HandParameters,
    maintaskid: String,
    context: Context,
) -> Result<(String, Task), anyhow::Error> {
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        let worker = {
            let mut workers = workers.write().await;
            let worker = workers.iter_mut().find(|(w, status)| status.is_available());
            if let Some(worker) = worker {
                *worker.1 = WorkerStatus::Working(Task {
                    id: "".to_string(),
                    text: task.text.to_string(),
                    style: task.style,
                    bias: task.bias,
                    color: task.color.clone(),
                    width: task.width,
                    status: TaskStatus::Working(0, 0),
                });
                Some(worker.0.clone())
            } else {
                None
            }
        };
        if let Some(worker) = worker {
            let client = reqwest::Client::new();
            let res = client
                .post(format!("http://{}/create", worker))
                .json(&task)
                .send()
                .await?;
            if res.status().is_success() {
                tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                let res = client
                    .get(format!("http://{}/status", worker))
                    .send()
                    .await?;
                let response = res.json::<Task>().await?;
                if response.status.is_completed() {
                    let mut c = context.write().await;
                    let maintask = c.iter_mut().find(|f| f.id == maintaskid);
                    if let Some(task) = maintask {
                        if let TaskStatus::Working(done, total) = &mut task.status {
                            *done += 1;
                        }
                    }
                    return Ok((task.text.clone(), response));
                }
            } else {
                log::error!("task {:#?} to worker {} failed {:#?}", task, worker, res);
                return Err(anyhow::anyhow!(format!(
                    "task {:#?} to worker {} failed {:#?}",
                    task, worker, res
                )));
            }
        }
    }
}

pub async fn server(context: Context, servers: Vec<String>) {
    let workers: Arc<RwLock<HashMap<String, WorkerStatus>>> = Arc::new(RwLock::new(HashMap::new()));
    let workersclone = workers.clone();
    let refresher = tokio::task::spawn(async move {
        let mut refreshers = vec![];
        for server in servers.into_iter() {
            let workerscloneserver = workersclone.clone();
            let fut = tokio::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    let serverid = server.clone();
                    let req = reqwest::get(format!("https://{}/worker", server))
                        .and_then(|f| f.json::<WorkerStatus>())
                        .await;
                    match req {
                        Ok(worker) => {
                            let mut workers = workerscloneserver.write().await;
                            workers.insert(serverid, worker);
                        }
                        Err(err) => {
                            log::debug!("Worker {:#?} refresh failed {:#?}", serverid, err)
                        }
                    }
                }
            });
            refreshers.push(fut);
        }
        futures::future::join_all(refreshers).await;
    });

    let manager = tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            let task: Option<Task> = {
                let mut c = context.write().await;
                let task_to_do = c.iter_mut().find(|t| t.status.is_waiting());
                if let Some(task) = task_to_do {
                    let lines = accomodate_list_to_character_limit(&task.text);
                    task.status = TaskStatus::Working(0, lines.len() as u32);

                    Some(task.clone())
                } else {
                    None
                }
            };
            if let Some(task) = task {
                let taskc = task.clone();
                let lines = accomodate_list_to_character_limit(&taskc.text);
                let futs = lines.into_iter().map(|line| {
                    let param = HandParameters {
                        text: line,
                        style: task.style,
                        bias: task.bias,
                        color: task.color.clone(),
                        width: task.width,
                    };
                    complete_line(workers.clone(), param, task.id.clone(), context.clone())
                });
                let f = futures::future::try_join_all(futs).await;
                match f {
                    Ok(tasks) => {
                        let mut strokesall = vec![];
                        let mut failed = false;
                        for td in tasks {
                            if let Some(succ) = td.1.status.as_completed() {
                                if let Some(succtype) = succ.as_success() {
                                    strokesall.push(succtype.stroke.clone())
                                } else {
                                    log::error!("task failed by worker {:#?} ", td.1);
                                    failed = true;
                                    complete_task(
                                        context.clone(),
                                        &taskc.id,
                                        TaskStatus::Completed(TaskCompleteTypes::Failed(format!(
                                            "task failed by worker {:#?}",
                                            td.1
                                        ))),
                                    )
                                    .await;
                                    break;
                                }
                            } else {
                                log::error!("task failed by worker {:#?} ", td.1);
                                failed = true;
                                complete_task(
                                    context.clone(),
                                    &taskc.id,
                                    TaskStatus::Completed(TaskCompleteTypes::Failed(format!(
                                        "task failed by worker {:#?}",
                                        td.1
                                    ))),
                                )
                                .await;
                                break;
                            }
                        }
                        if !failed {
                            let svg = HandWritingGen::write_svg_fromstroke(
                                strokesall.clone(),
                                1000.,
                                60.0,
                            );

                            match svg {
                                Ok(svg) => {
                                    let mut strokepaths = vec![];
                                    for mut stroke in strokesall {
                                        strokepaths.append(&mut stroke.strokes)
                                    }
                                    complete_task(
                                        context.clone(),
                                        &task.id,
                                        TaskStatus::Completed(TaskCompleteTypes::Success(
                                            SuccessResult {
                                                url: format!(
                                                    "https://handwrite.herokuapp.com/file/{}.svg",
                                                    task.id.clone()
                                                ),
                                                svg: svg,
                                                stroke: Stroke {
                                                    strokes: strokepaths,
                                                },
                                            },
                                        )),
                                    )
                                    .await;
                                }
                                Err(err) => {
                                    complete_task(
                                        context.clone(),
                                        &taskc.id,
                                        TaskStatus::Completed(TaskCompleteTypes::Failed(format!(
                                            "task failed {:#?}",
                                            err
                                        ))),
                                    )
                                    .await;
                                }
                            }
                        }
                    }
                    Err(err) => {
                        complete_task(
                            context.clone(),
                            &taskc.id,
                            TaskStatus::Completed(TaskCompleteTypes::Failed(format!(
                                "task failed {:#?}",
                                err
                            ))),
                        )
                        .await;
                    }
                }
            }
        }
    });
    futures::join!(refresher, manager);
}
