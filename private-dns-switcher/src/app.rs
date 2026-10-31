use crate::routes::Route;
use dioxus::prelude::*;

const MAIN_CSS: Asset = asset!("/assets/styles/main.css");

#[component]
pub fn App() -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        style { {include_str!("../assets/styles/main.css")} }
        Router::<Route> {}
    }
}
