use crate::relay::Relay;
use beacon_api_client::Client;
use ethereum_consensus::state_transition::Context;
use futures::StreamExt;
use mev_rs::{blinded_block_provider::Server as BlindedBlockProviderServer, Error, Network};
use serde::Deserialize;
use std::{future::Future, net::Ipv4Addr, pin::Pin, sync::Arc, task::Poll};
use tokio::task::{JoinError, JoinHandle};
use url::Url;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub host: Ipv4Addr,
    pub port: u16,
    pub beacon_node_url: String,
    #[serde(default)]
    pub network: Network,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: Ipv4Addr::LOCALHOST,
            port: 28545,
            beacon_node_url: "http://127.0.0.1:5052".into(),
            network: Default::default(),
        }
    }
}

pub struct Service {
    host: Ipv4Addr,
    port: u16,
    beacon_node: Client,
    network: Network,
}

impl Service {
    pub fn from(config: Config) -> Self {
        let endpoint: Url = config.beacon_node_url.parse().unwrap();
        let beacon_node = Client::new(endpoint);
        Self { host: config.host, port: config.port, beacon_node, network: config.network }
    }

    /// Configures the [`Relay`] and the [`BlindedBlockProviderServer`] and spawns both to
    /// individual tasks
    pub async fn spawn(&self, context: Option<Context>) -> Result<ServiceHandle, Error> {
        let network = &self.network;
        let context =
            if let Some(context) = context { context } else { Context::try_from(network)? };
        let clock = context.clock(None);
        let context = Arc::new(context);
        let relay = Relay::new(self.beacon_node.clone(), context);
        relay.initialize().await;

        let block_provider = relay.clone();
        let server = BlindedBlockProviderServer::new(self.host, self.port, block_provider).spawn();

        let relay = tokio::spawn(async move {
            let slots = clock.stream_slots();

            tokio::pin!(slots);

            let mut current_epoch = clock.current_epoch();
            let mut next_epoch = false;
            while let Some(slot) = slots.next().await {
                let epoch = clock.epoch_for(slot);
                if epoch > current_epoch {
                    current_epoch = epoch;
                    next_epoch = true;
                }
                relay.on_slot(slot, next_epoch).await;
            }
        });

        Ok(ServiceHandle { relay, server })
    }
}

/// Contains the handles to spawned [`Relay`] and [`BlindedBlockProviderServer`] tasks
///
/// This struct is created by the [`Service::spawn`] function
#[pin_project::pin_project]
pub struct ServiceHandle {
    #[pin]
    relay: JoinHandle<()>,
    #[pin]
    server: JoinHandle<()>,
}

impl Future for ServiceHandle {
    type Output = Result<(), JoinError>;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let relay = this.relay.poll(cx);
        if relay.is_ready() {
            return relay;
        }
        this.server.poll(cx)
    }
}
