use anyhow::Result;
use sv2_services::client::service::event::{Sv2ClientEvent, Sv2ClientEventError};
use sv2_services::client::service::outcome::Sv2ClientOutcome;
use sv2_services::client::service::subprotocols::template_distribution::handler::Sv2TemplateDistributionClientHandler;
use sv2_services::client::service::subprotocols::template_distribution::trigger::TemplateDistributionClientTrigger;
use sv2_services::roles_logic_sv2::template_distribution_sv2::{
    NewTemplate, RequestTransactionDataError, RequestTransactionDataSuccess, SetNewPrevHash,
};
use sv2_services::server::service::event::Sv2ServerEvent;
use sv2_services::server::service::subprotocols::mining::trigger::MiningServerTrigger;
use tracing::{error, info};

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::utils::bip34_block_height;

#[derive(Debug, Clone)]
pub struct PlebLotteryTemplateDistributionClientHandler {
    current_height: Arc<RwLock<u64>>,
    coinbase_output_max_additional_size: u32,
    coinbase_output_max_additional_sigops: u16,
}

impl PlebLotteryTemplateDistributionClientHandler {
    pub fn new(
        coinbase_output_max_additional_size: u32,
        coinbase_output_max_additional_sigops: u16,
    ) -> Self {
        Self {
            current_height: Arc::new(RwLock::new(0)),
            coinbase_output_max_additional_size,
            coinbase_output_max_additional_sigops,
        }
    }
}

impl Sv2TemplateDistributionClientHandler for PlebLotteryTemplateDistributionClientHandler {
    async fn start(&mut self) -> Result<Sv2ClientOutcome<'static>, Sv2ClientEventError> {
        Ok(Sv2ClientOutcome::TriggerNewEvent(Box::new(
            Sv2ClientEvent::TemplateDistributionTrigger(
                TemplateDistributionClientTrigger::SetCoinbaseOutputConstraints(
                    self.coinbase_output_max_additional_size,
                    self.coinbase_output_max_additional_sigops,
                ),
            ),
        )))
    }

    async fn handle_new_template(
        &self,
        template: NewTemplate<'static>,
    ) -> Result<Sv2ClientOutcome<'static>, Sv2ClientEventError> {
        let current_height = match bip34_block_height(&template.coinbase_prefix.to_vec()) {
            Ok(height) => height.checked_sub(1).unwrap_or(0), // Subtract 1 to get the **current** height
            Err(_) => 0,
        };

        {
            let mut height = self.current_height.write().await;
            if current_height != *height {
                *height = current_height;
                info!("Current Block Height: {}", current_height);
            }
        }

        let outcome = Sv2ClientOutcome::TriggerNewEvent(Box::new(
            Sv2ClientEvent::SendEventToSiblingServerService(Box::new(
                Sv2ServerEvent::MiningTrigger(MiningServerTrigger::NewTemplate(template)),
            )),
        ));
        Ok(outcome)
    }

    async fn handle_set_new_prev_hash(
        &self,
        prev_hash: SetNewPrevHash<'static>,
    ) -> Result<Sv2ClientOutcome<'static>, Sv2ClientEventError> {
        let outcome = Sv2ClientOutcome::TriggerNewEvent(Box::new(
            Sv2ClientEvent::SendEventToSiblingServerService(Box::new(
                Sv2ServerEvent::MiningTrigger(MiningServerTrigger::SetNewPrevHash(prev_hash)),
            )),
        ));
        Ok(outcome)
    }

    async fn handle_request_transaction_data_success(
        &self,
        _transaction_data: RequestTransactionDataSuccess<'static>,
    ) -> Result<Sv2ClientOutcome<'static>, Sv2ClientEventError> {
        error!("Received unexpected RequestTransactionDataSuccess");
        Err(Sv2ClientEventError::UnsupportedMessage)
    }

    async fn handle_request_transaction_data_error(
        &self,
        _error: RequestTransactionDataError<'static>,
    ) -> Result<Sv2ClientOutcome<'static>, Sv2ClientEventError> {
        error!("Received unexpected RequestTransactionDataError");
        Err(Sv2ClientEventError::UnsupportedMessage)
    }
}
