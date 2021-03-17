use yew::prelude::*;

use crate::components::home::Home;

pub struct App{

}

pub enum  Msg{

}

#[derive(Clone,Properties,Default)]
pub struct Props{

}

impl Component for App{
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Self{}
    }

    fn update(&mut self, msg: Self::Message) -> bool {
        false
    }

    fn change(&mut self, _props: Self::Properties) -> bool {
        false
    }

    fn view(&self) -> Html {
        html!{
            <div>
                <Home />
            </div>
        }
    }
}