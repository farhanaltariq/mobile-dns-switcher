use crate::screens::HomeScreen;
use dioxus::prelude::*;

#[derive(Routable, Clone, PartialEq, Debug)]
pub enum Route {
    #[route("/")]
    HomeScreen {},
}
