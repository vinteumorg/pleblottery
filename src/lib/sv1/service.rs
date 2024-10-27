use super::config::Sv1Config;
use super::handler::Sv1Handler;
use futures::{FutureExt, StreamExt};
use sv1_api::IsServer;
use tokio::io::AsyncWriteExt;

const MAX_LINE_LENGTH: usize = 2_usize.pow(16);

pub struct Sv1Service {
    listener: tokio::net::TcpListener,
    sv1_handler: Sv1Handler,
}

impl Sv1Service {
    pub async fn new(config: Sv1Config) -> anyhow::Result<Self> {
        let listen_host = config.host;
        let listen_port = config.port;
        let listener = tokio::net::TcpListener::bind((listen_host.clone(), listen_port)).await?;

        tracing::info!(
            "listening for sv1 connections at: {}:{}",
            listen_host,
            listen_port
        );

        let sv1_handler = Sv1Handler {};

        Ok(Self {
            listener,
            sv1_handler,
        })
    }

    pub fn serve(self) -> tokio::task::JoinHandle<anyhow::Result<()>> {
        let handle = tokio::task::spawn(async move {
            while let Ok((stream, addr)) = self.listener.accept().await {
                tracing::info!("established sv1 connection: {}", addr);

                Self::handle_tcp_stream(stream, addr, self.sv1_handler.clone());
            }
            Ok(())
        });

        handle
    }

    fn handle_tcp_stream(
        stream: tokio::net::TcpStream,
        addr: std::net::SocketAddr,
        mut sv1_handler: Sv1Handler,
    ) {
        tokio::task::spawn(async move {
            let (reader, mut writer) = stream.into_split();

            let mut message_reader = tokio_util::codec::FramedRead::new(
                reader,
                tokio_util::codec::LinesCodec::new_with_max_length(MAX_LINE_LENGTH),
            );

            loop {
                match message_reader.next().fuse().await {
                    Some(r) => match r {
                        Ok(message_str) => {
                            tracing::info!(
                                "received sv1 message from: {} | {}",
                                addr,
                                message_str
                            );

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
                                "sending sv1 response to: {} | {:?}",
                                addr,
                                sv1_response_str
                            );

                            let sv1_response_str_fmt = format!("{}\n", sv1_response_str);

                            writer
                                .write_all(sv1_response_str_fmt.as_bytes())
                                .await
                                .expect("should always send response over TcpStream writer");
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
