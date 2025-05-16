use crate::state::SharedStateHandle;
use crate::sv2_handlers::mining_server_handler::PlebLotteryMiningServerHandler;
use crate::sv2_handlers::template_distribution_client_handler::PlebLotteryTemplateDistributionClientHandler;
use anyhow::{anyhow, Result};
use tower_stratum::client::service::config::Sv2ClientServiceConfig;
use tower_stratum::client::service::request::RequestToSv2Client;
use tower_stratum::client::service::response::ResponseFromSv2Client;
use tower_stratum::client::service::subprotocols::template_distribution::request::RequestToSv2TemplateDistributionClientService;
use tower_stratum::client::service::subprotocols::template_distribution::response::ResponseToTemplateDistributionTrigger;
use tower_stratum::client::service::Sv2ClientService;
use tower_stratum::server::service::config::Sv2ServerServiceConfig;
use tower_stratum::server::service::Sv2ServerService;
use tower_stratum::tower::{Service, ServiceExt};

use tracing::{debug, info};

pub struct PlebLotteryService {
    server_config: Sv2ServerServiceConfig,
    server_service: Sv2ServerService<PlebLotteryMiningServerHandler>,
    client_config: Sv2ClientServiceConfig,
    client_service: Sv2ClientService<PlebLotteryTemplateDistributionClientHandler>,
}

impl PlebLotteryService {
    pub fn new(
        server_config: Sv2ServerServiceConfig,
        client_config: Sv2ClientServiceConfig,
        shared_state: SharedStateHandle,
    ) -> Result<Self> {
        let mining_server_handler = PlebLotteryMiningServerHandler::new(shared_state);
        let template_distribution_client_handler =
            PlebLotteryTemplateDistributionClientHandler::default();

        let (server_service, sibling_server_io) =
            Sv2ServerService::new_with_sibling_io(server_config.clone(), mining_server_handler)
                .map_err(|_| anyhow::anyhow!("Failed to create server service"))?;
        let client_service = Sv2ClientService::new_with_sibling_io(
            client_config.clone(),
            template_distribution_client_handler,
            sibling_server_io,
        )
        .map_err(|_| anyhow::anyhow!("Failed to create client service"))?;

        Ok(Self {
            server_config,
            server_service,
            client_config,
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

        let set_coinbase_output_constraints_response = self
            .client_service
            .call(RequestToSv2Client::TemplateDistributionTrigger(
                RequestToSv2TemplateDistributionClientService::SetCoinbaseOutputConstraints(
                    self.client_config
                        .clone()
                        .template_distribution_config
                        .expect("Template distribution config must be set")
                        .coinbase_output_constraints
                        .0,
                    self.client_config
                        .clone()
                        .template_distribution_config
                        .expect("Template distribution config must be set")
                        .coinbase_output_constraints
                        .1,
                ),
            ))
            .await
            .map_err(|e| anyhow!("Failed to request coinbase output constraints: {:?}", e))?;

        match set_coinbase_output_constraints_response {
            ResponseFromSv2Client::ResponseToTemplateDistributionTrigger(
                ResponseToTemplateDistributionTrigger::SuccessfullySetCoinbaseOutputConstraints,
            ) => {
                info!("Successfully set Coinbase Output Constraints with Template Distribution Server");
            }
            _ => {
                return Err(anyhow!("Failed to set coinbase output constraints"));
            }
        }

        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        debug!("Shutting down PlebLotteryService");
        self.server_service.shutdown().await;
        self.client_service.shutdown().await;
        Ok(())
    }
}
