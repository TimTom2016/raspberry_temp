use axum::Router;
use hypertext::prelude::*;

use crate::AppState;

mod api;
mod dashboard;
#[component]
pub fn shell<R: Renderable>(children: &R) -> impl Renderable {
    rsx! {
        <!DOCTYPE html>
        <meta charset="utf-8"/>
        <head>
            <title> "Raspberry Pi Temperature" </title>
            <link rel="stylesheet" href="style.css"/>
            <script src="https://cdn.jsdelivr.net/npm/htmx.org@2.0.8/dist/htmx.min.js"></script>
            <script src="//unpkg.com/alpinejs" defer></script>
            <script src="https://cdn.jsdelivr.net/npm/apexcharts"></script>
        </head>
        <body>
            (children)
        </body>
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(dashboard::router())
        .nest("/api/v1", api::router())
}
