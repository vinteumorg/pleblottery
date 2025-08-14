use sv2_services::client::service::event::Sv2ClientEvent;
use sv2_services::client::service::subprotocols::template_distribution::trigger::TemplateDistributionClientTrigger;
use sv2_services::roles_logic_sv2::channels::server::error::ExtendedChannelError;
use sv2_services::roles_logic_sv2::channels::server::error::StandardChannelError;
use sv2_services::roles_logic_sv2::channels::server::extended::ExtendedChannel;
use sv2_services::roles_logic_sv2::channels::server::group::GroupChannel;
use sv2_services::roles_logic_sv2::channels::server::jobs::job_store::DefaultJobStore;
use sv2_services::roles_logic_sv2::channels::server::share_accounting::{
    ShareValidationError, ShareValidationResult,
};
use sv2_services::roles_logic_sv2::channels::server::standard::StandardChannel;
use sv2_services::roles_logic_sv2::mining_sv2::NewExtendedMiningJob;
use sv2_services::roles_logic_sv2::mining_sv2::NewMiningJob;
use sv2_services::roles_logic_sv2::mining_sv2::OpenExtendedMiningChannelSuccess;
use sv2_services::roles_logic_sv2::mining_sv2::UpdateChannelError;
use sv2_services::roles_logic_sv2::mining_sv2::{
    CloseChannel, OpenExtendedMiningChannel, OpenMiningChannelError, OpenStandardMiningChannel,
    OpenStandardMiningChannelSuccess, SetCustomMiningJob, SubmitSharesError, SubmitSharesExtended,
    SubmitSharesStandard, SubmitSharesSuccess, UpdateChannel, MAX_EXTRANONCE_LEN,
};
use sv2_services::roles_logic_sv2::mining_sv2::{
    ExtendedExtranonce, SetNewPrevHash as SetNewPrevHashMp,
};
use sv2_services::roles_logic_sv2::parsers::{AnyMessage, Mining};
use sv2_services::roles_logic_sv2::template_distribution_sv2::{
    NewTemplate, SetNewPrevHash, SubmitSolution,
};
use sv2_services::server::service::client::Sv2MessagesToClient;
use sv2_services::server::service::event::Sv2ServerEvent;
use sv2_services::server::service::event::Sv2ServerEventError;
use sv2_services::server::service::outcome::Sv2ServerOutcome;
use sv2_services::server::service::subprotocols::mining::handler::Sv2MiningServerHandler;
use tokio::sync::RwLock;

use crate::state::SharedStateHandle;

use bitcoin::{transaction::TxOut, Amount};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tracing::{error, info};

#[derive(Debug)]
pub struct PleblotteryMiningClient {
    pub client_id: u32,
    pub connection_flags: u32,
    pub channel_id_factory: AtomicU32,
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
    pub extranonce_prefix_factory_extended: Arc<RwLock<ExtendedExtranonce>>,
    pub share_batch_size: usize,
    pub expected_shares_per_minute: f32,
}

impl PlebLotteryMiningServerHandler {
    pub async fn new(
        shared_state: SharedStateHandle,
        coinbase_output_script: bitcoin::ScriptBuf,
        coinbase_tag: String,
        share_batch_size: usize,
        expected_shares_per_minute: f32,
    ) -> Self {
        let range_0 = std::ops::Range { start: 0, end: 0 };

        let full_coinbase_tag = format!("pleblottery {}", coinbase_tag);

        let range_1 = std::ops::Range {
            start: 0,
            end: full_coinbase_tag.len() + 8,
        };
        let range_2 = std::ops::Range {
            start: full_coinbase_tag.len() + 8,
            end: MAX_EXTRANONCE_LEN,
        };
        let clients = Arc::new(RwLock::new(HashMap::new()));
        shared_state.write().await.clients = clients.clone();

        Self {
            clients,
            shared_state,
            coinbase_output_script,
            future_templates: Arc::new(RwLock::new(HashMap::new())),
            last_activated_future_template: Arc::new(RwLock::new(None)),
            last_prev_hash: Arc::new(RwLock::new(None)),
            extranonce_prefix_factory_standard: Arc::new(RwLock::new(
                ExtendedExtranonce::new(
                    range_0.clone(),
                    range_1.clone(),
                    range_2.clone(),
                    Some(full_coinbase_tag.as_bytes().to_vec()),
                )
                .expect("valid ExtendedExtranonce must not fail"),
            )),
            extranonce_prefix_factory_extended: Arc::new(RwLock::new(
                ExtendedExtranonce::new(
                    range_0,
                    range_1,
                    range_2,
                    Some(full_coinbase_tag.as_bytes().to_vec()),
                )
                .expect("valid ExtendedExtranonce must not fail"),
            )),
            share_batch_size,
            expected_shares_per_minute,
        }
    }

    async fn get_client(
        &self,
        client_id: u32,
    ) -> Result<Arc<RwLock<PleblotteryMiningClient>>, Sv2ServerEventError> {
        let clients = self.clients.read().await;
        let client = match clients.get(&client_id) {
            Some(client) => client.clone(),
            None => {
                return Err(Sv2ServerEventError::MiningHandlerError(format!(
                    "Client with id {} not found",
                    client_id
                )));
            }
        };
        Ok(client)
    }

    async fn get_last_activated_template(&self) -> Option<NewTemplate<'static>> {
        let last_activated_future_template_guard = self.last_activated_future_template.read().await;
        let last_activated_future_template = (*last_activated_future_template_guard).clone();
        last_activated_future_template
    }

    async fn get_last_prev_hash(&self) -> Option<SetNewPrevHash<'static>> {
        let last_prev_hash_guard = self.last_prev_hash.read().await;
        let last_prev_hash = (*last_prev_hash_guard).clone();
        last_prev_hash
    }

    async fn get_coinbase_outputs(&self) -> Result<Vec<TxOut>, Sv2ServerEventError> {
        let future_template = self.get_last_activated_template().await.ok_or(
            Sv2ServerEventError::MiningHandlerError(format!("No last activated template found")),
        )?;
        let coinbase_txout = TxOut {
            value: Amount::from_sat(future_template.coinbase_tx_value_remaining),
            script_pubkey: self.coinbase_output_script.clone(),
        };
        Ok(vec![coinbase_txout])
    }

    async fn get_future_job_message_extended(
        &self,
        extended_channel: &ExtendedChannel<'static>,
    ) -> Result<(u32, NewExtendedMiningJob<'static>), Sv2ServerEventError> {
        let template = self.get_last_activated_template().await.ok_or(
            Sv2ServerEventError::MiningHandlerError(format!("No last activated template found")),
        )?;
        let future_job_id = *extended_channel
            .get_future_template_to_job_id()
            .get(&template.template_id)
            .ok_or(Sv2ServerEventError::MiningHandlerError(format!(
                "Future job must be present"
            )))?;
        let future_job = extended_channel
            .get_future_jobs()
            .get(&future_job_id)
            .ok_or(Sv2ServerEventError::MiningHandlerError(format!(
                "Future job must be present"
            )))?
            .get_job_message()
            .clone();

        Ok((future_job_id, future_job))
    }
    async fn get_future_job_message(
        &self,
        extended_channel: &StandardChannel<'static>,
    ) -> Result<(u32, NewMiningJob<'static>), Sv2ServerEventError> {
        let template = self.get_last_activated_template().await.ok_or(
            Sv2ServerEventError::MiningHandlerError(format!("No last activated template found")),
        )?;
        let future_job_id = *extended_channel
            .get_future_template_to_job_id()
            .get(&template.template_id)
            .ok_or(Sv2ServerEventError::MiningHandlerError(format!(
                "Future job must be present"
            )))?;
        let future_job = extended_channel
            .get_future_jobs()
            .get(&future_job_id)
            .ok_or(Sv2ServerEventError::MiningHandlerError(format!(
                "Future job must be present"
            )))?
            .get_job_message()
            .clone();

        Ok((future_job_id, future_job))
    }

    async fn register_extended_channel(
        &self,
        client_id: u32,
        channel_id: u32,
        extended_channel: ExtendedChannel<'static>,
    ) -> Result<(), Sv2ServerEventError> {
        // Register the new extended channel
        let client_guard = self.get_client(client_id).await?;
        let ext_channels_arc = &client_guard.read().await.extended_channels;
        ext_channels_arc
            .write()
            .await
            .insert(channel_id, Arc::new(RwLock::new(extended_channel)));
        Ok(())
    }

    async fn register_standard_channel(
        &self,
        client_id: u32,
        channel_id: u32,
        standard_channel: StandardChannel<'static>,
    ) -> Result<u32, Sv2ServerEventError> {
        // Register the new standard channel
        let client_guard = self.get_client(client_id).await?;
        let std_channels_arc = &client_guard.read().await.standard_channels;
        std_channels_arc
            .write()
            .await
            .insert(channel_id, Arc::new(RwLock::new(standard_channel)));

        // Add channel to group channel if present
        if let Some(group_channel) = client_guard.read().await.group_channel.as_ref() {
            let mut group_channel = group_channel.write().await;
            group_channel.add_standard_channel_id(channel_id);
        }

        // Prepare response
        let group_channel_id = if let Some(gc) = client_guard.read().await.group_channel.as_ref() {
            gc.read().await.get_group_channel_id()
        } else {
            0
        };
        Ok(group_channel_id)
    }
}

