use tokio::sync::{Mutex, RwLock};
use tower_stratum::roles_logic_sv2::channels::server::error::StandardChannelError;
use tower_stratum::roles_logic_sv2::channels::server::extended::ExtendedChannel;
use tower_stratum::roles_logic_sv2::channels::server::group::GroupChannel;
use tower_stratum::roles_logic_sv2::channels::server::standard::StandardChannel;
use tower_stratum::roles_logic_sv2::mining_sv2::{
    CloseChannel, OpenExtendedMiningChannel, OpenMiningChannelError, OpenStandardMiningChannel,
    OpenStandardMiningChannelSuccess, SetCustomMiningJob, SubmitSharesExtended,
    SubmitSharesStandard, UpdateChannel, MAX_EXTRANONCE_LEN,
};
use tower_stratum::roles_logic_sv2::mining_sv2::{
    ExtendedExtranonce, SetNewPrevHash as SetNewPrevHashMp,
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
    pub channel_id_factory: Arc<Mutex<IdFactory>>,
    pub group_channel: Option<Arc<RwLock<GroupChannel<'static>>>>, // only one group per client, all standard channels belong to it
    pub standard_channels: Arc<RwLock<HashMap<u32, Arc<RwLock<StandardChannel<'static>>>>>>,
    pub extended_channels: Arc<RwLock<HashMap<u32, Arc<RwLock<ExtendedChannel<'static>>>>>>,
}

#[derive(Debug, Clone)]
pub struct PlebLotteryMiningServerHandler {
    pub clients: Arc<RwLock<HashMap<u32, Arc<RwLock<PleblotteryMiningClient>>>>>,
    pub shared_state: SharedStateHandle,
    pub coinbase_output_script: bitcoin::ScriptBuf,
    pub future_templates: Arc<RwLock<HashMap<u64, NewTemplate<'static>>>>,
    pub last_activated_future_template: Arc<RwLock<Option<NewTemplate<'static>>>>,
    pub last_prev_hash: Arc<RwLock<Option<SetNewPrevHash<'static>>>>,
    pub extranonce_prefix_factory_standard: Arc<RwLock<ExtendedExtranonce>>,
    pub share_batch_size: usize,
    pub expected_shares_per_minute: f32,
}

