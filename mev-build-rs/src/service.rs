use crate::mempool_builder::Builder;
use beacon_api_client::Client;
use ethereum_consensus::{crypto::SecretKey, state_transition::Context};
use futures::StreamExt;
use mev_rs::{
    blinded_block_provider::Server as BlindedBlockProviderServer,
    engine_api_proxy::{
        client::Client as EngineApiClient,
        server::{
            Client as HttpClient, Config as EngineApiProxyConfig, Proxy, Server as EngineApiProxy,
        },
    },
    transaction_injector::{Config as TransactionInjectorConfig, Injector as TransactionInjector},
    Error, Network,
};
use serde::Deserialize;
use std::{fmt, future::Future, net::Ipv4Addr, pin::Pin, sync::Arc, task::Poll};
use tokio::{
    sync::mpsc,
    task::{JoinError, JoinHandle},
};
use url::Url;

const BUILD_JOB_BUFFER_SIZE: usize = 1;

#[derive(Deserialize)]
pub struct Config {
    pub host: Ipv4Addr,
    pub port: u16,
    pub beacon_api_endpoint: String,
    #[serde(default)]
    pub network: Network,
    pub engine_api_proxy: EngineApiProxyConfig,
    pub secret_key: SecretKey,
    #[serde(default)]
    pub transaction_injector: Option<TransactionInjectorConfig>,
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("beacon_api_endpoint", &self.beacon_api_endpoint)
            .field("network", &self.network)
            .field("engine_api_proxy", &self.engine_api_proxy)
            .field("secret_key", &"...")
            .field("transaction_injector", &self.transaction_injector)
            .finish()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: Ipv4Addr::UNSPECIFIED,
            port: 28545,
            beacon_api_endpoint: String::new(),
            network: Default::default(),
            engine_api_proxy: Default::default(),
            secret_key: SecretKey::default(),
            transaction_injector: None,
        }
    }
}

pub struct Service {
    config: Config,
}

impl Service {
    pub fn from(config: Config) -> Self {
        Self { config }
    }

    pub async fn spawn(self, context: Option<Context>) -> Result<ServiceHandle, Error> {
        let Config {
            host,
            port,
            beacon_api_endpoint,
            network,
            engine_api_proxy,
            secret_key,
            transaction_injector,
        } = self.config;

        let beacon_api_endpoint: Url = beacon_api_endpoint.parse().unwrap();
        let client = Client::new(beacon_api_endpoint);

        let context =
            if let Some(context) = context { context } else { Context::try_from(&network)? };
        let (tx, rx) = mpsc::channel(BUILD_JOB_BUFFER_SIZE);
        let engine_api_client = EngineApiClient::new(&engine_api_proxy.engine_api_endpoint);
        let http_client = HttpClient::new();
        let proxy = Arc::new(Proxy::new(http_client, &engine_api_proxy.engine_api_endpoint, tx));
        let engine_api_proxy = EngineApiProxy::new(engine_api_proxy);

        let genesis_details = client.get_genesis_details().await?;
        let genesis_validators_root = genesis_details.genesis_validators_root;
        let clock = context.clock(Some(genesis_details.genesis_time));
        let builder = Builder::new(
            secret_key,
            genesis_validators_root,
            client,
            context,
            engine_api_client,
            proxy.clone(),
        );

        let block_provider = builder.clone();
        let engine_builder = builder.clone();

        let current_epoch = clock.current_epoch();
        builder.initialize(current_epoch).await;

        let mut transaction_injector =
            transaction_injector.and_then(|config| match TransactionInjector::new(config) {
                Ok(injector) => Some(injector),
                Err(err) => {
                    tracing::warn!("could not create transaction injector: {err}");
                    None
                }
            });
        let clock = tokio::spawn(async move {
            let slots = clock.stream_slots();

            tokio::pin!(slots);

            while let Some(slot) = slots.next().await {
                builder.on_slot(slot).await;
                if let Some(transaction_injector) = &mut transaction_injector {
                    match transaction_injector.submit_transaction().await {
                        Ok(txn_hash) => tracing::info!("injected transaction with hash {txn_hash}"),
                        Err(err) => tracing::warn!("error injecting transaction: {err}"),
                    }
                }
            }
        });

        let api_server = BlindedBlockProviderServer::new(host, port, block_provider).spawn();
        let engine_api_proxy = engine_api_proxy.spawn(proxy);
        let builder = engine_builder.spawn(rx);

        Ok(ServiceHandle { clock, api_server, engine_api_proxy, builder })
    }
}

/// Contains the handles to spawned [`Builder`] and [`BlindedBlockProviderServer`] tasks
///
/// This struct is created by the [`Service::spawn`] function
#[pin_project::pin_project]
pub struct ServiceHandle {
    #[pin]
    clock: JoinHandle<()>,
    #[pin]
    api_server: JoinHandle<()>,
    #[pin]
    engine_api_proxy: JoinHandle<()>,
    #[pin]
    builder: JoinHandle<()>,
}

impl Future for ServiceHandle {
    type Output = Result<(), JoinError>;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let clock = this.clock.poll(cx);
        if clock.is_ready() {
            return clock
        }
        let api_server = this.api_server.poll(cx);
        if api_server.is_ready() {
            return api_server
        }
        let engine_api_proxy = this.engine_api_proxy.poll(cx);
        if engine_api_proxy.is_ready() {
            return engine_api_proxy
        }
        this.builder.poll(cx)
    }
}
