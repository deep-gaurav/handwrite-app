use handwriter_shared::Task;
use yew::{prelude::*, services::{IntervalService, interval::IntervalTask}};
use yewtil::future::LinkFuture;

use crate::components::raw_html::RawHtml;

pub struct WrImage{
    task:Task,
    interval_task:IntervalTask,
    link:ComponentLink<Self>,
    reqwest_waiting:bool,
}

#[derive(Debug,Clone,Properties,PartialEq)]
pub struct Props{
    pub task:Task
}
pub enum Msg {
    Tick,
    UpdateFail,
    Update(Task)
}

impl Component for WrImage {
    type Message = Msg;

    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let interval_task = IntervalService::spawn(std::time::Duration::from_millis(3000), 
            link.callback(|_|Msg::Tick)
        );
        Self{
            link,
            task:props.task,
            interval_task,
            reqwest_waiting:false,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg{
            Msg::Tick => {
                if self.task.status.is_completed(){
                    false
                }else{
                    let id =self.task.id.clone();
                    self.reqwest_waiting=true;
                    self.link.send_future(
                        
                        async move {
                            let resp =reqwest::get(&format!("https://handwrite.herokuapp.com/status/{}",id))
                            .await;
                            match resp {
                                Ok(resp) => {
                                    match resp.json::<Task>().await{
                                        Ok(task) => {
                                            Msg::Update(task)
                                        }
                                        Err(_) => {
                                            Msg::UpdateFail
                                        }
                                    }
                                }
                                Err(_) => {
                                    Msg::UpdateFail
                                }
                            }
                        }
                    );
                    false
                }
            }
            Msg::UpdateFail => {
                self.reqwest_waiting=false;
                false
            }
            Msg::Update(task) => {
                self.reqwest_waiting=false;
                self.task=task;
                true
            }
        }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        if _props.task!=self.task {
            self.task = _props.task;
            true
        }else{
            false
        }
    }

    fn view(&self) -> Html {
        match &self.task.status {
            handwriter_shared::TaskStatus::Waiting(position) => {
                html!{
                    <article class="message is-info">
                        <div class="message-header">
                            <p>{"Loading"}</p>
                        </div>
                        <div class="message-body">
                            {format!("Image Loading. Queue Position : {}",position)}
                        </div>
                    </article>
                }
            }
            handwriter_shared::TaskStatus::Completed(status) => {
                match status {
                    handwriter_shared::TaskCompleteTypes::Success(result) => {
                        html!{

                            <article class="message is-success">
                                <div class="message-header">
                                    <p><a href=result.url.clone()>{"Success"}</a></p>
                                </div>
                                <div class="message-body">
                                    <RawHtml inner_html=result.svg.clone() />
                                </div>
                            </article>
                        }
                    }
                    handwriter_shared::TaskCompleteTypes::Failed(err) => {
                        html!{

                            <article class="message is-danger">
                                <div class="message-header">
                                    <p>{"Failed"}</p>
                                </div>
                                <div class="message-body">
                                    {err}
                                </div>
                            </article>
                        }
                    }
                }
            }
        }
    }
}