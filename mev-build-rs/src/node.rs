//! Customized types for the builder to configuring reth

use crate::payload::{
    attributes::BuilderPayloadBuilderAttributes, service_builder::PayloadServiceBuilder,
};
use reth::{
    api::{
        validate_version_specific_fields, EngineApiMessageVersion, EngineObjectValidationError,
        EngineTypes, FullNodeTypes, PayloadOrAttributes,
    },
    builder::{components::ComponentsBuilder, NodeTypes},
    payload::EthBuiltPayload,
    primitives::ChainSpec,
    rpc::types::{
        engine::{
            ExecutionPayloadEnvelopeV2, ExecutionPayloadEnvelopeV3, ExecutionPayloadEnvelopeV4,
            PayloadAttributes as EthPayloadAttributes,
        },
        ExecutionPayloadV1,
    },
};
use reth_node_ethereum::node::{
    EthereumExecutorBuilder, EthereumNetworkBuilder, EthereumPoolBuilder,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct BuilderNode;

impl BuilderNode {
    /// Returns a [ComponentsBuilder] configured for a regular Ethereum node.
    pub fn components_with<Node>(
        payload_service_builder: PayloadServiceBuilder,
    ) -> ComponentsBuilder<
        Node,
        EthereumPoolBuilder,
        PayloadServiceBuilder,
        EthereumNetworkBuilder,
        EthereumExecutorBuilder,
    >
    where
        Node: FullNodeTypes<Engine = BuilderEngineTypes>,
    {
        ComponentsBuilder::default()
            .node_types::<Node>()
            .pool(EthereumPoolBuilder::default())
            .payload(payload_service_builder)
            .network(EthereumNetworkBuilder::default())
            .executor(EthereumExecutorBuilder::default())
    }
}

impl NodeTypes for BuilderNode {
    type Primitives = ();
    type Engine = BuilderEngineTypes;
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct BuilderEngineTypes;

impl EngineTypes for BuilderEngineTypes {
    type PayloadAttributes = EthPayloadAttributes;
    type PayloadBuilderAttributes = BuilderPayloadBuilderAttributes;
    type BuiltPayload = EthBuiltPayload;
    type ExecutionPayloadV1 = ExecutionPayloadV1;
    type ExecutionPayloadV2 = ExecutionPayloadEnvelopeV2;
    type ExecutionPayloadV3 = ExecutionPayloadEnvelopeV3;
    type ExecutionPayloadV4 = ExecutionPayloadEnvelopeV4;

    fn validate_version_specific_fields(
        chain_spec: &ChainSpec,
        version: EngineApiMessageVersion,
        payload_or_attrs: PayloadOrAttributes<'_, Self::PayloadAttributes>,
    ) -> Result<(), EngineObjectValidationError> {
        validate_version_specific_fields(chain_spec, version, payload_or_attrs)
    }
}
