use tonic::{Request, Response, Status};
use tonic_reflection::pb::v1::server_reflection_server::{
    ServerReflection, ServerReflectionServer
};

use std::sync::Arc;

use crate::tw_main::{AppState, BuyRequest as InnerBuyRequest, SellRequest as InnerSellRequest,
    buy_impl, sell_impl, ordered_locks_buy, ordered_locks_sell};

pub mod twin_proto {
    // Include generated Rust structs/traits from compiled .proto at build time
    tonic::include_proto!("twin");

    // Embed the raw compiled descriptor bytes (built by prost-build/tonic-build 
    // with descriptor output enabled) into the binary as a static byte array.
    //
    // Compiled binary blob (from .proto file) describing gRPC service's schema
    // (messages, methods)
    pub const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("twin_descriptor");
}

use twin_proto::{
    twin_server::{Twin, TwinServer},
    BuyRequest, BuyResponse, SellRequest, SellResponse,
    AllocationRequest, AllocationResponse,
};

pub struct TwinGrpcService {
    pub state: Arc<AppState>,
}

#[tonic::async_trait]
impl Twin for TwinGrpcService {

    /// Note: BuyRequest auto generated from proto file
    async fn buy(&self, req: Request<BuyRequest>) -> Result<Response<BuyResponse>, Status> {
        let buy_req = req.into_inner();
        let mut bids = ordered_locks_buy(&self.state);
        buy_impl(
            &self.state.buy_seq_no,
            &self.state.supply,
            &self.state.allocations,
            &mut bids,
            InnerBuyRequest::new(buy_req.user, buy_req.volume, buy_req.price),
        );
        Ok(Response::new(BuyResponse {}))
    }

    async fn sell(&self, req: Request<SellRequest>) -> Result<Response<SellResponse>, Status> {
        if req.get_ref().volume == 0 {
            return Err(Status::invalid_argument("volume must be greater than 0"));
        }

        let sell_req = req.into_inner();
        let mut bids = ordered_locks_sell(&self.state);
        sell_impl(
            &self.state.supply,
            &self.state.allocations,
            &mut bids,
            InnerSellRequest { volume: sell_req.volume },
        );
        Ok(Response::new(SellResponse {}))
    }

    async fn get_allocation(
        &self, req: Request<AllocationRequest>
    ) -> Result<Response<AllocationResponse>, Status> {
        let req = req.into_inner();
        let volume = 
            self.state.allocations.get(&req.username).map(|v| *v).unwrap_or(0);
        Ok(Response::new(AllocationResponse { volume }))
    }
}

pub fn make_grpc_server(state: Arc<AppState>) -> TwinServer<TwinGrpcService> {
    TwinServer::new(TwinGrpcService { state })
}

/// Feeds raw compiled descriptor bytes into the reflection server so tools 
/// like grpcurl or Postman can discover service's methods/messages
/// dynamically.
/// e.g. grpcurl -plaintext localhost:50051 list
pub fn make_reflection_service() -> ServerReflectionServer<impl ServerReflection> {
    tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(twin_proto::FILE_DESCRIPTOR_SET)
        .build_v1()
        .expect("-- gRPC: Panicking, reflection-service init failure")
}