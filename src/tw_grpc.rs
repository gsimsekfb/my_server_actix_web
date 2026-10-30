use tonic::{Request, Response, Status};
use tonic_reflection::pb::v1::server_reflection_server::{
    ServerReflection, ServerReflectionServer
};

use std::sync::Arc;

use crate::tw_main::{
    ActiveStorage,
    AppState, BuyRequest as TwnBuyRequest, SellRequest as TwnSellRequest,
    buy_impl, sell_impl
};

use crate::tw_matching_engine_trait::TwnEngine;

#[allow(clippy::result_large_err)]
pub mod twn_proto {
    // Include generated Rust structs/traits from compiled .proto at build time
    tonic::include_proto!("twn");

    // Embed the raw compiled descriptor bytes (built by prost-build/tonic-build 
    // with descriptor output enabled) into the binary as a static byte array.
    //
    // Compiled binary blob (from .proto file) describing gRPC service's schema
    // (messages, methods)
    pub const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("twn_descriptor");
}

use twn_proto::{
    twn_server::{Twn, TwnServer},
    BuyRequest, BuyResponse, SellRequest, SellResponse,
    AllocationRequest, AllocationResponse,
};

pub struct TwnGrpcService {
    pub state: Arc<AppState>,
}

#[tonic::async_trait]
impl Twn for TwnGrpcService {

    /// Note: BuyRequest auto generated from proto file
    async fn buy(&self, req: Request<BuyRequest>) -> Result<Response<BuyResponse>, Status> {
        let buy_req = req.into_inner();
        let req = TwnBuyRequest::new(buy_req.user, buy_req.volume, buy_req.price);
        self.state.storage.buy(req).await;

        Ok(Response::new(BuyResponse {}))
    }

    async fn sell(&self, req: Request<SellRequest>) -> Result<Response<SellResponse>, Status> {
        if req.get_ref().volume == 0 {
            return Err(Status::invalid_argument("volume must be greater than 0"));
        }
        let req = TwnSellRequest { volume:  req.into_inner().volume };
        self.state.storage.sell(req).await;

        Ok(Response::new(SellResponse {}))
    }

    async fn get_allocation(
        &self, req: Request<AllocationRequest>
    ) -> Result<Response<AllocationResponse>, Status> {
        let req = req.into_inner();
        let volume = self.state.storage.allocation(&req.username).await
            .unwrap_or_default();

        Ok(Response::new(AllocationResponse { volume }))
    }
}

pub fn make_grpc_server(state: Arc<AppState>) -> TwnServer<TwnGrpcService> {
    TwnServer::new(TwnGrpcService { state })
}

/// Feeds raw compiled descriptor bytes into the reflection server so tools 
/// like grpcurl or Postman can discover service's methods/messages
/// dynamically.
/// e.g. grpcurl -plaintext localhost:50051 list
pub fn make_reflection_service() -> ServerReflectionServer<impl ServerReflection> {
    tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(twn_proto::FILE_DESCRIPTOR_SET)
        .build_v1()
        .expect("-- gRPC: Panicking, reflection-service init failure")
}