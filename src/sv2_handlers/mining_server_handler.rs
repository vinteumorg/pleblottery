use tokio::sync::RwLock;
use tower_stratum::roles_logic_sv2::channels::server::group::GroupChannel;
use tower_stratum::roles_logic_sv2::mining_sv2::{
    CloseChannel, OpenExtendedMiningChannel, OpenStandardMiningChannel, SetCustomMiningJob,
    SubmitSharesExtended, SubmitSharesStandard, UpdateChannel,
    MESSAGE_TYPE_NEW_EXTENDED_MINING_JOB,
};
use tower_stratum::roles_logic_sv2::mining_sv2::{
    SetNewPrevHash as SetNewPrevHashMp, MESSAGE_TYPE_MINING_SET_NEW_PREV_HASH,
};
use tower_stratum::roles_logic_sv2::parsers::{AnyMessage, Mining};
use tower_stratum::roles_logic_sv2::template_distribution_sv2::{NewTemplate, SetNewPrevHash};
use tower_stratum::roles_logic_sv2::utils::Id as IdFactory;
use tower_stratum::server::service::client::Sv2MessagesToClient;
use tower_stratum::server::service::request::RequestToSv2Server;
use tower_stratum::server::service::request::RequestToSv2ServerError;
use tower_stratum::server::service::response::ResponseFromSv2Server;
use tower_stratum::server::service::subprotocols::mining::handler::Sv2MiningServerHandler;

use crate::state::SharedStateHandle;

use bitcoin::{transaction::TxOut, Amount};
use std::collections::HashMap;
use std::sync::Arc;
use std::task::{Context, Poll};
use tracing::{error, info};

#[derive(Debug)]
pub struct PleblotteryMiningClient {
    pub client_id: u32,
    pub connection_flags: u32,
    pub channel_id_factory: IdFactory,
    pub group_channel: Option<Arc<RwLock<GroupChannel<'static>>>>, // only one group per client, all standard channels belong to it
}

#[derive(Debug, Clone, Default)]
pub struct PlebLotteryMiningServerHandler {
    pub clients: Arc<RwLock<HashMap<u32, Arc<RwLock<PleblotteryMiningClient>>>>>,
    pub shared_state: SharedStateHandle,
    pub coinbase_output_script: bitcoin::ScriptBuf,
    pub future_templates: Arc<RwLock<HashMap<u64, NewTemplate<'static>>>>,
    pub last_activated_future_template: Arc<RwLock<Option<NewTemplate<'static>>>>,
    pub last_prev_hash: Arc<RwLock<Option<SetNewPrevHash<'static>>>>,
}

