use anyhow::Result;
use tower_stratum::client::service::request::{RequestToSv2Client, RequestToSv2ClientError};
use tower_stratum::client::service::response::ResponseFromSv2Client;
use tower_stratum::client::service::subprotocols::template_distribution::handler::Sv2TemplateDistributionClientHandler;
use tower_stratum::roles_logic_sv2::template_distribution_sv2::{
    NewTemplate, RequestTransactionDataError, RequestTransactionDataSuccess, SetNewPrevHash,
};
use tower_stratum::server::service::request::RequestToSv2Server;
use tower_stratum::server::service::subprotocols::mining::request::RequestToSv2MiningServer;
use tracing::info;

use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::RwLock;

use crate::utils::bip34_block_height;

#[derive(Debug, Clone, Default)]
pub struct PlebLotteryTemplateDistributionClientHandler {
    current_height: Arc<RwLock<u64>>,
}

impl Sv2TemplateDistributionClientHandler for PlebLotteryTemplateDistributionClientHandler {
    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), RequestToSv2ClientError>> {
        Poll::Ready(Ok(()))
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

        let response = ResponseFromSv2Client::TriggerNewRequest(
            RequestToSv2Client::SendRequestToSiblingServerService(Box::new(
                RequestToSv2Server::MiningTrigger(RequestToSv2MiningServer::NewTemplate(template)),
            )),
        );
        Ok(response)
    }

    async fn handle_set_new_prev_hash(
        &self,
        prev_hash: SetNewPrevHash<'static>,
    ) -> Result<ResponseFromSv2Client<'static>, RequestToSv2ClientError> {
        let response = ResponseFromSv2Client::TriggerNewRequest(
            RequestToSv2Client::SendRequestToSiblingServerService(Box::new(
                RequestToSv2Server::MiningTrigger(RequestToSv2MiningServer::SetNewPrevHash(
                    prev_hash,
                )),
            )),
        );
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
