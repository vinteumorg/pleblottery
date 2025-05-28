use tower_stratum::roles_logic_sv2::mining_sv2::{
    CloseChannel, OpenExtendedMiningChannel, OpenStandardMiningChannel, SetCustomMiningJob,
    SubmitSharesExtended, SubmitSharesStandard, UpdateChannel,
};
use tower_stratum::roles_logic_sv2::template_distribution_sv2::{NewTemplate, SetNewPrevHash};
use tower_stratum::server::service::request::RequestToSv2ServerError;
use tower_stratum::server::service::response::ResponseFromSv2Server;
use tower_stratum::server::service::subprotocols::mining::handler::Sv2MiningServerHandler;

use crate::state::SharedStateHandle;

use std::task::{Context, Poll};
use tracing::info;

#[derive(Debug, Clone, Default)]
pub struct PlebLotteryMiningServerHandler {
    pub shared_state: SharedStateHandle,
}

impl PlebLotteryMiningServerHandler {
    pub fn new(shared_state: SharedStateHandle) -> Self {
        Self { shared_state }
    }
}

impl Sv2MiningServerHandler for PlebLotteryMiningServerHandler {
    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), RequestToSv2ServerError>> {
        Poll::Ready(Ok(()))
    }

    fn setup_connection_success_flags(&self) -> u32 {
        // no requirement for fixed version field
        // no requirement for extended channel only
        0
    }

    async fn add_client(&mut self, _client_id: u32, _flags: u32) {
        // todo
    }

    async fn remove_client(&mut self, _client_id: u32) {
        // todo
    }

    async fn remove_all_clients(&mut self) {
        // todo
    }

    async fn handle_open_standard_mining_channel(
        &self,
        _client_id: u32,
        _m: OpenStandardMiningChannel<'static>,
    ) -> Result<ResponseFromSv2Server<'static>, RequestToSv2ServerError> {
        info!("Received OpenStandardMiningChannel message");
        Ok(ResponseFromSv2Server::Ok)
    }

    async fn handle_open_extended_mining_channel(
        &self,
        _client_id: u32,
        _m: OpenExtendedMiningChannel<'static>,
    ) -> Result<ResponseFromSv2Server<'static>, RequestToSv2ServerError> {
        info!("Received OpenExtendedMiningChannel message");
        Ok(ResponseFromSv2Server::Ok)
    }

    async fn handle_update_channel(
        &self,
        _client_id: u32,
        _m: UpdateChannel<'static>,
    ) -> Result<ResponseFromSv2Server<'static>, RequestToSv2ServerError> {
        info!("Received UpdateChannel message");
        Ok(ResponseFromSv2Server::Ok)
    }

    async fn handle_close_channel(
        &self,
        _client_id: u32,
        _m: CloseChannel<'static>,
    ) -> Result<ResponseFromSv2Server<'static>, RequestToSv2ServerError> {
        info!("Received CloseChannel message");
        Ok(ResponseFromSv2Server::Ok)
    }

    async fn handle_submit_shares_standard(
        &self,
        _client_id: u32,
        _m: SubmitSharesStandard,
    ) -> Result<ResponseFromSv2Server<'static>, RequestToSv2ServerError> {
        info!("Received SubmitSharesStandard message");
        Ok(ResponseFromSv2Server::Ok)
    }

    async fn handle_submit_shares_extended(
        &self,
        _client_id: u32,
        _m: SubmitSharesExtended<'static>,
    ) -> Result<ResponseFromSv2Server<'static>, RequestToSv2ServerError> {
        info!("Received SubmitSharesExtended message");
        Ok(ResponseFromSv2Server::Ok)
    }

    async fn handle_set_custom_mining_job(
        &self,
        _client_id: u32,
        _m: SetCustomMiningJob<'static>,
    ) -> Result<ResponseFromSv2Server<'static>, RequestToSv2ServerError> {
        info!("Received SetCustomMiningJob message");
        Ok(ResponseFromSv2Server::Ok)
    }

    async fn on_new_template(
        &self,
        template: NewTemplate<'static>,
    ) -> Result<ResponseFromSv2Server<'static>, RequestToSv2ServerError> {
        info!(
            "Received NewTemplate message with template id {:?}",
            template.template_id
        );

        {
            let mut state = self.shared_state.write().await;
            state.latest_template = Some(template);
        }

        Ok(ResponseFromSv2Server::Ok)
    }

    async fn on_set_new_prev_hash(
        &self,
        prev_hash: SetNewPrevHash<'static>,
    ) -> Result<ResponseFromSv2Server<'static>, RequestToSv2ServerError> {
        info!(
            "Received SetNewPrevHash message with prev hash {}",
            prev_hash
                .prev_hash
                .to_vec()
                .iter()
                .rev()
                .map(|byte| format!("{:02x}", byte))
                .collect::<String>()
        );

        {
            let mut state = self.shared_state.write().await;
            state.latest_prev_hash = Some(prev_hash);
        }

        Ok(ResponseFromSv2Server::Ok)
    }
}