impl PlebLotteryMiningServerHandler {
    pub fn new(
        shared_state: SharedStateHandle,
        coinbase_output_script: bitcoin::ScriptBuf,
        coinbase_tag: String,
        share_batch_size: usize,
        expected_shares_per_minute: f32,
    ) -> Self {
        let range_0 = std::ops::Range { start: 0, end: 0 };

        let range_1 = std::ops::Range {
            start: 0,
            end: coinbase_tag.len() + 8,
        };
        let range_2 = std::ops::Range {
            start: coinbase_tag.len() + 8,
            end: MAX_EXTRANONCE_LEN,
        };
        Self {
            shared_state,
            clients: Arc::new(RwLock::new(HashMap::new())),
            coinbase_output_script,
            future_templates: Arc::new(RwLock::new(HashMap::new())),
            last_activated_future_template: Arc::new(RwLock::new(None)),
            last_prev_hash: Arc::new(RwLock::new(None)),
            extranonce_prefix_factory_standard: Arc::new(RwLock::new(
                ExtendedExtranonce::new(
                    range_0,
                    range_1,
                    range_2,
                    Some(coinbase_tag.as_bytes().to_vec()),
                )
                .expect("valid ExtendedExtranonce must not fail"),
            )),
            share_batch_size,
            expected_shares_per_minute,
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

        let standard_channels = Arc::new(RwLock::new(HashMap::new()));
        let extended_channels = Arc::new(RwLock::new(HashMap::new()));
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
            channel_id_factory: Arc::new(Mutex::new(channel_id_factory)),
            group_channel,
            standard_channels,
            extended_channels,
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

    async fn shutdown(&mut self) {}

    async fn handle_open_standard_mining_channel(
        &self,
        client_id: u32,
        m: OpenStandardMiningChannel<'static>,
    ) -> Result<ResponseFromSv2Server<'static>, RequestToSv2ServerError> {
        info!("Received OpenStandardMiningChannel message");
        let mut messages = Vec::new();

        let mut clients = self.clients.write().await;
        let client = clients.get_mut(&client_id).ok_or_else(|| {
            error!("Client with id {} not found", client_id);
            RequestToSv2ServerError::IdNotFound
        })?;

        // Get extranonce prefix
        let extranonce_prefix = {
            let mut factory = self.extranonce_prefix_factory_standard.write().await;
            match factory.next_prefix_standard() {
                Ok(prefix) => prefix.to_vec(),
                Err(e) => {
                    error!(
                        "Failed to get extranonce prefix for client {}: {:?}",
                        client_id, e
                    );
                    return Err(RequestToSv2ServerError::MiningHandlerError(format!(
                        "Failed to get extranonce prefix: {:?}",
                        e
                    )));
                }
            }
        };

        let channel_id = {
            let client_guard = client.read().await;
            let channel_id = client_guard.channel_id_factory.lock().await.next();
            channel_id
        };

        let user_identity = std::str::from_utf8(m.user_identity.as_ref())
            .map(|s| s.to_string())
            .map_err(|e| {
                error!("Invalid UTF-8 in user_identity: {:?}", e);
                RequestToSv2ServerError::MiningHandlerError(format!(
                    "Invalid UTF-8 in user_identity: {:?}",
                    e
                ))
            })?;

        // Clone max_target so m is not partially moved
        let max_target = m.max_target.clone();

        // Create standard channel
        let mut standard_channel = match StandardChannel::new(
            channel_id,
            user_identity,
            extranonce_prefix,
            max_target.into(),
            m.nominal_hash_rate,
            self.share_batch_size,
            self.expected_shares_per_minute,
        ) {
            Ok(channel) => channel,
            Err(e) => match e {
                StandardChannelError::InvalidNominalHashrate => {
                    error!("OpenMiningChannelError: invalid-nominal-hashrate");
                    let error_message = OpenMiningChannelError {
                        request_id: m.get_request_id_as_u32(),
                        error_code: "invalid-nominal-hashrate".to_string().try_into().unwrap(),
                    };
                    return Ok(ResponseFromSv2Server::TriggerNewRequest(Box::new(
                        RequestToSv2Server::SendMessagesToClient(Box::new(Sv2MessagesToClient {
                            client_id,
                            messages: vec![AnyMessage::Mining(Mining::OpenMiningChannelError(
                                error_message,
                            ))],
                        })),
                    )));
                }
                StandardChannelError::RequestedMaxTargetOutOfRange => {
                    error!("OpenMiningChannelError: requested-max-target-out-of-range");
                    let error_message = OpenMiningChannelError {
                        request_id: m.get_request_id_as_u32(),
                        error_code: "max-target-out-of-range".to_string().try_into().unwrap(),
                    };
                    return Ok(ResponseFromSv2Server::TriggerNewRequest(Box::new(
                        RequestToSv2Server::SendMessagesToClient(Box::new(Sv2MessagesToClient {
                            client_id,
                            messages: vec![AnyMessage::Mining(Mining::OpenMiningChannelError(
                                error_message,
                            ))],
                        })),
                    )));
                }
                _ => {
                    error!("error in handle_open_standard_mining_channel: {:?}", e);
                    return Err(RequestToSv2ServerError::MiningHandlerError(format!(
                        "Error creating standard channel: {:?}",
                        e
                    )));
                }
            },
        };

        // Extract needed fields before mutably borrowing standard_channel
        let target = standard_channel.get_target().clone().into();
        let extranonce_prefix = standard_channel
            .get_extranonce_prefix()
            .clone()
            .try_into()
            .map_err(|e| {
                error!("Failed to convert extranonce prefix: {:?}", e);
                RequestToSv2ServerError::MiningHandlerError(format!(
                    "Failed to convert extranonce prefix: {:?}",
                    e
                ))
            })?;

        // Get last activated future template
        let last_activated_future_template_guard = self.last_activated_future_template.read().await;
        let last_activated_future_template = (*last_activated_future_template_guard)
            .clone()
            .ok_or_else(|| {
                error!("No last activated future template available");
                RequestToSv2ServerError::MiningHandlerError(
                    "No last activated future template available".to_string(),
                )
            })?;
        let coinbase_tx_value_remaining =
            last_activated_future_template.coinbase_tx_value_remaining;
        let coinbase_output = TxOut {
            value: Amount::from_sat(coinbase_tx_value_remaining),
            script_pubkey: self.coinbase_output_script.clone(),
        };

        // Call on_new_template before moving standard_channel
        standard_channel
            .on_new_template(
                last_activated_future_template.clone(),
                vec![coinbase_output],
            )
            .map_err(|e| {
                error!("Error processing new template on standard channel: {:?}", e);
                RequestToSv2ServerError::MiningHandlerError(format!(
                    "Error processing new template on standard channel: {:?}",
                    e
                ))
            })?;

        // Get future standard job id
        let template_id = last_activated_future_template.template_id;
        let future_standard_job_id = *standard_channel
            .get_future_template_to_job_id()
            .get(&template_id)
            .ok_or_else(|| {
                error!(
                    "Future standard job should exist for template_id {}",
                    template_id
                );
                RequestToSv2ServerError::MiningHandlerError(format!(
                    "Future standard job should exist for template_id {}",
                    template_id
                ))
            })?;

        // Get future standard job
        let future_standard_job = standard_channel
            .get_future_jobs()
            .get(&future_standard_job_id)
            .ok_or_else(|| {
                error!(
                    "Future standard job should exist for job_id {}",
                    future_standard_job_id
                );
                RequestToSv2ServerError::MiningHandlerError(format!(
                    "Future standard job should exist for job_id {}",
                    future_standard_job_id
                ))
            })?;
        let future_job_message = future_standard_job.get_job_message().clone();

        // Call on_set_new_prev_hash on the standard channel before moving it
        let last_prev_hash = self.last_prev_hash.read().await.clone().ok_or_else(|| {
            error!("No last prev hash available");
            RequestToSv2ServerError::MiningHandlerError("No last prev hash available".to_string())
        })?;
        standard_channel
            .on_set_new_prev_hash(last_prev_hash.clone())
            .map_err(|e| {
                error!(
                    "Error processing SetNewPrevHash on standard channel: {:?}",
                    e
                );
                RequestToSv2ServerError::MiningHandlerError(format!(
                    "Error processing SetNewPrevHash on standard channel: {:?}",
                    e
                ))
            })?;

        // Prepare SetNewPrevHashMp message
        let set_new_prev_hash_mp = SetNewPrevHashMp {
            channel_id,
            job_id: future_standard_job_id,
            prev_hash: last_prev_hash.prev_hash.clone(),
            min_ntime: last_prev_hash.header_timestamp,
            nbits: last_prev_hash.n_bits,
        };

        // Register the new standard channel
        let client_guard = client.read().await;
        let std_channels_arc = &client_guard.standard_channels;
        std_channels_arc
            .write()
            .await
            .insert(channel_id, Arc::new(RwLock::new(standard_channel)));

        // Add channel to group channel if present
        if let Some(group_channel) = client_guard.group_channel.as_ref() {
            let mut group_channel = group_channel.write().await;
            group_channel.add_standard_channel_id(channel_id);
        }

        // Prepare response
        let group_channel_id = if let Some(gc) = client_guard.group_channel.as_ref() {
            gc.read().await.get_group_channel_id()
        } else {
            0
        };
        drop(client_guard);

        let open_standard_mining_channel_response = OpenStandardMiningChannelSuccess {
            request_id: m.request_id,
            channel_id,
            target,
            extranonce_prefix,
            group_channel_id,
        };

        messages.push(AnyMessage::Mining(
            Mining::OpenStandardMiningChannelSuccess(open_standard_mining_channel_response),
        ));
        messages.push(AnyMessage::Mining(Mining::NewMiningJob(future_job_message)));
        messages.push(AnyMessage::Mining(Mining::SetNewPrevHash(
            set_new_prev_hash_mp,
        )));

        info!(
            "Opened standard mining channel with id: {} for client: {}",
            channel_id, client_id
        );

        Ok(ResponseFromSv2Server::TriggerNewRequest(Box::new(
            RequestToSv2Server::SendMessagesToClient(Box::new(Sv2MessagesToClient {
                client_id,
                messages,
            })),
        )))
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

                        let group_extended_job = AnyMessage::Mining(Mining::NewExtendedMiningJob(
                            future_job.get_job_message().clone(),
                        ));

                        info!("Sending future NewExtendedMiningJob message to channel {} of client {:?} for job id {:?}", group_channel.get_group_channel_id(), client_id, future_job_id);
                        messages_to_client.push(group_extended_job);
                    }

                    // process standard channels
                    let standard_channels_arc = &client.standard_channels;
                    for (_, standard_channel_guard) in standard_channels_arc.read().await.iter() {
                        let mut standard_channel = standard_channel_guard.write().await;
                        standard_channel
                            .on_new_template(template.clone(), vec![coinbase_tx_output.clone()])
                            .map_err(|e| {
                                error!(
                                    "Error sending new future template to standard channel: {:?}",
                                    e
                                );
                                RequestToSv2ServerError::MiningHandlerError(format!(
                                    "Error sending new future template to standard channel: {:?}",
                                    e
                                ))
                            })?;

                        let future_job_id = standard_channel
                            .get_future_template_to_job_id()
                            .get(&template.template_id)
                            .ok_or_else(|| {
                                error!("Error getting future job id");
                                RequestToSv2ServerError::MiningHandlerError(format!(
                                    "Error getting future job id for template {:?} for standard channel {:?}",
                                    template.template_id,
                                    standard_channel.get_channel_id()
                                ))
                            })?;

                        let future_job = standard_channel.get_future_jobs().get(future_job_id).ok_or_else(|| {
                            error!("Error getting future job");
                            RequestToSv2ServerError::MiningHandlerError(format!(
                                "Error getting future job for template {:?} for standard channel {:?}",
                                template.template_id,
                                standard_channel.get_channel_id()
                            ))
                        })?;

                        let future_standard_job = AnyMessage::Mining(Mining::NewMiningJob(
                            future_job.get_job_message().clone(),
                        ));

                        info!("Sending future NewMiningJob message to channel {} of client {:?} for job id {:?}", standard_channel.get_channel_id(), client_id, future_job_id);
                        messages_to_client.push(future_standard_job);
                    }

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

                        let group_extended_job = AnyMessage::Mining(Mining::NewExtendedMiningJob(
                            active_job.get_job_message().clone(),
                        ));

                        info!("Sending non-future NewExtendedMiningJob message to channel {} of client {:?} for job id {:?}", group_channel.get_group_channel_id(), client_id, active_job.get_job_id());
                        messages_to_client.push(group_extended_job);
                    }

