use anyhow::Result;
use axum::Router;
use tokio::net::TcpListener;
use tower_http::services::ServeDir;
use tracing::info;

use crate::config::PlebLotteryWebConfig;
use crate::state::SharedStateHandle;
use crate::web::routes::{api::api_routes, html::html_routes};

pub async fn start_web_server(
    web_config: &PlebLotteryWebConfig,
    shared_state: SharedStateHandle,
) -> Result<()> {
    let app = Router::new()
        .nest_service("/static", ServeDir::new("src/web/assets"))
        .merge(html_routes())
        .merge(api_routes(shared_state));

    let addr = format!("0.0.0.0:{}", web_config.listening_port);
    let listener = TcpListener::bind(&addr).await?;

    info!(
        "Web server started on http://localhost:{}",
        web_config.listening_port
    );

    axum::serve(listener, app).await.expect("axum serve failed");

    Ok(())
}
