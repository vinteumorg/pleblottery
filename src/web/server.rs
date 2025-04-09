use anyhow::Result;
use axum::Router;
use tokio::net::TcpListener;
use tower_http::services::ServeDir;

use crate::web::routes::{api::api_routes, html::html_routes};

pub async fn start_web_server() -> Result<()> {
    let app = Router::new()
        .nest_service("/static", ServeDir::new("src/web/assets"))
        .merge(html_routes())
        .merge(api_routes());

    let listener = TcpListener::bind("0.0.0.0:8000").await?;

    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("axum serve failed");
    });

    Ok(())
}
