use super::bitcoin::gbt;
use super::config::Sv1Config;
use super::handler::Sv1Handler;
use async_std::io::BufReader;
use async_std::net::TcpStream;
use async_std::prelude::*;
use async_std::stream::StreamExt;
use futures::future::FutureExt;
use std::sync::Arc;
use sv1_api::IsServer;
use tokio::sync::mpsc;
use tokio::sync::Mutex;

const MAX_LINE_LENGTH: usize = 2_usize.pow(16);

pub struct Sv1Service {
    listener: async_std::net::TcpListener,
    sv1_handler: Arc<Mutex<Sv1Handler>>,
    bitcoin_rpc_client: bitcoincore_rpc::Client,
    getblocktemplate_interval: f32,
}

impl Sv1Service {
    pub async fn new(config: Sv1Config) -> anyhow::Result<Self> {
        let listen_host = config.listen_host;
        let listen_port = config.listen_port;
        let listen_addr = format!("{}:{}", listen_host, listen_port);
        let listener = async_std::net::TcpListener::bind(listen_addr).await?;

        tracing::info!(
            "listening for sv1 connections at: {}:{}",
            listen_host,
            listen_port
        );

        let sv1_handler = Sv1Handler::new(
            config.bitcoin_network,
            config.solo_miner_signature,
            config.solo_miner_address,
        )?;

        let bitcoin_rpc_url = format!("{}:{}", config.bitcoin_rpc_host, config.bitcoin_rpc_port);
        let bitcoin_rpc_client = bitcoincore_rpc::Client::new(
            &bitcoin_rpc_url,
            bitcoincore_rpc::Auth::UserPass(config.bitcoin_rpc_user, config.bitcoin_rpc_pass),
        )?;

        Ok(Self {
            listener,
            sv1_handler: Arc::new(Mutex::new(sv1_handler)),
            bitcoin_rpc_client,
            getblocktemplate_interval: config.getblocktemplate_interval,
        })
    }

    async fn block_template_updater(
        bitcoin_rpc_client: bitcoincore_rpc::Client,
        getblocktemplate_interval: f32,
        sv1_handler: Arc<Mutex<Sv1Handler>>,
    ) {
        loop {
            std::thread::sleep(std::time::Duration::from_secs_f32(
                getblocktemplate_interval,
            ));
            tracing::debug!("sending getblocktemplate RPC to Bitcoin Node");

            let gbt_result = gbt(&bitcoin_rpc_client).await;

            let mut sv1_handler = sv1_handler.lock().await;
            sv1_handler.update_template(gbt_result);
        }
    }

    pub fn serve(self) -> tokio::task::JoinHandle<anyhow::Result<()>> {
        let sv1_handler_clone = self.sv1_handler.clone();
        let handle = tokio::task::spawn(async move {
            while let Ok((stream, addr)) = self.listener.accept().await {
                tracing::info!("established sv1 connection: {}", addr);

                Self::handle_tcp_stream(stream, addr, sv1_handler_clone.clone());
            }
            Ok(())
        });

        tokio::task::spawn(async move {
            Self::block_template_updater(
                self.bitcoin_rpc_client,
                self.getblocktemplate_interval,
                self.sv1_handler,
            )
            .await;
        });

        handle
    }

    async fn tcp_writer_task(mut tcp_writer: TcpStream, mut writer_rx: mpsc::Receiver<String>) {
        while let Some(message) = writer_rx.recv().await {
            let message_bytes = message.as_bytes();
            tcp_writer
                .write_all(message_bytes)
                .await
                .expect("should always send response over TcpStream writer");
        }
    }