                    // process standard channels
                    let std_channels_arc = &client.standard_channels;
                    for (_, standard_channel_guard) in std_channels_arc.read().await.iter() {
                        let mut standard_channel = standard_channel_guard.write().await;
                        standard_channel
                            .on_new_template(template.clone(), vec![coinbase_tx_output.clone()])
                            .map_err(|e| {
                                error!("Error sending new template to standard channel: {:?}", e);
                                RequestToSv2ServerError::MiningHandlerError(format!(
                                    "Error sending new template to standard channel: {:?}",
                                    e
                                ))
                            })?;
                        let active_job = standard_channel.get_active_job().ok_or_else(|| {
                            error!("Error getting active job");
                            RequestToSv2ServerError::MiningHandlerError(format!(
                                "Error getting active job for standard channel {:?}",
                                standard_channel.get_channel_id()
                            ))
                        })?;
                        let standard_job = AnyMessage::Mining(Mining::NewMiningJob(
                            active_job.get_job_message().clone(),
                        ));
                        info!("Sending non-future NewMiningJob message to channel {} of client {:?} for job id {:?}", standard_channel.get_channel_id(), client_id, active_job.get_job_id());
                        messages_to_client.push(standard_job);
                    }
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
            let client_guard = client_guard.read().await;
            let mut messages_to_client = Vec::new();
            if let Some(group_channel_guard) = &client_guard.group_channel {
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

                let set_new_prev_hash_mp =
                    AnyMessage::Mining(Mining::SetNewPrevHash(set_new_prev_hash_mp));

                info!(
                    "Sending SetNewPrevHash message to channel {} of client {:?} for job id {:?}",
                    group_channel.get_group_channel_id(),
                    client_id,
                    active_job_id
                );
                messages_to_client.push(set_new_prev_hash_mp);
            }