impl PlebLotteryMiningServerHandler {
    pub fn new(
        shared_state: SharedStateHandle,
        coinbase_output_script: bitcoin::ScriptBuf,
    ) -> Self {
        Self {
            shared_state,
            clients: Arc::new(RwLock::new(HashMap::new())),
            coinbase_output_script,
            future_templates: Arc::new(RwLock::new(HashMap::new())),
            last_activated_future_template: Arc::new(RwLock::new(None)),
            last_prev_hash: Arc::new(RwLock::new(None)),
        }
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

    async fn add_client(&mut self, client_id: u32, flags: u32) {
        info!("Adding client with id: {}, flags: {}", client_id, flags);

        let mut channel_id_factory = IdFactory::new();

        // if SetupConnection.REQUIRES_STANDARD_JOBS is set
        // client does not understand group channels
        let group_channel = if flags & 0x0001 == 0x0001 {
            None
        } else {
            let group_channel_id = channel_id_factory.next();
            info!("Adding group channel with id: {}", group_channel_id);
            Some(Arc::new(RwLock::new(GroupChannel::new(group_channel_id))))
        };

        let client = PleblotteryMiningClient {
            client_id,
            connection_flags: flags,
            channel_id_factory,
            group_channel,
        };

        self.clients
            .write()
            .await
            .insert(client_id, Arc::new(RwLock::new(client)));
    }

    async fn remove_client(&mut self, client_id: u32) {
        info!("Removing client with id: {}", client_id);

        self.clients.write().await.remove(&client_id);
    }

    async fn remove_all_clients(&mut self) {
        info!("Removing all clients");

        self.clients.write().await.clear();
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
            state.latest_template = Some(template.clone());
        }

        let mut messages_to_clients: Vec<Sv2MessagesToClient> = Vec::new();

        let coinbase_output_script = self.coinbase_output_script.clone();
        let coinbase_output_value = template.coinbase_tx_value_remaining;
        let coinbase_tx_output = TxOut {
            value: Amount::from_sat(coinbase_output_value),
            script_pubkey: coinbase_output_script,
        };

        if template.future_template {
            self.future_templates
                .write()
                .await
                .insert(template.template_id, template.clone());
        }

        for (client_id, client_guard) in self.clients.read().await.iter() {
            let client = client_guard.write().await;

            let mut messages_to_client = Vec::new();

            match template.future_template {
                true => {
                    // if there's a group channel, process the template on it
                    if let Some(group_channel_guard) = &client.group_channel {
                        let mut group_channel = group_channel_guard.write().await;
                        group_channel
                            .on_new_template(template.clone(), vec![coinbase_tx_output.clone()])
                            .map_err(|e| {
                                error!("Error sending new template to group channel: {:?}", e);
                                RequestToSv2ServerError::MiningHandlerError(format!(
                                    "Error sending new template to group channel: {:?}",
                                    e
                                ))
                            })?;

                        let future_job_id = group_channel.get_future_template_to_job_id().get(&template.template_id).ok_or_else(|| {
                            error!("Error getting future job id");
                            RequestToSv2ServerError::MiningHandlerError(format!("Error getting future job id for template {:?} for group channel {:?}", template.template_id, group_channel.get_group_channel_id()))
                        })?;

                        let future_job = group_channel.get_future_jobs().get(future_job_id).ok_or_else(|| {
                            error!("Error getting future job");
                            RequestToSv2ServerError::MiningHandlerError(format!("Error getting future job for template {:?} for group channel {:?}", template.template_id, group_channel.get_group_channel_id()))
                        })?;

                        let group_extended_job = (
                            AnyMessage::Mining(Mining::NewExtendedMiningJob(
                                future_job.get_job_message().clone(),
                            )),
                            MESSAGE_TYPE_NEW_EXTENDED_MINING_JOB,
                        );

                        info!("Sending future NewExtendedMiningJob message to channel {} of client {:?} for job id {:?}", group_channel.get_group_channel_id(), client_id, future_job_id);
                        messages_to_client.push(group_extended_job);
                    }

                    // todo: process standard channels

                    // todo: process extended channels

                    let message_to_client = Sv2MessagesToClient {
                        client_id: *client_id,
                        messages: messages_to_client,
                    };

                    messages_to_clients.push(message_to_client);
                }
                false => {
                    // if there's a group channel, process the template on it
                    if let Some(group_channel_guard) = &client.group_channel {
                        let mut group_channel = group_channel_guard.write().await;
                        group_channel
                            .on_new_template(template.clone(), vec![coinbase_tx_output.clone()])
                            .map_err(|e| {
                                error!("Error sending new template to group channel: {:?}", e);
                                RequestToSv2ServerError::MiningHandlerError(format!(
                                    "Error sending new template to group channel: {:?}",
                                    e
                                ))
                            })?;

                        let active_job = group_channel.get_active_job().ok_or_else(|| {
                            error!("Error getting active job");
                            RequestToSv2ServerError::MiningHandlerError(format!(
                                "Error getting active job for group channel {:?}",
                                group_channel.get_group_channel_id()
                            ))
                        })?;

                        let group_extended_job = (
                            AnyMessage::Mining(Mining::NewExtendedMiningJob(
                                active_job.get_job_message().clone(),
                            )),
                            MESSAGE_TYPE_NEW_EXTENDED_MINING_JOB,
                        );

                        info!("Sending non-future NewExtendedMiningJob message to channel {} of client {:?} for job id {:?}", group_channel.get_group_channel_id(), client_id, active_job.get_job_id());
                        messages_to_client.push(group_extended_job);
                    }

                    // todo: process standard channels

                    // todo: process extended channels

                    let message_to_client = Sv2MessagesToClient {
                        client_id: *client_id,
                        messages: messages_to_client,
                    };

                    messages_to_clients.push(message_to_client);
                }
            }
        }

        Ok(ResponseFromSv2Server::TriggerNewRequest(Box::new(
            RequestToSv2Server::SendMessagesToClients(Box::new(messages_to_clients)),
        )))
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
            state.latest_prev_hash = Some(prev_hash.clone());
        }

