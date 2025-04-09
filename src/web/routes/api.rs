use crate::config::PleblotteryConfig;
use axum::{response::Html, Router};

pub async fn serve_config_htmx() -> Html<String> {
    match PleblotteryConfig::from_file("./config.toml") {
        Ok(config) => {
            let rows = [format!(
                    r#"<tr class="hover:bg-gray-100">
                        <td class="border px-4 py-2 font-bold">Sv2 Mining Port</td>
                        <td class="border px-4 py-2">{}</td>
                        <td class="border px-4 py-2">Port that Sv2 clients should connect to</td>
                    </tr>"#,
                    config.mining_server_config.listening_port
                ),
                format!(
                    r#"<tr class="hover:bg-gray-100">
                        <td class="border px-4 py-2 font-bold">Sv2 Noise Pubkey</td>
                        <td class="border px-4 py-2">{}</td>
                        <td class="border px-4 py-2">Public key used for Sv2 noise encryption with clients</td>
                    </tr>"#,
                    config.mining_server_config.pub_key
                ),
                format!(
                    r#"<tr class="hover:bg-gray-100">
                        <td class="border px-4 py-2 font-bold">Sv2 NoiseCertificate Validity</td>
                        <td class="border px-4 py-2">{}</td>
                        <td class="border px-4 py-2">Time window (in seconds) during which the certificate is valid for authentication under Sv2 noise. <br><br> This helps ensure you're connecting to a legitimate mining server.</td>
                    </tr>"#,
                    config.mining_server_config.cert_validity
                ),
                format!(
                    r#"<tr class="hover:bg-gray-100">
                        <td class="border px-4 py-2 font-bold">Inactivity Limit</td>
                        <td class="border px-4 py-2">{}</td>
                        <td class="border px-4 py-2">Inactivity timeout in seconds (time before a client is disconnected if they don't send any messages)</td>
                    </tr>"#,
                    config.mining_server_config.inactivity_limit
                ),
                // Template Distribution Config
                format!(
                    r#"<tr class="hover:bg-gray-100">
                        <td class="border px-4 py-2 font-bold">Sv2 Template Distribution Server</td>
                        <td class="border px-4 py-2">{}</td>
                        <td class="border px-4 py-2">Address of the template distribution server (URL:port)</td>
                    </tr>"#,
                    config.template_distribution_config.server_addr
                ),
                format!(
                    r#"<tr class="hover:bg-gray-100">
                        <td class="border px-4 py-2 font-bold">Sv2 Template Distribution Server Public Key</td>
                        <td class="border px-4 py-2">{}</td>
                        <td class="border px-4 py-2">Public key used for Sv2 noise encryption with the Sv2 Template Distribution Server</td>
                    </tr>"#,
                    config.template_distribution_config
                        .auth_pk
                        .as_ref()
                        .map(|key| key.to_string())
                        .unwrap_or_else(|| "None".to_string())
                )];

            // Return the rows as the response
            Html(rows.join(""))
        }
        Err(_) => Html(
            r#"<tr class="bg-red-100">
                <td colspan="3" class="border px-4 py-2 text-red-600">Error loading configuration</td>
            </tr>"#
            .to_string(),
        ),
    }
}

// Define the router for API routes
pub fn api_routes() -> Router {
    Router::new().route("/api/config", axum::routing::get(serve_config_htmx))
}
