use sv1_api::client_to_server::{Authorize, Configure, Submit};
use sv1_api::error::Error;
use sv1_api::server_to_client::VersionRollingParams;
use sv1_api::utils::{Extranonce, HexU32Be};
use sv1_api::Message;

#[derive(Clone)]
pub struct Sv1Handler;

impl<'a> sv1_api::IsServer<'a> for Sv1Handler {
    fn handle_configure(
        &mut self,
        _request: &Configure,
    ) -> (Option<VersionRollingParams>, Option<bool>) {
        todo!()
    }

    // dummy
    fn handle_subscribe(
        &self,
        _request: &sv1_api::client_to_server::Subscribe,
    ) -> Vec<(String, String)> {
        tracing::info!("handling mining.subscribe");

        let set_difficulty_sub = (
            "mining.set_difficulty".to_string(),
            "ae6812eb4cd7735a302a8a9dd95cf71f".to_string(),
        );
        let notify_sub = (
            "mining.notify".to_string(),
            "ae6812eb4cd7735a302a8a9dd95cf71f".to_string(),
        );

        vec![set_difficulty_sub, notify_sub]
    }

    // dummy
    fn handle_authorize(&self, _request: &Authorize) -> bool {
        true
    }

    fn handle_submit(&self, _request: &Submit<'a>) -> bool {
        todo!()
    }

    fn handle_extranonce_subscribe(&self) {
        todo!()
    }

    fn is_authorized(&self, _name: &str) -> bool {
        todo!()
    }

    fn authorize(&mut self, _name: &str) {
        // todo!()
    }

    // dummy
    fn set_extranonce1(&mut self, _extranonce1: Option<Extranonce<'a>>) -> Extranonce<'a> {
        let extranonce: Extranonce<'a> =
            Extranonce::try_from("000000000000000000000000000000010000000000000001")
                .expect("should always work");
        extranonce
    }

    fn extranonce1(&self) -> Extranonce<'a> {
        todo!()
    }

    // dummy
    fn set_extranonce2_size(&mut self, _extra_nonce2_size: Option<usize>) -> usize {
        8
    }

    fn extranonce2_size(&self) -> usize {
        todo!()
    }

    fn version_rolling_mask(&self) -> Option<HexU32Be> {
        todo!()
    }

    fn set_version_rolling_mask(&mut self, _mask: Option<HexU32Be>) {
        todo!()
    }

    fn set_version_rolling_min_bit(&mut self, _mask: Option<HexU32Be>) {
        todo!()
    }

    fn notify(&mut self) -> Result<Message, Error> {
        todo!()
    }
}
