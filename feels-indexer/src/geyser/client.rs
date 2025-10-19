use anyhow::Result;
use solana_sdk::pubkey::Pubkey;
use tokio_stream::StreamExt;
use tonic::transport::Channel;
use tracing::info;

// TODO: Fix tonic Body trait issue in generated code
// Include the generated stub code (no protoc needed)
// include!(concat!(env!("OUT_DIR"), "/geyser_stub.rs"));

// use geyser_stub::{
//     GeyserClient, CommitmentLevel, SubscribeRequest, SubscribeRequestFilterAccounts,
//     SubscribeUpdate,
// };

// Temporary placeholder types
type SubscribeUpdate = ();
type CommitmentLevel = ();

pub struct FeelsGeyserClient {
    // client: GeyserClient<Channel>,
    _channel: Channel, // Keep for future use
    program_id: Pubkey,
}

impl FeelsGeyserClient {
    pub async fn connect(endpoint: &str, program_id: Pubkey) -> Result<Self> {
        info!("Connecting to Geyser endpoint: {}", endpoint);
        
        let channel = Channel::from_shared(endpoint.to_string())?
            .connect()
            .await?;
            
        // let client = GeyserClient::new(channel);
        
        Ok(Self { _channel: channel, program_id })
    }

    pub async fn subscribe_to_program_accounts(&mut self) -> Result<impl StreamExt<Item = Result<SubscribeUpdate, tonic::Status>>> {
        // TODO: Fix tonic Body trait bounds issue
        // For now, return an empty stream
        use futures::stream;
        Ok(stream::empty())
        /*
        let mut accounts_filter = HashMap::new();
        
        // Subscribe to all accounts owned by the Feels program
        accounts_filter.insert(
            "feels_accounts".to_string(),
            SubscribeRequestFilterAccounts {
                account: vec![],
                owner: vec![self.program_id.to_string()],
            },
        );

        let request = SubscribeRequest {
            accounts: accounts_filter,
            transactions: HashMap::new(),
            slots: HashMap::new(),
            blocks: HashMap::new(),
            commitment: Some(CommitmentLevel::Confirmed),
        };

        debug!("Sending subscription request for program: {}", self.program_id);
        
        let response = self.client.subscribe(request).await?;
        let stream = response.into_inner();
        
        info!("Successfully subscribed to Geyser stream");
        Ok(stream)
        */
    }

    pub async fn subscribe_to_specific_accounts(&mut self, _accounts: Vec<Pubkey>) -> Result<impl StreamExt<Item = Result<SubscribeUpdate, tonic::Status>>> {
        // TODO: Fix tonic Body trait bounds issue
        // For now, return an empty stream
        use futures::stream;
        Ok(stream::empty())
        /*
        let mut accounts_filter = HashMap::new();
        
        let account_strings: Vec<String> = accounts.iter().map(|pk| pk.to_string()).collect();
        
        accounts_filter.insert(
            "specific_accounts".to_string(),
            SubscribeRequestFilterAccounts {
                account: account_strings,
                owner: vec![],
            },
        );

        let request = SubscribeRequest {
            accounts: accounts_filter,
            transactions: HashMap::new(),
            slots: HashMap::new(),
            blocks: HashMap::new(),
            commitment: Some(CommitmentLevel::Confirmed),
        };

        debug!("Subscribing to {} specific accounts", accounts.len());
        
        let response = self.client.subscribe(request).await?;
        let stream = response.into_inner();
        
        info!("Successfully subscribed to specific accounts");
        Ok(stream)
        */
    }
}

// Helper functions for working with the protobuf types
pub mod helpers {
    // use super::geyser_stub::*;
    use solana_sdk::pubkey::Pubkey;

    pub fn pubkey_from_bytes(bytes: &[u8]) -> Result<Pubkey, Box<dyn std::error::Error>> {
        if bytes.len() != 32 {
            return Err(format!("Invalid pubkey length: {}", bytes.len()).into());
        }
        let mut array = [0u8; 32];
        array.copy_from_slice(bytes);
        Ok(Pubkey::from(array))
    }

    // TODO: Re-enable these helper functions when geyser types are available
    /*
    pub fn is_feels_account_update(update: &SubscribeUpdateAccount, program_id: &Pubkey) -> bool {
        if let Some(account_info) = &update.account {
            if let Ok(owner) = pubkey_from_bytes(&account_info.owner) {
                return &owner == program_id;
            }
        }
        false
    }

    pub fn extract_account_data(update: &SubscribeUpdateAccount) -> Option<&[u8]> {
        update.account.as_ref().map(|info| info.data.as_slice())
    }

    pub fn extract_account_pubkey(update: &SubscribeUpdateAccount) -> Option<Pubkey> {
        update.account.as_ref()
            .and_then(|info| pubkey_from_bytes(&info.pubkey).ok())
    }
    */
    
    pub fn transaction_involves_program(_transaction_data: &[u8], _program_id: &Pubkey) -> bool {
        // This is a simplified check - in production would parse the transaction
        // and check if any instruction involves the program
        true // For now, process all transactions
    }
}
