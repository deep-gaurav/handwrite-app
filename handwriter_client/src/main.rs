#![recursion_limit="1024"]
pub mod app;
pub mod components;

use std::panic;

use app::App;

fn main() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    wasm_logger::init(wasm_logger::Config::default());
    yew::start_app::<App>();
}
