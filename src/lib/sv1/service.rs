use super::config::Sv1Config;

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
            }
            Ok(())
        });

        handle
    }
}
