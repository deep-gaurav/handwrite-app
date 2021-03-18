use handwriter_shared::{HandParameters, Task};
use yew::prelude::*;
use yewtil::future::LinkFuture;
use web_sys::{HtmlInputElement,HtmlTextAreaElement};

use crate::components::wrimage::WrImage;

pub struct Home{
    style_ref:NodeRef,
    bias_ref:NodeRef,
    text_ref:NodeRef,
    task:Option<Task>,
    is_loading:bool,
    link:ComponentLink<Self>,
}

pub enum Msg{
    Generate,
    GeneratedTask(Task),
    GenerateFail,
}

#[derive(Debug,Clone,Default,Properties)]
pub struct Props{

}

impl Component for Home {
    type Message = Msg;

    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Home{
            style_ref:NodeRef::default(),
            bias_ref:NodeRef::default(),
            text_ref:NodeRef::default(),
            task:None,
            is_loading:false,
            link
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::Generate => {
                let style = self.style_ref.cast::<HtmlInputElement>().map(|e|e.value()).unwrap_or_default().parse::<u32>().ok();
                let bias = self.bias_ref.cast::<HtmlInputElement>().map(|e|e.value()).unwrap_or_default().parse::<f32>().ok();
                let mut text = self.text_ref.cast::<HtmlTextAreaElement>().map(|e|e.value()).unwrap_or("The quick brown fox jumps over a lazy dog.".to_string());
                if text.is_empty() {
                    text = "The quick brown fox jumps over a lazy dog.".to_string();
                }
                self.link.send_future(
                    async move {
                        let client = reqwest::Client::new();
                        let resp =client.post("https://handwrite.herokuapp.com/create")
                            .json(
                                &HandParameters{
                                    text: text,
                                    style: style,
                                    bias: bias,
                                    color: None,
                                    width: None,
                                }
                            ).send()
                            .await;
                        match resp{
                            Ok(response) => {
                                let body = response.json::<Task>().await;

                                match body {
                                    Ok(task) => {
                                        Msg::GeneratedTask(task)
                                    }
                                    Err(err) => {
                                        Msg::GenerateFail
                                    }
                                }
                            }
                            Err(_) => {
                                Msg::GenerateFail
                            }
                        }
                        
                    }
                );
                self.is_loading = true;
                true
            }
            Msg::GeneratedTask(task)=>{
                self.task=Some(task);
                self.is_loading=false;
                true
            }
            Msg::GenerateFail => {
                self.is_loading=false;
                true
            }
        }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        html!{
            <div class="section">
                <fieldset disabled=self.is_loading>
                <div class="field">
                    <div class="control">
                        <textarea ref=self.text_ref.clone() class="textarea is-small" placeholder="e.g. The quick brown fox jumps over a lazy dog."></textarea>
                    </div>
                </div>
                <div class="field is-horizontal">
                    <div class="field-body">
                        <div class="field">
                        <p class="control is-expanded ">
                            <input ref=self.style_ref.clone() class="input" type="number" placeholder="Style (0,1,2.. 9)"/>
                        </p>
                        </div>
                        <div class="field">
                        <p class="control is-expanded ">
                            <input ref=self.bias_ref.clone() class="input" type="number" min=0 max=1 step=0.01 placeholder="Bias (0 - 1)" />
                        </p>
                        </div>
                    </div>
                </div>
                <div class="control" onclick=self.link.callback(|_|Msg::Generate)>
                    <a class=format!("button is-info {}",{
                        if self.is_loading{
                            "is-loading"
                        }else{
                            ""
                        }
                    })>
                    {"Generate"}
                    </a>
                </div>
                </fieldset>

                {
                    if let Some(task) = &self.task{
                        html!{
                            <div key=task.id.clone() class="box mt-2">
                                <WrImage key=task.id.clone() task=task.clone() />
                            </div>
                        }
                    }else{
                        html!{

                        }
                    }
                }
            </div>
        }
    }
}