        let mut last_prev_hash_guard = self.last_prev_hash.write().await;
        *last_prev_hash_guard = Some(prev_hash.clone());

        let mut future_templates_guard = self.future_templates.write().await;
        let activated_future_template = future_templates_guard
            .get(&prev_hash.template_id)
            .ok_or_else(|| {
                error!("Error getting future template");
                RequestToSv2ServerError::MiningHandlerError(format!(
                    "Error getting future template for prev hash {:?}",
                    prev_hash.prev_hash
                ))
            })?;

        let mut last_activated_future_template_guard =
            self.last_activated_future_template.write().await;
        *last_activated_future_template_guard = Some(activated_future_template.clone());

        future_templates_guard.clear();

        let mut messages_to_clients: Vec<Sv2MessagesToClient> = Vec::new();

        for (client_id, client_guard) in self.clients.read().await.iter() {
            let client = client_guard.write().await;

            let mut messages_to_client = Vec::new();

            if let Some(group_channel_guard) = &client.group_channel {
                let mut group_channel = group_channel_guard.write().await;

                group_channel
                    .on_set_new_prev_hash(prev_hash.clone())
                    .map_err(|e| {
                        error!("Error processing SetNewPrevHash on group channel: {:?}", e);
                        RequestToSv2ServerError::MiningHandlerError(format!(
                            "Error processing SetNewPrevHash on group channel: {:?}",
                            e
                        ))
                    })?;

                let group_channel_active_job = group_channel.get_active_job().ok_or_else(|| {
                    error!("Error getting active job");
                    RequestToSv2ServerError::MiningHandlerError(format!(
                        "Error getting active job for group channel {:?}",
                        group_channel.get_group_channel_id()
                    ))
                })?;

                let active_job_id = group_channel_active_job.get_job_id();

                let set_new_prev_hash_mp = SetNewPrevHashMp {
                    channel_id: group_channel.get_group_channel_id(),
                    prev_hash: prev_hash.prev_hash.clone(),
                    job_id: active_job_id,
                    min_ntime: prev_hash.header_timestamp,
                    nbits: prev_hash.n_bits,
                };

                let set_new_prev_hash_mp = (
                    AnyMessage::Mining(Mining::SetNewPrevHash(set_new_prev_hash_mp)),
                    MESSAGE_TYPE_MINING_SET_NEW_PREV_HASH,
                );

                info!(
                    "Sending SetNewPrevHash message to channel {} of client {:?} for job id {:?}",
                    group_channel.get_group_channel_id(),
                    client_id,
                    active_job_id
                );
                messages_to_client.push(set_new_prev_hash_mp);
            }

            // todo: process standard channels

            // todo: process extended channels

            let messages_to_client = Sv2MessagesToClient {
                client_id: *client_id,
                messages: messages_to_client,
            };

            messages_to_clients.push(messages_to_client);
        }

        Ok(ResponseFromSv2Server::TriggerNewRequest(Box::new(
            RequestToSv2Server::SendMessagesToClients(Box::new(messages_to_clients)),
        )))
    }
}