impl Sv2MiningServerHandler for PlebLotteryMiningServerHandler {
    fn setup_connection_success_flags(&self) -> u32 {
        // no requirement for fixed version field
        // no requirement for extended channel only
        0
    }

    async fn add_client(&mut self, client_id: u32, flags: u32) {
        info!("Adding client with id: {}, flags: {:04b}", client_id, flags);

        let channel_id_factory = AtomicU32::new(1);

        let standard_channels = Arc::new(RwLock::new(HashMap::new()));
        let extended_channels = Arc::new(RwLock::new(HashMap::new()));
        // if SetupConnection.REQUIRES_STANDARD_JOBS is set
        // client does not understand group channels
        let group_channel = if flags & 0x0001 == 0x0001 {
            None
        } else {
            let group_channel_id = channel_id_factory.fetch_add(1, Ordering::SeqCst);
            info!("Adding group channel with id: {}", group_channel_id);
            let job_store = Box::new(DefaultJobStore::new());
            Some(Arc::new(RwLock::new(GroupChannel::new(
                group_channel_id,
                job_store,
            ))))
        };

        let client = PleblotteryMiningClient {
            client_id,
            connection_flags: flags,
            channel_id_factory,
            group_channel,
            standard_channels,
            extended_channels,
        };

        self.clients
            .write()
            .await
            .insert(client_id, Arc::new(RwLock::new(client)));

        {
            let total_clients = self.clients.read().await.len() as u32;
            let mut state = self.shared_state.write().await;
            state.total_clients = total_clients;
        }
    }

    async fn remove_client(&mut self, client_id: u32) {
        info!("Removing client with id: {}", client_id);

        let hashrate = {
            let mut hash = 0.0;
            let clients_guard = self.clients.read().await;
            let client = match clients_guard.get(&client_id) {
                Some(c) => c,
                None => {
                    info!(
                        "Client {} not found in clients list — assuming already dropped.",
                        client_id
                    );
                    return;
                }
            };
            let client_guard = client.read().await;

            // Calculate hashrate from standard channels
            let std_channels = client_guard.standard_channels.read().await;
            for (_, channel) in std_channels.iter() {
                hash += channel.read().await.get_nominal_hashrate();
            }

            // Calculate hashrate from extended channels
            let ext_channels = client_guard.extended_channels.read().await;
            for (_, channel) in ext_channels.iter() {
                hash += channel.read().await.get_nominal_hashrate();
            }

            hash
        };
        self.clients.write().await.remove(&client_id);

        {
            let total_clients = self.clients.read().await.len() as u32;
            let mut state = self.shared_state.write().await;
            state.total_clients = total_clients;
            state.total_hashrate -= hashrate;
            // Ensure hashrate doesn't go negative due to floating point precision
            if state.total_hashrate < 0.0 {
                state.total_hashrate = 0.0;
            }
        }
    }

