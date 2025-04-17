use anyhow::Result;
use tower_stratum::client::service::request::RequestToSv2ClientError;
use tower_stratum::client::service::response::ResponseFromSv2Client;
use tower_stratum::client::service::subprotocols::template_distribution::handler::Sv2TemplateDistributionClientHandler;
use tower_stratum::roles_logic_sv2::template_distribution_sv2::{
    NewTemplate, RequestTransactionDataError, RequestTransactionDataSuccess, SetNewPrevHash,
};

use tracing::info;

#[derive(Debug, Clone, Default)]
pub struct PlebLotteryTemplateDistributionClientHandler {}

impl Sv2TemplateDistributionClientHandler for PlebLotteryTemplateDistributionClientHandler {
    async fn handle_new_template(
        &self,
        _template: NewTemplate<'static>,
    ) -> Result<ResponseFromSv2Client<'static>, RequestToSv2ClientError> {
        info!("Received NewTemplate message");
        Ok(ResponseFromSv2Client::ToDo)
    }

    async fn handle_set_new_prev_hash(
        &self,
        _prev_hash: SetNewPrevHash<'static>,
    ) -> Result<ResponseFromSv2Client<'static>, RequestToSv2ClientError> {
        info!("Received SetNewPrevHash message");
        Ok(ResponseFromSv2Client::ToDo)
    }

    async fn handle_request_transaction_data_success(
        &self,
        _transaction_data: RequestTransactionDataSuccess<'static>,
    ) -> Result<ResponseFromSv2Client<'static>, RequestToSv2ClientError> {
        info!("Received RequestTransactionDataSuccess message");
        Ok(ResponseFromSv2Client::ToDo)
    }

    async fn handle_request_transaction_data_error(
        &self,
        _error: RequestTransactionDataError<'static>,
    ) -> Result<ResponseFromSv2Client<'static>, RequestToSv2ClientError> {
        info!("Received RequestTransactionDataError message");
        Ok(ResponseFromSv2Client::ToDo)
    }
}