    fn handle_tcp_stream(
        stream: TcpStream,
        addr: std::net::SocketAddr,
        sv1_handler: Arc<Mutex<Sv1Handler>>,
    ) {
        tokio::task::spawn(async move {
            let stream = Arc::new(stream);

            // split TcpStream
            let (tcp_reader, tcp_writer) = (stream.clone(), stream);

            // leverage channels to receive/send asynchronously
            let (writer_tx, writer_rx) = mpsc::channel::<String>(1);

            // spawn tcp_writer_task
            tokio::task::spawn(async move {
                Self::tcp_writer_task((*tcp_writer).clone(), writer_rx).await;
            });

            let sv1_handler_clone = sv1_handler.clone();

            let writer_tx_clone = writer_tx.clone();

            // tmp: send mining.set_difficulty + mining.notify
            tokio::task::spawn(async move {
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(5));

                    let sv1_handler = sv1_handler_clone.lock().await;

                    if sv1_handler.is_authorized {
                        let sv1_set_difficulty =
                            sv1_api::methods::server_to_client::SetDifficulty { value: 0.00002328342918345014 };

                        let sv1_set_difficulty_msg: sv1_api::json_rpc::Message =
                            sv1_set_difficulty.into();

                        let sv1_set_difficulty_msg_str =
                            serde_json::to_string(&sv1_set_difficulty_msg)
                                .expect("should always work");

                        tracing::info!(
                            "sending sv1 mining.set_difficulty to: {} | {}",
                            addr,
                            sv1_set_difficulty_msg_str
                        );

                        let sv1_set_difficulty_msg_str_fmt =
                            format!("{}\n", sv1_set_difficulty_msg_str);
                        writer_tx_clone
                            .send(sv1_set_difficulty_msg_str_fmt)
                            .await
                            .expect("should always work");

                        let sv1_notify = sv1_api::methods::server_to_client::Notify {
                            job_id: "0".to_string(),
                            prev_hash: sv1_handler.template.prevhash.clone(),
                            coin_base1: sv1_handler.template.coinbase_prefix.clone(),
                            coin_base2: sv1_handler.template.coinbase_suffix.clone(),
                            merkle_branch: sv1_handler.template.merkle_branches.clone(),
                            version: sv1_handler.template.version.clone(),
                            bits: sv1_handler.template.bits.clone(),
                            time: sv1_handler.template.time.clone(),
                            clean_jobs: false,
                        };

                        let sv1_notify_msg: sv1_api::json_rpc::Message = sv1_notify.into();

                        let sv1_notify_msg_str = serde_json::to_string(&sv1_notify_msg)
                            .expect("should always convert to string");

                        tracing::info!(
                            "sending sv1 mining.notify to: {} | {}",
                            addr,
                            sv1_notify_msg_str
                        );

                        let sv1_notify_msg_str_fmt = format!("{}\n", sv1_notify_msg_str);
                        writer_tx_clone
                            .send(sv1_notify_msg_str_fmt)
                            .await
                            .expect("should always work");
                    };
                }
            });

            let buf_reader = BufReader::new(&*tcp_reader);
            let mut message_reader = tokio_util::codec::FramedRead::new(
                async_compat::Compat::new(buf_reader),
                tokio_util::codec::LinesCodec::new_with_max_length(MAX_LINE_LENGTH),
            );

            loop {
                match message_reader.next().fuse().await {
                    Some(r) => match r {
                        Ok(message_str) => {
                            tracing::info!("received sv1 message from: {} | {}", addr, message_str);

                            let mut sv1_handler = sv1_handler.lock().await;

                            let sv1_message: sv1_api::json_rpc::Message =
                                serde_json::from_str(&message_str)
                                    .expect("failed to parse JSON from sv1 message string");

                            let sv1_response: sv1_api::json_rpc::Message = sv1_handler
                                .handle_message(sv1_message)
                                .expect("error handling sv1 message")
                                .expect("should always be Some")
                                .try_into()
                                .expect("should always convert to Message");

                            let sv1_response_str = serde_json::to_string(&sv1_response)
                                .expect("should always convert to string");

                            tracing::info!(
                                "sending sv1 response to: {} | {}",
                                addr,
                                sv1_response_str
                            );

                            let sv1_response_str_fmt = format!("{}\n", sv1_response_str);

                            writer_tx
                                .send(sv1_response_str_fmt)
                                .await
                                .expect("should always work");
                        }
                        Err(e) => {
                            panic!("error reading TcpStream: {}", e);
                        }
                    },
                    None => {
                        tracing::info!("closed sv1 connection: {}", addr);
                        break;
                    }
                }
            }
        });
    }
}
