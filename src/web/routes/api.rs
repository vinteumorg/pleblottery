use crate::state::SharedStateHandle;
use crate::{config::PleblotteryConfig, utils::bip34_block_height};
use axum::{extract::State, response::Html, Router};

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

pub async fn get_latest_template(State(shared_state): State<SharedStateHandle>) -> Html<String> {
    let state = shared_state.read().await;
    if let Some(template) = &state.latest_template {
        let rows = format!(
            r#"
            <tr>
                <td>Template ID</td>
                <td>{}</td>
            </tr>
            <tr>
                <td>Version</td>
                <td>{}</td>
            </tr>
            <tr>
                <td>Coinbase Value</td>
                <td>{}</td>
            </tr>"#,
            template.template_id,
            template
                .version
                .to_be_bytes()
                .iter()
                .map(|byte| format!("{:02x}", byte))
                .collect::<String>(),
            template.coinbase_tx_value_remaining as f64 / 100_000_000.0
        );
        Html(rows)
    } else {
        Html(
            r#"<tr>
                <td colspan="4">No template available</td>
            </tr>"#
                .to_string(),
        )
    }
}

pub async fn get_latest_prev_hash(State(shared_state): State<SharedStateHandle>) -> Html<String> {
    let state = shared_state.read().await;
    let mut rows = String::new();

    if let Some(template) = &state.latest_template {
        let current_height = match bip34_block_height(&template.coinbase_prefix.to_vec()) {
            Ok(height) => height.checked_sub(1).unwrap_or(0), // Subtract 1 to get the **current** height
            Err(_) => 0,
        };
        rows.push_str(&format!(
            r#"
            <tr>
                <td>Height</td>
                <td>{}</td>
            </tr>
            "#,
            current_height
        ));
    } else {
        rows.push_str(
            r#"<tr class="bg-red-100">
                <td colspan="4">No current height available</td>
            </tr>"#,
        );
    }

    if let Some(prev_hash) = &state.latest_prev_hash {
        rows.push_str(&format!(
            r#"
            <tr>
                <td>Prev Hash</td>
                <td>{}</td>
            </tr>
            <tr>
                <td>nBits</td>
                <td>{}</td>
            </tr>
            <tr>
                <td>Target</td>
                <td>{}</td>
            </tr>
            "#,
            prev_hash
                .prev_hash
                .to_vec()
                .iter()
                .rev()
                .map(|byte| format!("{:02x}", byte))
                .collect::<String>(),
            format!("{:02x}", prev_hash.n_bits),
            prev_hash
                .target
                .to_vec()
                .iter()
                .rev()
                .map(|byte| format!("{:02x}", byte))
                .collect::<String>()
        ));
    } else {
        rows.push_str(
            r#"<tr class="bg-red-100">
                <td colspan="4">No prev hash available</td>
            </tr>"#,
        );
    }

    Html(rows)
}

pub async fn get_mining_stats(State(shared_state): State<SharedStateHandle>) -> Html<String> {
    let state = shared_state.read().await;
    let mut rows = String::new();

    if let Some(_) = &state.latest_prev_hash {
        rows.push_str(&format!(
            r#"
                <tr>
                    <td>Total Clients</td>
                    <td>{}</td>
                </tr>
                <tr>
                    <td>Total shares</td>
                    <td>{}</td>
                </tr>
                <tr>
                    <td>Best Share</td>
                    <td>{}</td>
                </tr>
                <tr>
                    <td>Total Hashrate</td>
                    <td>{}</td>
                </tr>
                <tr>
                    <td>💰 Blocks Found 💰</td>
                    <td>{}</td>
                </tr>
            "#,
            state.total_clients,
            state.total_shares_submitted,
            state.format_best_share(),
            state.format_hashrate(),
            state.blocks_found
        ));
    } else {
        rows.push_str(
            r#"<tr>
                <td colspan="4">No mining stats available</td>
            </tr>"#,
        );
    }

    Html(rows)
}

pub async fn get_clients_stats(State(shared_state): State<SharedStateHandle>) -> Html<String> {
    let state = shared_state.read().await;
    let mut rows = String::new();

    if state.clients.read().await.len() > 0 as usize {
        let clients = state.clients.read().await;
        for (_, client) in clients.iter() {
            let client = client.read().await;
            rows.push_str(&format!(
                r#"
                <div>
                    <table class="tg">
                        <thead>
                            <tr>
                                <th colspan="2">Client {}</th>
                            </tr>
                        </thead>
                        <tbody>
                            <tr>
                                <td>Client ID</td>
                                <td>{}</td>
                            </tr>
                            <tr>
                                <td>Connection Flags</td>
                                <td>{:04b}</td>
                            </tr>
                            <tr>
                                <td>Group Channel</td>
                                <td>{}</td>
                            </tr>
                            <tr>
                                <td>Standard Channels</td>
                                <td>{}</td>
                            </tr>
                            <tr>
                                <td>Extended Channels</td>
                                <td>{}</td>
                            </tr>
                        </tbody>
                    </table>
                </div>
                "#,
                client.client_id,
                client.client_id,
                client.connection_flags,
                client
                    .group_channel
                    .is_some()
                    .then(|| "Yes")
                    .unwrap_or("No"),
                client.standard_channels.read().await.len(),
                client.extended_channels.read().await.len()
            ));
        }
    }

    Html(rows)
}

pub fn api_routes(shared_state: SharedStateHandle) -> Router {
    Router::new()
        .route("/api/config", axum::routing::get(serve_config_htmx))
        .route(
            "/api/latest-template",
            axum::routing::get(get_latest_template),
        )
        .route(
            "/api/latest-prev-hash",
            axum::routing::get(get_latest_prev_hash),
        )
        .route("/api/mining-stats", axum::routing::get(get_mining_stats))
        .route("/api/clients", axum::routing::get(get_clients_stats))
        .with_state(shared_state)
}