            // process standard channels
            let std_channels_arc = &client_guard.standard_channels;
            for (_, standard_channel_guard) in std_channels_arc.read().await.iter() {
                let mut standard_channel = standard_channel_guard.write().await;

                standard_channel
                    .on_set_new_prev_hash(prev_hash.clone())
                    .map_err(|e| {
                        error!(
                            "Error processing SetNewPrevHash on standard channel: {:?}",
                            e
                        );
                        RequestToSv2ServerError::MiningHandlerError(format!(
                            "Error processing SetNewPrevHash on standard channel: {:?}",
                            e
                        ))
                    })?;

                let standard_channel_active_job =
                    standard_channel.get_active_job().ok_or_else(|| {
                        error!("Error getting active job");
                        RequestToSv2ServerError::MiningHandlerError(format!(
                            "Error getting active job for standard channel {:?}",
                            standard_channel.get_channel_id()
                        ))
                    })?;

                let active_job_id = standard_channel_active_job.get_job_id();

                let set_new_prev_hash_mp = SetNewPrevHashMp {
                    channel_id: standard_channel.get_channel_id(),
                    prev_hash: prev_hash.prev_hash.clone(),
                    job_id: active_job_id,
                    min_ntime: prev_hash.header_timestamp,
                    nbits: prev_hash.n_bits,
                };

                let set_new_prev_hash_mp =
                    AnyMessage::Mining(Mining::SetNewPrevHash(set_new_prev_hash_mp));

                info!(
                    "Sending SetNewPrevHash message to channel {} of client {:?} for job id {:?}",
                    standard_channel.get_channel_id(),
                    client_id,
                    active_job_id
                );
                messages_to_client.push(set_new_prev_hash_mp);
            }

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
