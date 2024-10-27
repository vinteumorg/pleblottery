use std::net::SocketAddr;
use futures::{StreamExt, FutureExt};
use super::config::Sv1Config;

const MAX_LINE_LENGTH: usize = 2_usize.pow(16);

pub struct Sv1Service {
    listener: tokio::net::TcpListener,
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

        Ok(Self { listener })
    }

    pub fn serve(self) -> tokio::task::JoinHandle<anyhow::Result<()>> {
        let handle = tokio::task::spawn(async move {
            while let Ok((stream, addr)) = self.listener.accept().await {
                tracing::info!("established sv1 connection: {}", addr);

                Self::handle_tcp_stream(stream, addr);
            }
            Ok(())
        });

        handle
    }

    fn handle_tcp_stream(mut stream: tokio::net::TcpStream, addr: std::net::SocketAddr) {
        tokio::task::spawn(async move {
            let (mut reader, mut writer) = stream.into_split();

            let mut message_reader = tokio_util::codec::FramedRead::new(
                reader,
                tokio_util::codec::LinesCodec::new_with_max_length(MAX_LINE_LENGTH),
            );

            loop {
                match message_reader.next().fuse().await {
                    Some(r) => {
                        match r {
                            Ok(message_str) => {
                                let sv1_message: sv1_api::json_rpc::Message = serde_json::from_str(&message_str).expect("failed to parse JSON from sv1 message string");
                                tracing::info!("received sv1 message from: {} | {:?}", addr, sv1_message);
                            },
                            Err(e) => {
                                panic!("error reading TcpStream: {}", e);
                            }
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
