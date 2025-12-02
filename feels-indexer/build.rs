use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Always generate geyser stub types for both real and mock modes
    // These types are compatible with Yellowstone gRPC protobuf protocol
    // We use manual types to avoid yellowstone-grpc-proto dependency conflicts
    let generated_content = r#"
// This is a stub for geyser client functionality
// In production, you would use actual Solana gRPC client

// Geyser proto types are defined in geyser_stub module

// Stub implementations to avoid protoc dependency
pub mod geyser_stub {
    use tonic::{Request, Response, Status};
    use futures::stream::Stream;
    use std::pin::Pin;

    #[derive(Clone, Debug)]
    pub struct SubscribeRequest {
        pub accounts: std::collections::HashMap<String, SubscribeRequestFilterAccounts>,
        pub transactions: std::collections::HashMap<String, SubscribeRequestFilterTransactions>,
        pub slots: std::collections::HashMap<String, SubscribeRequestFilterSlots>,
        pub blocks: std::collections::HashMap<String, SubscribeRequestFilterBlocks>,
        pub commitment: Option<CommitmentLevel>,
    }

    #[derive(Clone, Debug)]
    pub struct SubscribeRequestFilterAccounts {
        pub account: Vec<String>,
        pub owner: Vec<String>,
    }

    #[derive(Clone, Debug)]
    pub struct SubscribeRequestFilterTransactions {
        pub vote: Option<bool>,
        pub failed: Option<bool>,
        pub signature: Vec<String>,
        pub account_include: Vec<String>,
        pub account_exclude: Vec<String>,
        pub account_required: Vec<String>,
    }

    #[derive(Clone, Debug)]
    pub struct SubscribeRequestFilterSlots {
        pub filter_by_commitment: Option<bool>,
    }

    #[derive(Clone, Debug)]
    pub struct SubscribeRequestFilterBlocks {
        pub account_include: Vec<String>,
        pub include_transactions: Option<bool>,
        pub include_accounts: Option<bool>,
        pub include_entries: Option<bool>,
    }

    #[derive(Clone, Debug)]
    pub struct SubscribeUpdate {
        pub update_oneof: Option<UpdateOneof>,
    }

    #[derive(Clone, Debug)]
    pub enum UpdateOneof {
        Account(SubscribeUpdateAccount),
        Slot(SubscribeUpdateSlot),
        Transaction(SubscribeUpdateTransaction),
    }

    #[derive(Clone, Debug)]
    pub struct SubscribeUpdateAccount {
        pub account: Option<SubscribeUpdateAccountInfo>,
        pub slot: u64,
        pub is_startup: bool,
    }

    #[derive(Clone, Debug)]
    pub struct SubscribeUpdateAccountInfo {
        pub pubkey: Vec<u8>,
        pub lamports: u64,
        pub owner: Vec<u8>,
        pub executable: bool,
        pub rent_epoch: u64,
        pub data: Vec<u8>,
        pub write_version: u64,
        pub txn_signature: Option<Vec<u8>>,
    }

    #[derive(Clone, Debug)]
    pub struct SubscribeUpdateSlot {
        pub slot: u64,
        pub parent: Option<u64>,
        pub status: SlotStatus,
    }

    #[derive(Clone, Debug)]
    pub struct SubscribeUpdateTransaction {
        pub transaction: Option<SubscribeUpdateTransactionInfo>,
        pub slot: u64,
    }

    #[derive(Clone, Debug)]
    pub struct SubscribeUpdateTransactionInfo {
        pub signature: Vec<u8>,
        pub is_vote: bool,
        pub index: u64,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum CommitmentLevel {
        Processed = 0,
        Confirmed = 1,
        Finalized = 2,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum SlotStatus {
        ProcessedSlot = 0,
        ConfirmedSlot = 1,
        FinalizedSlot = 2,
    }

    #[tonic::async_trait]
    pub trait Geyser {
        type SubscribeStream: Stream<Item = Result<SubscribeUpdate, Status>> + Send + 'static;
        
        async fn subscribe(
            &self,
            request: Request<SubscribeRequest>,
        ) -> Result<Response<Self::SubscribeStream>, Status>;
    }

    pub struct GeyserClient<T> {
        inner: tonic::client::Grpc<T>,
    }

    impl<T> GeyserClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::BoxBody>,
        T::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
        T::ResponseBody: tonic::codec::Decoder + Default + Send + 'static,
        <T::ResponseBody as http_body::Body>::Error: Into<Box<dyn std::error::Error + Send + Sync>> + Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }

        pub async fn subscribe(
            &mut self,
            request: impl tonic::IntoRequest<SubscribeRequest>,
        ) -> Result<
            tonic::Response<
                tonic::codec::Streaming<SubscribeUpdate>
            >,
            tonic::Status,
        > {
            // Stub implementation
            unimplemented!("Geyser client is a stub")
        }
    }
}
"#;

    // Write the generated file
    let gen_path = out_dir.join("geyser_stub.rs");
    std::fs::write(&gen_path, generated_content)?;

    // Create empty proto file to satisfy cargo rerun-if-changed
    let proto_path = out_dir.join("geyser.proto");
    std::fs::write(&proto_path, "")?;

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", proto_path.display());
    println!("cargo:rerun-if-changed={}", out_dir.display());

    Ok(())
}