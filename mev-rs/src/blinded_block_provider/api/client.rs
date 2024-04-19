use crate::{
    types::{
        AuctionContents, AuctionRequest, SignedBlindedBeaconBlock, SignedBuilderBid,
        SignedValidatorRegistration,
    },
    Error,
};
use axum::http::{Method, StatusCode};
use beacon_api_client::{
    api_error_or_ok, mainnet::Client as BeaconApiClient, ApiResult, Error as ApiError,
    VersionedValue, ETH_CONSENSUS_VERSION_HEADER,
};
use std::sync::Arc;

/// A `Client` for a service implementing the Builder APIs.
/// Note that `Client` does not implement the `BlindedBlockProvider` trait so that
/// it can provide more flexibility to callers with respect to the types
/// it accepts.
#[derive(Clone)]
pub struct Client {
    api: Arc<BeaconApiClient>,
}

impl Client {
    pub fn new(api_client: Arc<BeaconApiClient>) -> Self {
        Self { api: api_client }
    }

    pub async fn check_status(&self) -> Result<(), beacon_api_client::Error> {
        let response = self.api.http_get("/eth/v1/builder/status").await?;
        api_error_or_ok(response).await
    }

    pub async fn register_validators(
        &self,
        registrations: &[SignedValidatorRegistration],
    ) -> Result<(), Error> {
        let response = self.api.http_post("/eth/v1/builder/validators", &registrations).await?;
        api_error_or_ok(response).await.map_err(From::from)
    }

    pub async fn fetch_best_bid(
        &self,
        auction_request: &AuctionRequest,
    ) -> Result<SignedBuilderBid, Error> {
        let target = format!(
            "/eth/v1/builder/header/{}/{:?}/{:?}",
            auction_request.slot, auction_request.parent_hash, auction_request.public_key
        );
        let response = self.api.http_get(&target).await?;

        if response.status() == StatusCode::NO_CONTENT {
            return Err(Error::NoBidPrepared(auction_request.clone()));
        }

        let result: ApiResult<VersionedValue<SignedBuilderBid>> =
            response.json().await.map_err(beacon_api_client::Error::Http)?;
        match result {
            ApiResult::Ok(result) => Ok(result.data),
            ApiResult::Err(err) => Err(Error::Api(err.into())),
        }
    }

    pub async fn open_bid(
        &self,
        signed_block: &SignedBlindedBeaconBlock,
    ) -> Result<AuctionContents, Error> {
        let endpoint = self
            .api
            .endpoint
            .join("/eth/v1/builder/blinded_blocks")
            .map_err(beacon_api_client::Error::Url)?;
        let response = self
            .api
            .http
            .request(Method::POST, endpoint)
            .header(ETH_CONSENSUS_VERSION_HEADER, signed_block.version().to_string())
            .json(signed_block)
            .send()
            .await
            .map_err(beacon_api_client::Error::Http)?;

        let result = response
            .json::<ApiResult<VersionedValue<AuctionContents>>>()
            .await
            .map_err(beacon_api_client::Error::Http)?;
        match result {
            ApiResult::Ok(result) => Ok(result.data),
            ApiResult::Err(err) => Err(ApiError::from(err).into()),
        }
    }
}
