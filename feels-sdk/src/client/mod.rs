pub mod base;
pub mod liquidity;
pub mod market;
pub mod pomm;
pub mod position;
pub mod protocol;
pub mod registry;
pub mod swap;

use std::sync::Arc;

use crate::prelude::*;
// Removed heavy solana_client dependency - using ureq for RPC calls
// pub struct RpcClient will be implemented with ureq

use crate::{
    core::{program_id, SdkResult},
    protocol::PdaBuilder,
};

pub use base::BaseClient;
pub use liquidity::LiquidityService;
pub use market::MarketService;
pub use pomm::PommService;
pub use position::PositionService;
pub use protocol::ProtocolService;
pub use registry::RegistryService;
pub use swap::SwapService;

/// Main Feels Protocol client with service-based architecture
pub struct FeelsClient {
    /// Base RPC client
    pub base: Arc<BaseClient>,
    /// Market operations service
    pub market: MarketService,
    /// Swap execution service
    pub swap: SwapService,
    /// Liquidity management service
    pub liquidity: LiquidityService,
    /// Protocol management service
    pub protocol: ProtocolService,
    /// Position management service (with NFT support)
    pub position: PositionService,
    /// Pool registry service
    pub registry: RegistryService,
    /// Protocol-Owned Market Making service
    pub pomm: PommService,
    /// PDA builder
    pub pda: Arc<PdaBuilder>,
}

impl FeelsClient {
    /// Create a new client with default configuration
    pub async fn new(rpc_url: &str) -> SdkResult<Self> {
        let rpc = Arc::new(RpcClient::new(rpc_url.to_string()));
        let base = Arc::new(BaseClient::new(rpc));
        let program_id = program_id();
        let pda = Arc::new(PdaBuilder::new(program_id));

        Ok(Self {
            market: MarketService::new(base.clone(), pda.clone()),
            swap: SwapService::new(base.clone(), pda.clone(), program_id),
            liquidity: LiquidityService::new(base.clone(), pda.clone(), program_id),
            protocol: ProtocolService::new(base.clone(), pda.clone(), program_id),
            position: PositionService::new(base.clone(), pda.clone(), program_id),
            registry: RegistryService::new(base.clone(), pda.clone(), program_id),
            pomm: PommService::new(base.clone(), pda.clone(), program_id),
            base,
            pda,
        })
    }

    /// Create a new client with custom program ID
    pub async fn with_program_id(rpc_url: &str, program_id: Pubkey) -> SdkResult<Self> {
        let rpc = Arc::new(RpcClient::new(rpc_url.to_string()));
        let base = Arc::new(BaseClient::new(rpc));
        let pda = Arc::new(PdaBuilder::new(program_id));

        Ok(Self {
            market: MarketService::new(base.clone(), pda.clone()),
            swap: SwapService::new(base.clone(), pda.clone(), program_id),
            liquidity: LiquidityService::new(base.clone(), pda.clone(), program_id),
            protocol: ProtocolService::new(base.clone(), pda.clone(), program_id),
            position: PositionService::new(base.clone(), pda.clone(), program_id),
            registry: RegistryService::new(base.clone(), pda.clone(), program_id),
            pomm: PommService::new(base.clone(), pda.clone(), program_id),
            base,
            pda,
        })
    }

    /// Get the program ID
    pub fn program_id(&self) -> Pubkey {
        self.base.program_id()
    }

    /// Get the RPC endpoint
    pub fn rpc_url(&self) -> String {
        self.base.rpc_url()
    }
}
