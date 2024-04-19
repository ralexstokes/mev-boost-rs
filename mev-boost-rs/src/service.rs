use crate::relay_mux::RelayMux;
use ethereum_consensus::{
    networks::{self, Network},
    state_transition::Context,
};
use futures::StreamExt;
use mev_rs::{
    blinded_block_provider::Server as BlindedBlockProviderServer,
    relay::{parse_relay_endpoints, Relay, RelayEndpoint},
    Error,
};
use serde::{Deserialize, Serialize};
use std::{future::Future, net::Ipv4Addr, pin::Pin, task::Poll};
use tokio::task::{JoinError, JoinHandle};
use tracing::{error, info};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub host: Ipv4Addr,
    pub port: u16,
    pub relays: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self { host: Ipv4Addr::UNSPECIFIED, port: 18550, relays: vec![] }
    }
}

pub struct Service {
    host: Ipv4Addr,
    port: u16,
    relays: Vec<RelayEndpoint>,
    network: Network,
}

impl Service {
    pub fn from(network: Network, config: Config) -> Self {
        let relays = parse_relay_endpoints(&config.relays);

        Self { host: config.host, port: config.port, relays, network }
    }

    /// Spawns a new [`RelayMux`] and [`BlindedBlockProviderServer`] task
    pub fn spawn(self) -> Result<ServiceHandle, Error> {
        let Self { host, port, relays, network } = self;

        if relays.is_empty() {
            error!("no valid relays provided; please restart with correct configuration");
        } else {
            let count = relays.len();
            info!("configured with {count} relay(s)");
            for relay in &relays {
                info!(%relay, "configured with relay");
            }
        }

        let context = Context::try_from(network)?;
        let relays = relays.into_iter().map(Relay::from);
        let clock = context.clock().unwrap_or_else(|| {
            let genesis_time = networks::typical_genesis_time(&context);
            context.clock_at(genesis_time)
        });
        let relay_mux = RelayMux::new(relays, context);

        let relay_mux_clone = relay_mux.clone();
        let relay_task = tokio::spawn(async move {
            let relay_mux = relay_mux_clone;
            let slots = clock.stream_slots();

            tokio::pin!(slots);

            let mut current_epoch = clock.current_epoch().expect("after genesis");
            while let Some(slot) = slots.next().await {
                relay_mux.on_slot(slot);

                let epoch = clock.epoch_for(slot);
                if epoch != current_epoch {
                    relay_mux.on_epoch(epoch);
                    current_epoch = epoch;
                }
            }
        });

        let server = BlindedBlockProviderServer::new(host, port, relay_mux).spawn();

        Ok(ServiceHandle { relay_mux: relay_task, server })
    }
}

/// Contains the handles to spawned [`RelayMux`] and [`BlindedBlockProviderServer`] tasks
///
/// This struct is created by the [`Service::spawn`] function
#[pin_project::pin_project]
pub struct ServiceHandle {
    #[pin]
    relay_mux: JoinHandle<()>,
    #[pin]
    server: JoinHandle<()>,
}

impl Future for ServiceHandle {
    type Output = Result<(), JoinError>;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let relay_mux = this.relay_mux.poll(cx);
        if relay_mux.is_ready() {
            return relay_mux
        }
        this.server.poll(cx)
    }
}
