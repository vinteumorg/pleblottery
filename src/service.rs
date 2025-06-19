use crate::config::PlebLotteryMiningServerConfig;
use crate::config::PlebLotteryTemplateDistributionClientConfig;
use crate::state::SharedStateHandle;
use crate::sv2_handlers::mining_server_handler::PlebLotteryMiningServerHandler;
use crate::sv2_handlers::template_distribution_client_handler::PlebLotteryTemplateDistributionClientHandler;
use anyhow::{anyhow, Result};
use tower_stratum::client::service::config::Sv2ClientServiceConfig;
use tower_stratum::client::service::subprotocols::mining::handler::NullSv2MiningClientHandler;
use tower_stratum::client::service::Sv2ClientService;
use tower_stratum::server::service::config::Sv2ServerServiceConfig;
use tower_stratum::server::service::Sv2ServerService;
use tower_stratum::tower::ServiceExt;

use tracing::debug;

pub struct PlebLotteryService {
    server_service: Sv2ServerService<PlebLotteryMiningServerHandler>,
    client_service:
        Sv2ClientService<NullSv2MiningClientHandler, PlebLotteryTemplateDistributionClientHandler>,
}

impl PlebLotteryService {
    pub fn new(
        mining_server_config: PlebLotteryMiningServerConfig,
        template_distribution_client_config: PlebLotteryTemplateDistributionClientConfig,
        shared_state: SharedStateHandle,
    ) -> Result<Self> {
        let server_config: Sv2ServerServiceConfig = mining_server_config.clone().into();
        let client_config: Sv2ClientServiceConfig = template_distribution_client_config.into();

        let mining_server_handler = PlebLotteryMiningServerHandler::new(
            shared_state,
            mining_server_config.coinbase_output_script,
            mining_server_config.coinbase_tag,
            mining_server_config.share_batch_size,
            mining_server_config.expected_shares_per_minute,
        );
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

        let (server_service, sibling_server_io) =
            Sv2ServerService::new_with_sibling_io(server_config.clone(), mining_server_handler)
                .map_err(|_| anyhow::anyhow!("Failed to create server service"))?;
        let client_service = Sv2ClientService::new_from_sibling_io(
            client_config.clone(),
            NullSv2MiningClientHandler,
            template_distribution_client_handler,
            sibling_server_io,
        )
        .map_err(|_| anyhow::anyhow!("Failed to create client service"))?;

        Ok(Self {
            server_service,
            client_service,
        })
    }

    pub async fn start(&mut self) -> Result<()> {
        self.server_service.start().await?;

        match self.client_service.start().await {
            Ok(_) => {}
            Err(e) => {
                self.server_service.shutdown().await;
                return Err(anyhow!("Failed to start client service: {:?}", e));
            }
        }

        self.client_service
            .ready()
            .await
            .map_err(|_| anyhow::anyhow!("Failed to await client service ready"))?;

        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        debug!("Shutting down PlebLotteryService");
        self.server_service.shutdown().await;
        self.client_service.shutdown().await;
        Ok(())
    }
}