    async fn start(&mut self) -> Result<Sv2ServerOutcome<'static>, Sv2ServerEventError> {
        Ok(Sv2ServerOutcome::Ok)
    }

    async fn handle_open_standard_mining_channel(
        &self,
        client_id: u32,
        m: OpenStandardMiningChannel<'static>,
    ) -> Result<Sv2ServerOutcome<'static>, Sv2ServerEventError> {
        info!("Received OpenStandardMiningChannel message");
        let mut messages = Vec::new();

        let client = self.get_client(client_id).await?;

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
                    return Err(Sv2ServerEventError::MiningHandlerError(format!(
                        "Failed to get extranonce prefix: {:?}",
                        e
                    )));
                }
            }
        };

        let channel_id = {
            let client_guard = client.read().await;
            let channel_id = client_guard
                .channel_id_factory
                .fetch_add(1, Ordering::SeqCst);
            channel_id
        };

        let user_identity = std::str::from_utf8(m.user_identity.as_ref())
            .map(|s| s.to_string())
            .map_err(|e| {
                error!("Invalid UTF-8 in user_identity: {:?}", e);
                Sv2ServerEventError::MiningHandlerError(format!(
                    "Invalid UTF-8 in user_identity: {:?}",
                    e
                ))
            })?;

        // Clone max_target so m is not partially moved
        let max_target = m.max_target.clone();

        // Create standard channel
        let job_store = Box::new(DefaultJobStore::new());

        let mut standard_channel = match StandardChannel::new(
            channel_id,
            user_identity,
            extranonce_prefix,
            max_target.into(),
            m.nominal_hash_rate,
            self.share_batch_size,
            self.expected_shares_per_minute,
            job_store,
        ) {
            Ok(channel) => channel,
            Err(e) => match e {
                StandardChannelError::InvalidNominalHashrate => {
                    error!("OpenMiningChannelError: invalid-nominal-hashrate");
                    let error_message = OpenMiningChannelError {
                        request_id: m.get_request_id_as_u32(),
                        error_code: "invalid-nominal-hashrate".to_string().try_into().unwrap(),
                    };
                    return Ok(Sv2ServerOutcome::TriggerNewEvent(Box::new(
                        Sv2ServerEvent::SendMessagesToClient(Box::new(Sv2MessagesToClient {
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
                    return Ok(Sv2ServerOutcome::TriggerNewEvent(Box::new(
                        Sv2ServerEvent::SendMessagesToClient(Box::new(Sv2MessagesToClient {
                            client_id,
                            messages: vec![AnyMessage::Mining(Mining::OpenMiningChannelError(
                                error_message,
                            ))],
                        })),
                    )));
                }
                _ => {
                    error!("error in handle_open_standard_mining_channel: {:?}", e);
                    return Err(Sv2ServerEventError::MiningHandlerError(format!(
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
                Sv2ServerEventError::MiningHandlerError(format!(
                    "Failed to convert extranonce prefix: {:?}",
                    e
                ))
            })?;

        // Get last activated future template
        let last_activated_future_template = match self.get_last_activated_template().await {
            Some(template) => template,
            None => {
                error!("Unable to open standard mining channel with client {}: No last activated future template available", client_id);
                return Ok(Sv2ServerOutcome::TriggerNewEvent(Box::new(
                    Sv2ServerEvent::SendMessagesToClient(Box::new(Sv2MessagesToClient {
                        client_id: client_id,
                        messages: vec![AnyMessage::Mining(Mining::OpenMiningChannelError(
                            OpenMiningChannelError {
                                request_id: m.get_request_id_as_u32(),
                                error_code: "not-ready-to-open-channel" //note: non-standard error code
                                    .to_string()
                                    .try_into()
                                    .expect("error code must be valid string"),
                            },
                        ))],
                    })),
                )));
            }
        };
        let coinbase_output = self.get_coinbase_outputs().await?;

        // Call on_new_template before moving standard_channel
        standard_channel
            .on_new_template(last_activated_future_template.clone(), coinbase_output)
            .map_err(|e| {
                error!("Error processing new template on standard channel: {:?}", e);
                Sv2ServerEventError::MiningHandlerError(format!(
                    "Error processing new template on standard channel: {:?}",
                    e
                ))
            })?;

        let (future_standard_job_id, future_job_message) =
            self.get_future_job_message(&standard_channel).await?;

        let last_prev_hash = match self.get_last_prev_hash().await {
            Some(prev_hash) => prev_hash,
            None => {
                error!("Unable to open standard mining channel with client {}: No last activated prev hash available", client_id);
                return Ok(Sv2ServerOutcome::TriggerNewEvent(Box::new(
                    Sv2ServerEvent::SendMessagesToClient(Box::new(Sv2MessagesToClient {
                        client_id: client_id,
                        messages: vec![AnyMessage::Mining(Mining::OpenMiningChannelError(
                            OpenMiningChannelError {
                                request_id: m.get_request_id_as_u32(),
                                error_code: "not-ready-to-open-channel" //note: non-standard error code
                                    .to_string()
                                    .try_into()
                                    .expect("error code must be valid string"),
                            },
                        ))],
                    })),
                )));
            }
        };
        standard_channel
            .on_set_new_prev_hash(last_prev_hash.clone())
            .map_err(|e| {
                error!(
                    "Error processing SetNewPrevHash on standard channel: {:?}",
                    e
                );
                Sv2ServerEventError::MiningHandlerError(format!(
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

        let nominal_hashrate = standard_channel.get_nominal_hashrate();

        let group_channel_id = self
            .register_standard_channel(client_id, channel_id, standard_channel)
            .await?;

        let open_standard_mining_channel_response = OpenStandardMiningChannelSuccess {
            request_id: m.request_id,
            channel_id,
            target,
            extranonce_prefix,
            group_channel_id,
        };

        {
            let mut state = self.shared_state.write().await;
            state.total_hashrate += nominal_hashrate;
        }

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

        Ok(Sv2ServerOutcome::TriggerNewEvent(Box::new(
            Sv2ServerEvent::SendMessagesToClient(Box::new(Sv2MessagesToClient {
                client_id,
                messages,
            })),
        )))
    }

    async fn handle_open_extended_mining_channel(
        &self,
        client_id: u32,
        m: OpenExtendedMiningChannel<'static>,
    ) -> Result<Sv2ServerOutcome<'static>, Sv2ServerEventError> {
        info!("Received OpenExtendedMiningChannel message");

        let mut messages = Vec::new();

        let client = self.get_client(client_id).await?;

        let channel_id = {
            let client_guard = client.read().await;
            let channel_id = client_guard
                .channel_id_factory
                .fetch_add(1, Ordering::SeqCst);
            channel_id
        };

        let user_identity = std::str::from_utf8(m.user_identity.as_ref())
            .unwrap()
            .to_string();

        let extranonce_prefix = {
            self.extranonce_prefix_factory_extended
                .write()
                .await
                .next_prefix_standard()
                .map_err(|e| {
                    error!("Could not get extranonce prefix: {:?}", e);
                    Sv2ServerEventError::MiningHandlerError(format!(
                        "Could not get extranonce prefix: {:?}",
                        e
                    ))
                })?
                .to_vec()
        };

        let job_store = Box::new(DefaultJobStore::new());
        let mut extended_channel = match ExtendedChannel::new(
            channel_id,
            user_identity,
            extranonce_prefix,
            m.max_target.to_owned().into(),
            m.nominal_hash_rate,
            true,
            m.min_extranonce_size,
            self.share_batch_size,
            self.expected_shares_per_minute,
            job_store,
        ) {
            Ok(channel) => channel,
            Err(e) => match e {
                ExtendedChannelError::InvalidNominalHashrate => {
                    error!("OpenMiningChannelError: invalid-nominal-hashrate");
                    let error_message = OpenMiningChannelError {
                        request_id: m.get_request_id_as_u32(),
                        error_code: "invalid-nominal-hashrate".to_string().try_into().unwrap(),
                    };
                    return Ok(Sv2ServerOutcome::TriggerNewEvent(Box::new(
                        Sv2ServerEvent::SendMessagesToClient(Box::new(Sv2MessagesToClient {
                            client_id,
                            messages: vec![AnyMessage::Mining(Mining::OpenMiningChannelError(
                                error_message,
                            ))],
                        })),
                    )));
                }
                ExtendedChannelError::RequestedMaxTargetOutOfRange => {
                    error!("OpenMiningChannelError: max-target-out-of-range");
                    let error_message = OpenMiningChannelError {
                        request_id: m.get_request_id_as_u32(),
                        error_code: "max-target-out-of-range".to_string().try_into().unwrap(),
                    };
                    return Ok(Sv2ServerOutcome::TriggerNewEvent(Box::new(
                        Sv2ServerEvent::SendMessagesToClient(Box::new(Sv2MessagesToClient {
                            client_id,
                            messages: vec![AnyMessage::Mining(Mining::OpenMiningChannelError(
                                error_message,
                            ))],
                        })),
                    )));
                }
                ExtendedChannelError::RequestedMinExtranonceSizeTooLarge => {
                    error!("OpenMiningChannelError: min-extranonce-size-too-large");
                    let error_message = OpenMiningChannelError {
                        request_id: m.get_request_id_as_u32(),
                        error_code: "min-extranonce-size-too-large"
                            .to_string()
                            .try_into()
                            .unwrap(),
                    };
                    return Ok(Sv2ServerOutcome::TriggerNewEvent(Box::new(
                        Sv2ServerEvent::SendMessagesToClient(Box::new(Sv2MessagesToClient {
                            client_id,
                            messages: vec![AnyMessage::Mining(Mining::OpenMiningChannelError(
                                error_message,
                            ))],
                        })),
                    )));
                }
                _ => {
                    error!("Error in handle_open_extended_mining_channel: {:?}", e);
                    return Err(Sv2ServerEventError::MiningHandlerError(format!(
                        "Error creating extended channel: {:?}",
                        e
                    )));
                }
            },
        };

        let last_activated_future_template = match self.get_last_activated_template().await {
            Some(template) => template,
            None => {
                error!("Unable to open standard mining channel with client {}: No last activated future template available", client_id);
                return Ok(Sv2ServerOutcome::TriggerNewEvent(Box::new(
                    Sv2ServerEvent::SendMessagesToClient(Box::new(Sv2MessagesToClient {
                        client_id: client_id,
                        messages: vec![AnyMessage::Mining(Mining::OpenMiningChannelError(
                            OpenMiningChannelError {
                                request_id: m.get_request_id_as_u32(),
                                error_code: "not-ready-to-open-channel" //note: non-standard error code
                                    .to_string()
                                    .try_into()
                                    .expect("error code must be valid string"),
                            },
                        ))],
                    })),
                )));
            }
        };
        let coinbase_outputs = self.get_coinbase_outputs().await?;

        extended_channel
            .on_new_template(last_activated_future_template.clone(), coinbase_outputs)
            .map_err(|e| {
                error!("Error processing new template on extended channel: {:?}", e);
                Sv2ServerEventError::MiningHandlerError(format!(
                    "Error processing new template on extended channel: {:?}",
                    e
                ))
            })?;

        let (future_job_id, future_job_message) = self
            .get_future_job_message_extended(&extended_channel)
            .await?;

        let last_prev_hash = match self.get_last_prev_hash().await {
            Some(prev_hash) => prev_hash,
            None => {
                error!("Unable to open standard mining channel with client {}: No last activated prev hash available", client_id);
                return Ok(Sv2ServerOutcome::TriggerNewEvent(Box::new(
                    Sv2ServerEvent::SendMessagesToClient(Box::new(Sv2MessagesToClient {
                        client_id: client_id,
                        messages: vec![AnyMessage::Mining(Mining::OpenMiningChannelError(
                            OpenMiningChannelError {
                                request_id: m.get_request_id_as_u32(),
                                error_code: "not-ready-to-open-channel" //note: non-standard error code
                                    .to_string()
                                    .try_into()
                                    .expect("error code must be valid string"),
                            },
                        ))],
                    })),
                )));
            }
        };

        // Now mutably borrow extended_channel
        extended_channel
            .on_set_new_prev_hash(last_prev_hash.clone())
            .expect("Error processing SetNewPrevHash on extended channel");

        let oxmcs = OpenExtendedMiningChannelSuccess {
            request_id: m.request_id.clone(),
            channel_id,
            target: extended_channel.get_target().clone().into(),
            extranonce_size: extended_channel.get_rollable_extranonce_size(),
            extranonce_prefix: extended_channel
                .get_extranonce_prefix()
                .clone()
                .try_into()
                .expect("could not parse extranonce prefix"),
        };

        messages.push(AnyMessage::Mining(
            Mining::OpenExtendedMiningChannelSuccess(oxmcs),
        ));
        messages.push(AnyMessage::Mining(Mining::NewExtendedMiningJob(
            future_job_message,
        )));

        //get set new prev hash message
        let snphmp = SetNewPrevHashMp {
            channel_id,
            job_id: future_job_id,
            prev_hash: last_prev_hash.prev_hash,
            min_ntime: last_prev_hash.header_timestamp,
            nbits: last_prev_hash.n_bits,
        };
        messages.push(AnyMessage::Mining(Mining::SetNewPrevHash(snphmp)));

        let nominal_hashrate = extended_channel.get_nominal_hashrate();

        // Register the new extended channel
        self.register_extended_channel(client_id, channel_id, extended_channel)
            .await?;

        {
            let mut state = self.shared_state.write().await;
            state.total_hashrate += nominal_hashrate;
        }

        Ok(Sv2ServerOutcome::TriggerNewEvent(Box::new(
            Sv2ServerEvent::SendMessagesToClient(Box::new(Sv2MessagesToClient {
                client_id,
                messages,
            })),
        )))
    }

    async fn handle_update_channel(
        &self,
        client_id: u32,
        m: UpdateChannel<'static>,
    ) -> Result<Sv2ServerOutcome<'static>, Sv2ServerEventError> {
        info!("Received UpdateChannel message");
        let client = self.get_client(client_id).await?;
        let is_standard_channel = client
            .read()
            .await
            .standard_channels
            .read()
            .await
            .contains_key(&m.channel_id);

        let is_extended_channel = client
            .read()
            .await
            .extended_channels
            .read()
            .await
            .contains_key(&m.channel_id);

        if is_standard_channel {
            let client_read_guard = client.read().await;
            let std_channels_read_guard = client_read_guard.standard_channels.read().await;
            let standard_channel = std_channels_read_guard
                .get(&m.channel_id)
                .expect("Standard channel must exist");

            match standard_channel.write().await.update_channel(
                m.nominal_hash_rate,
                Some(m.maximum_target.into_static().into()),
            ) {
                Ok(()) => {
                    info!("Updated standard channel | channel_id: {}", m.channel_id);
                    return Ok(Sv2ServerOutcome::Ok);
                }
                Err(e) => match e {
                    StandardChannelError::InvalidNominalHashrate => {
                        error!("UpdateChannelError: invalid-nominal-hashrate");
                        let update_channel_error = UpdateChannelError {
                            channel_id: m.channel_id,
                            error_code: "invalid-nominal-hashrate"
                                .to_string()
                                .try_into()
                                .expect("error code must be valid string"),
                        };
                        return Ok(Sv2ServerOutcome::TriggerNewEvent(Box::new(
                            Sv2ServerEvent::SendMessagesToClient(Box::new(Sv2MessagesToClient {
                                client_id,
                                messages: vec![AnyMessage::Mining(Mining::UpdateChannelError(
                                    update_channel_error,
                                ))],
                            })),
                        )));
                    }
                    StandardChannelError::RequestedMaxTargetOutOfRange => {
                        error!("UpdateChannelError: requested-max-target-out-of-range");
                        let update_channel_error = UpdateChannelError {
                            channel_id: m.channel_id,
                            error_code: "requested-max-target-out-of-range"
                                .to_string()
                                .try_into()
                                .expect("error code must be valid string"),
                        };
                        return Ok(Sv2ServerOutcome::TriggerNewEvent(Box::new(
                            Sv2ServerEvent::SendMessagesToClient(Box::new(Sv2MessagesToClient {
                                client_id,
                                messages: vec![AnyMessage::Mining(Mining::UpdateChannelError(
                                    update_channel_error,
                                ))],
                            })),
                        )));
                    }
                    _ => {
                        return Err(Sv2ServerEventError::MiningHandlerError(format!(
                            "Error updating standard channel: {:?}",
                            e
                        )));
                    }
                },
            };
        } else if is_extended_channel {
            // Scope the client_read_guard so it is dropped before the await
            let extended_channel = {
                let client_read_guard = client.read().await;
                let ext_channels_read_guard = client_read_guard.extended_channels.read().await;
                ext_channels_read_guard
                    .get(&m.channel_id)
                    .expect("Extended channel must exist")
                    .clone()
            };

            let update_result = {
                let mut channel = extended_channel.write().await;
                channel.update_channel(
                    m.nominal_hash_rate,
                    Some(m.maximum_target.into_static().into()),
                )
            };

            match update_result {
                Ok(()) => {
                    info!("Updated extended channel | channel_id: {}", m.channel_id);
                    return Ok(Sv2ServerOutcome::Ok);
                }
                Err(e) => match e {
                    ExtendedChannelError::InvalidNominalHashrate => {
                        error!("UpdateChannelError: invalid-nominal-hashrate");
                        let update_channel_error = UpdateChannelError {
                            channel_id: m.channel_id,
                            error_code: "invalid-nominal-hashrate"
                                .to_string()
                                .try_into()
                                .expect("error code must be valid string"),
                        };
                        return Ok(Sv2ServerOutcome::TriggerNewEvent(Box::new(
                            Sv2ServerEvent::SendMessagesToClient(Box::new(Sv2MessagesToClient {
                                client_id,
                                messages: vec![AnyMessage::Mining(Mining::UpdateChannelError(
                                    update_channel_error,
                                ))],
                            })),
                        )));
                    }
                    ExtendedChannelError::RequestedMaxTargetOutOfRange => {
                        error!("UpdateChannelError: requested-max-target-out-of-range");
                        let update_channel_error = UpdateChannelError {
                            channel_id: m.channel_id,
                            error_code: "requested-max-target-out-of-range"
                                .to_string()
                                .try_into()
                                .expect("error code must be valid string"),
                        };
                        return Ok(Sv2ServerOutcome::TriggerNewEvent(Box::new(
                            Sv2ServerEvent::SendMessagesToClient(Box::new(Sv2MessagesToClient {
                                client_id,
                                messages: vec![AnyMessage::Mining(Mining::UpdateChannelError(
                                    update_channel_error,
                                ))],
                            })),
                        )));
                    }
                    _ => {
                        return Err(Sv2ServerEventError::MiningHandlerError(format!(
                            "Error updating extended channel: {:?}",
                            e
                        )));
                    }
                },
            }
        } else {
            error!(
                "UpdateChannelError: channel_id: {}, error_code: invalid-channel-id ❌",
                m.channel_id
            );
            let update_channel_error = UpdateChannelError {
                channel_id: m.channel_id,
                error_code: "invalid-channel-id"
                    .to_string()
                    .try_into()
                    .expect("error code must be valid string"),
            };
            return Ok(Sv2ServerOutcome::TriggerNewEvent(Box::new(
                Sv2ServerEvent::SendMessagesToClient(Box::new(Sv2MessagesToClient {
                    client_id,
                    messages: vec![AnyMessage::Mining(Mining::UpdateChannelError(
                        update_channel_error,
                    ))],
                })),
            )));
        }
    }

    async fn handle_close_channel(
        &self,
        _client_id: u32,
        _m: CloseChannel<'static>,
    ) -> Result<Sv2ServerOutcome<'static>, Sv2ServerEventError> {
        info!("Received CloseChannel message");
        Ok(Sv2ServerOutcome::Ok)
    }

    async fn handle_submit_shares_standard(
        &self,
        client_id: u32,
        m: SubmitSharesStandard,
    ) -> Result<Sv2ServerOutcome<'static>, Sv2ServerEventError> {
        info!("Received SubmitSharesStandard message");
        let clients_guard = self.clients.read().await;
        let client = match clients_guard.get(&client_id) {
            Some(client) => client,
            None => {
                error!("Client with id {} not found", client_id);
                return Err(Sv2ServerEventError::IdNotFound);
            }
        };

        let client_guard = client.read().await;
        let standard_channels_arc = &client_guard.standard_channels;
        let std_channels_guard = standard_channels_arc.read().await;
        let standard_channel_arc = match std_channels_guard.get(&m.channel_id) {
            Some(channel) => channel,
            None => {
                error!("SubmitSharesError: channel_id: {}, sequence_number: {}, error_code: invalid-channel-id ❌", m.channel_id, m.sequence_number);
                return Ok(Sv2ServerOutcome::TriggerNewEvent(Box::new(
                    Sv2ServerEvent::SendMessagesToClient(Box::new(Sv2MessagesToClient {
                        client_id,
                        messages: vec![AnyMessage::Mining(Mining::SubmitSharesError(
                            SubmitSharesError {
                                channel_id: m.channel_id,
                                sequence_number: m.sequence_number,
                                error_code: "invalid-channel-id"
                                    .to_string()
                                    .try_into()
                                    .expect("error code must be valid string"),
                            },
                        ))],
                    })),
                )));
            }
        };

        let mut standard_channel = standard_channel_arc.write().await;
        let share_validation_result = standard_channel.validate_share(m.clone());

        match share_validation_result {
            Ok(ShareValidationResult::Valid) => {
                info!(
                    "SubmitSharesStandard: valid share | channel_id: {}, sequence_number: {} ☑️",
                    m.channel_id, m.sequence_number
                );
                {
                    let mut state = self.shared_state.write().await;
                    state.total_shares_submitted += 1;
                }
                return Ok(Sv2ServerOutcome::Ok);
            }
            Ok(ShareValidationResult::ValidWithAcknowledgement(
                last_sequence_number,
                new_submits_accepted_count,
                new_shares_sum,
            )) => {
                let success = SubmitSharesSuccess {
                    channel_id: m.channel_id,
                    last_sequence_number,
                    new_submits_accepted_count,
                    new_shares_sum,
                };
                info!("SubmitSharesExtended: {:?} ✅", success);

                {
                    let share_accounting = standard_channel.get_share_accounting();
                    let mut state = self.shared_state.write().await;
                    let best_share = share_accounting.get_best_diff();
                    state.best_share = if best_share > state.best_share {
                        best_share
                    } else {
                        state.best_share
                    };
                    state.total_shares_submitted += 1;
                }
                return Ok(Sv2ServerOutcome::TriggerNewEvent(Box::new(
                    Sv2ServerEvent::SendMessagesToClient(Box::new(Sv2MessagesToClient {
                        client_id,
                        messages: vec![AnyMessage::Mining(Mining::SubmitSharesSuccess(success))],
                    })),
                )));
            }
            Ok(ShareValidationResult::BlockFound(template_id, coinbase)) => {
                info!("SubmitSharesStandard: 💰 Block Found!!! 💰");
                let template_id = template_id
                    .expect("Pleblottery does not support custom jobs. Something weird happened.");

                info!("SubmitSharesStandard: Propagating solution to the Template Provider.");

                {
                    let mut state = self.shared_state.write().await;
                    state.blocks_found += 1;
                }

                let share_accounting = standard_channel.get_share_accounting();

                return Ok(Sv2ServerOutcome::TriggerNewEvent(Box::new(
                    Sv2ServerEvent::MultipleEvents(Box::new(vec![
                        Sv2ServerEvent::SendEventToSiblingClientService(Box::new(
                            Sv2ClientEvent::TemplateDistributionTrigger(
                                TemplateDistributionClientTrigger::SubmitSolution(SubmitSolution {
                                    template_id,
                                    version: m.version,
                                    header_timestamp: m.ntime,
                                    header_nonce: m.nonce,
                                    coinbase_tx: coinbase
                                        .try_into()
                                        .expect("coinbase tx must be valid"),
                                }),
                            ),
                        )),
                        Sv2ServerEvent::SendMessagesToClient(Box::new(Sv2MessagesToClient {
                            client_id,
                            messages: vec![AnyMessage::Mining(Mining::SubmitSharesSuccess(
                                SubmitSharesSuccess {
                                    channel_id: m.channel_id,
                                    last_sequence_number: share_accounting
                                        .get_last_share_sequence_number(),
                                    new_submits_accepted_count: share_accounting
                                        .get_shares_accepted(),
                                    new_shares_sum: share_accounting.get_share_work_sum(),
                                },
                            ))],
                        })),
                    ])),
                )));
            }
            Err(ShareValidationError::Invalid) => {
                error!("SubmitSharesError: channel_id: {}, sequence_number: {}, error_code: invalid-share ❌", m.channel_id, m.sequence_number);
                return Ok(Sv2ServerOutcome::TriggerNewEvent(Box::new(
                    Sv2ServerEvent::SendMessagesToClient(Box::new(Sv2MessagesToClient {
                        client_id,
                        messages: vec![AnyMessage::Mining(Mining::SubmitSharesError(
                            SubmitSharesError {
                                channel_id: m.channel_id,
                                sequence_number: m.sequence_number,
                                error_code: "invalid-share"
                                    .to_string()
                                    .try_into()
                                    .expect("error code must be valid string"),
                            },
                        ))],
                    })),
                )));
            }
            Err(ShareValidationError::Stale) => {
                error!("SubmitSharesError: channel_id: {}, sequence_number: {}, error_code: stale-share ❌", m.channel_id, m.sequence_number);
                return Ok(Sv2ServerOutcome::TriggerNewEvent(Box::new(
                    Sv2ServerEvent::SendMessagesToClient(Box::new(Sv2MessagesToClient {
                        client_id,
                        messages: vec![AnyMessage::Mining(Mining::SubmitSharesError(
                            SubmitSharesError {
                                channel_id: m.channel_id,
                                sequence_number: m.sequence_number,
                                error_code: "stale-share"
                                    .to_string()
                                    .try_into()
                                    .expect("error code must be valid string"),
                            },
                        ))],
                    })),
                )));
            }
            Err(ShareValidationError::InvalidJobId) => {
                error!("SubmitSharesError: channel_id: {}, sequence_number: {}, error_code: invalid-job-id ❌", m.channel_id, m.sequence_number);
                return Ok(Sv2ServerOutcome::TriggerNewEvent(Box::new(
                    Sv2ServerEvent::SendMessagesToClient(Box::new(Sv2MessagesToClient {
                        client_id,
                        messages: vec![AnyMessage::Mining(Mining::SubmitSharesError(
                            SubmitSharesError {
                                channel_id: m.channel_id,
                                sequence_number: m.sequence_number,
                                error_code: "invalid-job-id"
                                    .to_string()
                                    .try_into()
                                    .expect("error code must be valid string"),
                            },
                        ))],
                    })),
                )));
            }
            Err(ShareValidationError::DoesNotMeetTarget) => {
                error!("SubmitSharesError: channel_id: {}, sequence_number: {}, error_code: difficulty-too-low ❌", m.channel_id, m.sequence_number);
                return Ok(Sv2ServerOutcome::TriggerNewEvent(Box::new(
                    Sv2ServerEvent::SendMessagesToClient(Box::new(Sv2MessagesToClient {
                        client_id,
                        messages: vec![AnyMessage::Mining(Mining::SubmitSharesError(
                            SubmitSharesError {
                                channel_id: m.channel_id,
                                sequence_number: m.sequence_number,
                                error_code: "difficulty-too-low"
                                    .to_string()
                                    .try_into()
                                    .expect("error code must be valid string"),
                            },
                        ))],
                    })),
                )));
            }
            Err(ShareValidationError::DuplicateShare) => {
                error!("SubmitSharesError: channel_id: {}, sequence_number: {}, error_code: duplicate-share ❌", m.channel_id, m.sequence_number);

                return Ok(Sv2ServerOutcome::TriggerNewEvent(Box::new(
                    Sv2ServerEvent::SendMessagesToClient(Box::new(Sv2MessagesToClient {
                        client_id,
                        messages: vec![AnyMessage::Mining(Mining::SubmitSharesError(
                            SubmitSharesError {
                                channel_id: m.channel_id,
                                sequence_number: m.sequence_number,
                                error_code: "duplicate-share"
                                    .to_string()
                                    .try_into()
                                    .expect("error code must be valid string"),
                            },
                        ))],
                    })),
                )));
            }
            _ => {
                error!(
                    "Unhandled share validation result for client: {}",
                    client_id
                );
                return Err(Sv2ServerEventError::MiningHandlerError(format!(
                    "Unhandled share validation result for client: {}",
                    client_id
                )));
            }
        }
    }

    async fn handle_submit_shares_extended(
        &self,
        client_id: u32,
        m: SubmitSharesExtended<'static>,
    ) -> Result<Sv2ServerOutcome<'static>, Sv2ServerEventError> {
        info!("Received SubmitSharesExtended message");
        let clients_guard = self.clients.read().await;
        let client = match clients_guard.get(&client_id) {
            Some(client) => client,
            None => {
                error!("Client with id {} not found", client_id);
                return Err(Sv2ServerEventError::IdNotFound);
            }
        };

        let client_guard = client.read().await;
        let extended_channels_arc = &client_guard.extended_channels;
        let ext_channels_guard = extended_channels_arc.read().await;
        let extended_channel_arc = match ext_channels_guard.get(&m.channel_id) {
            Some(channel) => channel,
            None => {
                error!("SubmitSharesError: channel_id: {}, sequence_number: {}, error_code: invalid-channel-id ❌", m.channel_id, m.sequence_number);
                return Ok(Sv2ServerOutcome::TriggerNewEvent(Box::new(
                    Sv2ServerEvent::SendMessagesToClient(Box::new(Sv2MessagesToClient {
                        client_id,
                        messages: vec![AnyMessage::Mining(Mining::SubmitSharesError(
                            SubmitSharesError {
                                channel_id: m.channel_id,
                                sequence_number: m.sequence_number,
                                error_code: "invalid-channel-id"
                                    .to_string()
                                    .try_into()
                                    .expect("error code must be valid string"),
                            },
                        ))],
                    })),
                )));
            }
        };

        let mut extended_channel = extended_channel_arc.write().await;
        let share_validation_result = extended_channel.validate_share(m.clone());

        match share_validation_result {
            Ok(ShareValidationResult::Valid) => {
                info!(
                    "SubmitSharesExtended: valid share | channel_id: {}, sequence_number: {} ☑️",
                    m.channel_id, m.sequence_number
                );
                {
                    let mut state = self.shared_state.write().await;
                    state.total_shares_submitted += 1;
                }
                return Ok(Sv2ServerOutcome::Ok);
            }
            Ok(ShareValidationResult::ValidWithAcknowledgement(
                last_sequence_number,
                new_submits_accepted_count,
                new_shares_sum,
            )) => {
                let success = SubmitSharesSuccess {
                    channel_id: m.channel_id,
                    last_sequence_number,
                    new_submits_accepted_count,
                    new_shares_sum,
                };
                info!("SubmitSharesExtended: {:?} ✅", success);

                {
                    let share_accounting = extended_channel.get_share_accounting();
                    let mut state = self.shared_state.write().await;
                    let best_share = share_accounting.get_best_diff();
                    state.best_share = if best_share > state.best_share {
                        best_share
                    } else {
                        state.best_share
                    };
                    state.total_shares_submitted += 1;
                }
                return Ok(Sv2ServerOutcome::TriggerNewEvent(Box::new(
                    Sv2ServerEvent::SendMessagesToClient(Box::new(Sv2MessagesToClient {
                        client_id,
                        messages: vec![AnyMessage::Mining(Mining::SubmitSharesSuccess(success))],
                    })),
                )));
            }
            Ok(ShareValidationResult::BlockFound(template_id, coinbase)) => {
                info!("SubmitSharesExtended: 💰 Block Found!!! 💰");
                let template_id = template_id
                    .expect("Pleblottery does not support custom jobs. Something weird happened.");

                info!("SubmitSharesExtended: Propagating solution to the Template Provider.");

                {
                    let mut state = self.shared_state.write().await;
                    state.blocks_found += 1;
                }

                let share_accounting = extended_channel.get_share_accounting();

                return Ok(Sv2ServerOutcome::TriggerNewEvent(Box::new(
                    Sv2ServerEvent::MultipleEvents(Box::new(vec![
                        Sv2ServerEvent::SendEventToSiblingClientService(Box::new(
                            Sv2ClientEvent::TemplateDistributionTrigger(
                                TemplateDistributionClientTrigger::SubmitSolution(SubmitSolution {
                                    template_id,
                                    version: m.version,
                                    header_timestamp: m.ntime,
                                    header_nonce: m.nonce,
                                    coinbase_tx: coinbase
                                        .try_into()
                                        .expect("coinbase tx must be valid"),
                                }),
                            ),
                        )),
                        Sv2ServerEvent::SendMessagesToClient(Box::new(Sv2MessagesToClient {
                            client_id,
                            messages: vec![AnyMessage::Mining(Mining::SubmitSharesSuccess(
                                SubmitSharesSuccess {
                                    channel_id: m.channel_id,
                                    last_sequence_number: share_accounting
                                        .get_last_share_sequence_number(),
                                    new_submits_accepted_count: share_accounting
                                        .get_shares_accepted(),
                                    new_shares_sum: share_accounting.get_share_work_sum(),
                                },
                            ))],
                        })),
                    ])),
                )));
            }
            Err(ShareValidationError::Invalid) => {
                error!("SubmitSharesError: channel_id: {}, sequence_number: {}, error_code: invalid-share ❌", m.channel_id, m.sequence_number);
                return Ok(Sv2ServerOutcome::TriggerNewEvent(Box::new(
                    Sv2ServerEvent::SendMessagesToClient(Box::new(Sv2MessagesToClient {
                        client_id,
                        messages: vec![AnyMessage::Mining(Mining::SubmitSharesError(
                            SubmitSharesError {
                                channel_id: m.channel_id,
                                sequence_number: m.sequence_number,
                                error_code: "invalid-share"
                                    .to_string()
                                    .try_into()
                                    .expect("error code must be valid string"),
                            },
                        ))],
                    })),
                )));
            }
            Err(ShareValidationError::Stale) => {
                error!("SubmitSharesError: channel_id: {}, sequence_number: {}, error_code: stale-share ❌", m.channel_id, m.sequence_number);
                return Ok(Sv2ServerOutcome::TriggerNewEvent(Box::new(
                    Sv2ServerEvent::SendMessagesToClient(Box::new(Sv2MessagesToClient {
                        client_id,
                        messages: vec![AnyMessage::Mining(Mining::SubmitSharesError(
                            SubmitSharesError {
                                channel_id: m.channel_id,
                                sequence_number: m.sequence_number,
                                error_code: "stale-share"
                                    .to_string()
                                    .try_into()
                                    .expect("error code must be valid string"),
                            },
                        ))],
                    })),
                )));
            }
            Err(ShareValidationError::InvalidJobId) => {
                error!("SubmitSharesError: channel_id: {}, sequence_number: {}, error_code: invalid-job-id ❌", m.channel_id, m.sequence_number);
                return Ok(Sv2ServerOutcome::TriggerNewEvent(Box::new(
                    Sv2ServerEvent::SendMessagesToClient(Box::new(Sv2MessagesToClient {
                        client_id,
                        messages: vec![AnyMessage::Mining(Mining::SubmitSharesError(
                            SubmitSharesError {
                                channel_id: m.channel_id,
                                sequence_number: m.sequence_number,
                                error_code: "invalid-job-id"
                                    .to_string()
                                    .try_into()
                                    .expect("error code must be valid string"),
                            },
                        ))],
                    })),
                )));
            }
            Err(ShareValidationError::DoesNotMeetTarget) => {
                error!("SubmitSharesError: channel_id: {}, sequence_number: {}, error_code: difficulty-too-low ❌", m.channel_id, m.sequence_number);
                return Ok(Sv2ServerOutcome::TriggerNewEvent(Box::new(
                    Sv2ServerEvent::SendMessagesToClient(Box::new(Sv2MessagesToClient {
                        client_id,
                        messages: vec![AnyMessage::Mining(Mining::SubmitSharesError(
                            SubmitSharesError {
                                channel_id: m.channel_id,
                                sequence_number: m.sequence_number,
                                error_code: "difficulty-too-low"
                                    .to_string()
                                    .try_into()
                                    .expect("error code must be valid string"),
                            },
                        ))],
                    })),
                )));
            }
            Err(ShareValidationError::DuplicateShare) => {
                error!("SubmitSharesError: channel_id: {}, sequence_number: {}, error_code: duplicate-share ❌", m.channel_id, m.sequence_number);

                return Ok(Sv2ServerOutcome::TriggerNewEvent(Box::new(
                    Sv2ServerEvent::SendMessagesToClient(Box::new(Sv2MessagesToClient {
                        client_id,
                        messages: vec![AnyMessage::Mining(Mining::SubmitSharesError(
                            SubmitSharesError {
                                channel_id: m.channel_id,
                                sequence_number: m.sequence_number,
                                error_code: "duplicate-share"
                                    .to_string()
                                    .try_into()
                                    .expect("error code must be valid string"),
                            },
                        ))],
                    })),
                )));
            }
            _ => {
                error!(
                    "Unhandled share validation result for client: {}",
                    client_id
                );
                return Err(Sv2ServerEventError::MiningHandlerError(format!(
                    "Unhandled share validation result for client: {}",
                    client_id
                )));
            }
        }
    }

    async fn handle_set_custom_mining_job(
        &self,
        _client_id: u32,
        _m: SetCustomMiningJob<'static>,
    ) -> Result<Sv2ServerOutcome<'static>, Sv2ServerEventError> {
        panic!("Pleblottery does not support custom mining jobs.");
    }

    async fn on_new_template(
        &self,
        template: NewTemplate<'static>,
    ) -> Result<Sv2ServerOutcome<'static>, Sv2ServerEventError> {
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
                                Sv2ServerEventError::MiningHandlerError(format!(
                                    "Error sending new template to group channel: {:?}",
                                    e
                                ))
                            })?;

                        let future_job_id = group_channel.get_future_template_to_job_id().get(&template.template_id).ok_or_else(|| {
                            error!("Error getting future job id");
                            Sv2ServerEventError::MiningHandlerError(format!("Error getting future job id for template {:?} for group channel {:?}", template.template_id, group_channel.get_group_channel_id()))
                        })?;

                        let future_job = group_channel.get_future_jobs().get(future_job_id).ok_or_else(|| {
                            error!("Error getting future job");
                            Sv2ServerEventError::MiningHandlerError(format!("Error getting future job for template {:?} for group channel {:?}", template.template_id, group_channel.get_group_channel_id()))
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
                                Sv2ServerEventError::MiningHandlerError(format!(
                                    "Error sending new future template to standard channel: {:?}",
                                    e
                                ))
                            })?;

                        let future_job_id = standard_channel
                            .get_future_template_to_job_id()
                            .get(&template.template_id)
                            .ok_or_else(|| {
                                error!("Error getting future job id");
                                Sv2ServerEventError::MiningHandlerError(format!(
                                    "Error getting future job id for template {:?} for standard channel {:?}",
                                    template.template_id,
                                    standard_channel.get_channel_id()
                                ))
                            })?;

                        let future_job = standard_channel.get_future_jobs().get(future_job_id).ok_or_else(|| {
                            error!("Error getting future job");
                            Sv2ServerEventError::MiningHandlerError(format!(
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

                    //process extended channels
                    let extended_channels_arc = &client.extended_channels;
                    for (_, extended_channel_guard) in extended_channels_arc.read().await.iter() {
                        let mut extended_channel = extended_channel_guard.write().await;
                        extended_channel
                            .on_new_template(template.clone(), vec![coinbase_tx_output.clone()])
                            .map_err(|e| {
                                error!(
                                    "Error sending new future template to extended  channel: {:?}",
                                    e
                                );
                                Sv2ServerEventError::MiningHandlerError(format!(
                                    "Error sending new future template to extended channel: {:?}",
                                    e
                                ))
                            })?;

                        let future_job_id = extended_channel
                            .get_future_template_to_job_id()
                            .get(&template.template_id)
                            .ok_or_else(|| {
                                error!("Error getting future job id");
                                Sv2ServerEventError::MiningHandlerError(format!(
                                    "Error getting future job id for template {:?} for extended channel {:?}",
                                    template.template_id,
                                    extended_channel.get_channel_id()
                                ))
                            })?;

                        let future_job = extended_channel.get_future_jobs().get(future_job_id).ok_or_else(|| {
                            error!("Error getting future job");
                            Sv2ServerEventError::MiningHandlerError(format!(
                                "Error getting future job for template {:?} for extended channel {:?}",
                                template.template_id,
                                extended_channel.get_channel_id()
                            ))
                        })?;

                        let future_standard_job = AnyMessage::Mining(Mining::NewExtendedMiningJob(
                            future_job.get_job_message().clone(),
                        ));

                        info!("Sending future NewExtendedMiningJob message to channel {} of client {:?} for job id {:?}", extended_channel.get_channel_id(), client_id, future_job_id);
                        messages_to_client.push(future_standard_job);
                    }

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
                                Sv2ServerEventError::MiningHandlerError(format!(
                                    "Error sending new template to group channel: {:?}",
                                    e
                                ))
                            })?;

                        let active_job = group_channel.get_active_job().ok_or_else(|| {
                            error!("Error getting active job");
                            Sv2ServerEventError::MiningHandlerError(format!(
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
                                Sv2ServerEventError::MiningHandlerError(format!(
                                    "Error sending new template to standard channel: {:?}",
                                    e
                                ))
                            })?;
                        let active_job = standard_channel.get_active_job().ok_or_else(|| {
                            error!("Error getting active job");
                            Sv2ServerEventError::MiningHandlerError(format!(
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
                    //process extended channels
                    let extended_channels_arc = &client.extended_channels;
                    for (_, extended_channel_guard) in extended_channels_arc.read().await.iter() {
                        let mut extended_channel = extended_channel_guard.write().await;
                        extended_channel
                            .on_new_template(template.clone(), vec![coinbase_tx_output.clone()])
                            .map_err(|e| {
                                error!("Error sending new template to extended channel: {:?}", e);
                                Sv2ServerEventError::MiningHandlerError(format!(
                                    "Error sending new template to extended channel: {:?}",
                                    e
                                ))
                            })?;
                        let active_job = extended_channel.get_active_job().ok_or_else(|| {
                            error!("Error getting active job");
                            Sv2ServerEventError::MiningHandlerError(format!(
                                "Error getting active job for extended channel {:?}",
                                extended_channel.get_channel_id()
                            ))
                        })?;
                        let standard_job = AnyMessage::Mining(Mining::NewExtendedMiningJob(
                            active_job.get_job_message().clone(),
                        ));
                        info!("Sending non-future NewExtendedMiningJob message to channel {} of client {:?} for job id {:?}", extended_channel.get_channel_id(), client_id, active_job.get_job_id());
                        messages_to_client.push(standard_job);
                    }

                    let message_to_client = Sv2MessagesToClient {
                        client_id: *client_id,
                        messages: messages_to_client,
                    };
                    messages_to_clients.push(message_to_client);
                }
            }
        }

        Ok(Sv2ServerOutcome::TriggerNewEvent(Box::new(
            Sv2ServerEvent::SendMessagesToClients(Box::new(messages_to_clients)),
        )))
    }

    async fn on_set_new_prev_hash(
        &self,
        prev_hash: SetNewPrevHash<'static>,
    ) -> Result<Sv2ServerOutcome<'static>, Sv2ServerEventError> {
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
                Sv2ServerEventError::MiningHandlerError(format!(
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
                        Sv2ServerEventError::MiningHandlerError(format!(
                            "Error processing SetNewPrevHash on group channel: {:?}",
                            e
                        ))
                    })?;

                let group_channel_active_job = group_channel.get_active_job().ok_or_else(|| {
                    error!("Error getting active job");
                    Sv2ServerEventError::MiningHandlerError(format!(
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
                        Sv2ServerEventError::MiningHandlerError(format!(
                            "Error processing SetNewPrevHash on standard channel: {:?}",
                            e
                        ))
                    })?;

                let standard_channel_active_job =
                    standard_channel.get_active_job().ok_or_else(|| {
                        error!("Error getting active job");
                        Sv2ServerEventError::MiningHandlerError(format!(
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

            //process extended channels
            let extended_channels_arc = &client_guard.extended_channels;
            for (_, extended_channel_guard) in extended_channels_arc.read().await.iter() {
                let mut extended_channel = extended_channel_guard.write().await;

                extended_channel
                    .on_set_new_prev_hash(prev_hash.clone())
                    .map_err(|e| {
                        error!(
                            "Error processing SetNewPrevHash on extended channel: {:?}",
                            e
                        );
                        Sv2ServerEventError::MiningHandlerError(format!(
                            "Error processing SetNewPrevHash on extended channel: {:?}",
                            e
                        ))
                    })?;

                let extended_channel_active_job =
                    extended_channel.get_active_job().ok_or_else(|| {
                        error!("Error getting active job");
                        Sv2ServerEventError::MiningHandlerError(format!(
                            "Error getting active job for extended channel {:?}",
                            extended_channel.get_channel_id()
                        ))
                    })?;

                let active_job_id = extended_channel_active_job.get_job_id();

                let set_new_prev_hash_mp = SetNewPrevHashMp {
                    channel_id: extended_channel.get_channel_id(),
                    prev_hash: prev_hash.prev_hash.clone(),
                    job_id: active_job_id,
                    min_ntime: prev_hash.header_timestamp,
                    nbits: prev_hash.n_bits,
                };

                let set_new_prev_hash_mp =
                    AnyMessage::Mining(Mining::SetNewPrevHash(set_new_prev_hash_mp));

                info!(
                    "Sending SetNewPrevHash message to channel {} of client {:?} for job id {:?}",
                    extended_channel.get_channel_id(),
                    client_id,
                    active_job_id
                );
                messages_to_client.push(set_new_prev_hash_mp);
            }

            let messages_to_client = Sv2MessagesToClient {
                client_id: *client_id,
                messages: messages_to_client,
            };
            messages_to_clients.push(messages_to_client);
        }

        Ok(Sv2ServerOutcome::TriggerNewEvent(Box::new(
            Sv2ServerEvent::SendMessagesToClients(Box::new(messages_to_clients)),
        )))
    }
}
