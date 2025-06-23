use anyhow::Result;
use tower_stratum::client::service::request::{RequestToSv2Client, RequestToSv2ClientError};
use tower_stratum::client::service::response::ResponseFromSv2Client;
use tower_stratum::client::service::subprotocols::template_distribution::handler::Sv2TemplateDistributionClientHandler;
use tower_stratum::client::service::subprotocols::template_distribution::trigger::TemplateDistributionClientTrigger;
use tower_stratum::roles_logic_sv2::template_distribution_sv2::{
    NewTemplate, RequestTransactionDataError, RequestTransactionDataSuccess, SetNewPrevHash,
};
use tower_stratum::server::service::request::RequestToSv2Server;
use tower_stratum::server::service::subprotocols::mining::trigger::MiningServerTrigger;
use tracing::info;

use std::sync::Arc;
use std::task::{Context, Poll};
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
    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), RequestToSv2ClientError>> {
        Poll::Ready(Ok(()))
    }

    async fn start(&mut self) -> Result<ResponseFromSv2Client<'static>, RequestToSv2ClientError> {
        Ok(ResponseFromSv2Client::TriggerNewRequest(Box::new(
            RequestToSv2Client::TemplateDistributionTrigger(
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
    ) -> Result<ResponseFromSv2Client<'static>, RequestToSv2ClientError> {
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

        let response = ResponseFromSv2Client::TriggerNewRequest(Box::new(
            RequestToSv2Client::SendRequestToSiblingServerService(Box::new(
                RequestToSv2Server::MiningTrigger(MiningServerTrigger::NewTemplate(template)),
            )),
        ));
        Ok(response)
    }

    async fn handle_set_new_prev_hash(
        &self,
        prev_hash: SetNewPrevHash<'static>,
    ) -> Result<ResponseFromSv2Client<'static>, RequestToSv2ClientError> {
        let response = ResponseFromSv2Client::TriggerNewRequest(Box::new(
            RequestToSv2Client::SendRequestToSiblingServerService(Box::new(
                RequestToSv2Server::MiningTrigger(MiningServerTrigger::SetNewPrevHash(prev_hash)),
            )),
        ));
        Ok(response)
    }

    async fn handle_request_transaction_data_success(
        &self,
        _transaction_data: RequestTransactionDataSuccess<'static>,
    ) -> Result<ResponseFromSv2Client<'static>, RequestToSv2ClientError> {
        unimplemented!("handle_request_transaction_data_success should not be called");
    }

    async fn handle_request_transaction_data_error(
        &self,
        _error: RequestTransactionDataError<'static>,
    ) -> Result<ResponseFromSv2Client<'static>, RequestToSv2ClientError> {
        unimplemented!("handle_request_transaction_data_error should not be called");
    }
}
