use std::{error, process::Stdio};

use tokio::{process::Command, task::spawn_blocking};

use warp::{http::Response, reject::Reject, Filter};

use tokio::fs::File;
use tokio::io::AsyncReadExt;

use handwriter_shared::*;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::complete_task;
use crate::Context;

pub async fn worker(context: Context) {
    {
        let hgen = tokio::task::spawn_blocking(|| {
            let hgen = handwriter::pystruct::HandWritingGen::new(true, true);
            hgen
        })
        .await;
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            let task: Option<Task> = {
                let mut c = context.write().await;
                let task_to_do = c.iter_mut().find(|t| t.status.is_waiting());
                if let Some(task) = task_to_do {
                    task.status = TaskStatus::Working(0, 0);
                    Some(task.clone())
                } else {
                    None
                }
            };
            if let Some(task) = task {
                let taskc = task.clone();
                match hgen {
                    Ok(ref hgen) => match hgen {
                        Ok(hgen) => {
                            let hgenf = hgen.clone();
                            let taskc = task.clone();
                            let svg = tokio::task::spawn_blocking(move || {
                                hgenf.gen_svg_and_stroke(
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
                                                svg:svg.0,
                                                stroke:svg.1,
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
    }
}
