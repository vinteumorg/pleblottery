use crate::config::PlebLotteryMiningServerConfig;
use crate::config::PlebLotteryTemplateDistributionClientConfig;
use crate::state::SharedStateHandle;
use crate::sv2_handlers::mining_server_handler::PlebLotteryMiningServerHandler;
use crate::sv2_handlers::template_distribution_client_handler::PlebLotteryTemplateDistributionClientHandler;
use anyhow::{anyhow, Result};
use sv2_services::client::service::config::Sv2ClientServiceConfig;
use sv2_services::client::service::subprotocols::mining::handler::NullSv2MiningClientHandler;
use sv2_services::client::service::Sv2ClientService;
use sv2_services::server::service::config::Sv2ServerServiceConfig;
use sv2_services::server::service::Sv2ServerService;
use sv2_services::Sv2Service;
use tokio_util::sync::CancellationToken;

use tracing::debug;

#[derive(Clone)]
pub struct PlebLotteryService {
    server_service: Sv2ServerService<PlebLotteryMiningServerHandler>,
    client_service:
        Sv2ClientService<NullSv2MiningClientHandler, PlebLotteryTemplateDistributionClientHandler>,
    cancellation_token: CancellationToken,
}

impl PlebLotteryService {
    pub async fn new(
        mining_server_config: PlebLotteryMiningServerConfig,
        template_distribution_client_config: PlebLotteryTemplateDistributionClientConfig,
        shared_state: SharedStateHandle,
    ) -> Result<Self> {
        let server_config: Sv2ServerServiceConfig = mining_server_config.clone().into();
        let client_config: Sv2ClientServiceConfig = template_distribution_client_config.into();

        let cancellation_token = CancellationToken::new();

        let mining_server_handler = PlebLotteryMiningServerHandler::new(
            shared_state,
            mining_server_config.coinbase_output_script,
            mining_server_config.coinbase_tag,
            mining_server_config.share_batch_size,
            mining_server_config.expected_shares_per_minute,
        )
        .await;
        let template_distribution_client_handler =
            PlebLotteryTemplateDistributionClientHandler::new(
                client_config
                    .template_distribution_config
                    .clone()
                    .expect("Template distribution config must be set")
                    .coinbase_output_constraints
                    .0,
                client_config
                    .template_distribution_config
                    .clone()
                    .expect("Template distribution config must be set")
                    .coinbase_output_constraints
                    .1,
            );

        let (server_service, sibling_server_io) = Sv2ServerService::new_with_sibling_io(
            server_config.clone(),
            mining_server_handler,
            cancellation_token.clone(),
        )
        .map_err(|_| anyhow::anyhow!("Failed to create server service"))?;
        let client_service = Sv2ClientService::new_from_sibling_io(
            client_config.clone(),
            NullSv2MiningClientHandler,
            template_distribution_client_handler,
            sibling_server_io,
            cancellation_token.clone(),
        )
        .map_err(|_| anyhow::anyhow!("Failed to create client service"))?;

        Ok(Self {
            server_service,
            client_service,
            cancellation_token,
        })
    }

    pub async fn start(&mut self) -> Result<()> {
        tokio::select! {
            result = self.server_service.start() => {
                if let Err(e) = result {
                    self.cancellation_token.cancel();
                    return Err(anyhow!("Failed to start server service: {:?}", e));
                }
            }
            result = self.client_service.start() => {
                if let Err(e) = result {
                    self.cancellation_token.cancel();
                    return Err(anyhow!("Failed to start client service: {:?}", e));
                }
            }
        }

        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        debug!("Shutting down PlebLotteryService");
        self.cancellation_token.cancel();
        Ok(())
    }
}
