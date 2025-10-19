use anyhow::Result;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

// Re-export for convenience
pub use solana_sdk::commitment_config::CommitmentLevel;

// Define our own types to avoid direct yellowstone dependency in other modules
#[derive(Debug, Clone)]
pub struct AccountUpdate {
    pub pubkey: Pubkey,
    pub slot: u64,
    pub program: String,
}

#[derive(Debug, Clone)]
pub struct TransactionUpdate {
    pub signature: String,
    pub slot: u64,
}

#[derive(Debug, Clone)]
pub struct SlotUpdate {
    pub slot: u64,
    pub parent: Option<u64>,
    pub status: CommitmentLevel,
}

#[derive(Debug, Clone)]
pub enum StreamingUpdate {
    Account(AccountUpdate),
    Transaction(TransactionUpdate),
    Slot(SlotUpdate),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StreamUpdate {
    pub slot: u64,
    pub update_type: String,
    pub data: serde_json::Value,
}

pub struct StreamingClient {
    endpoint: String,
    program_id: Option<Pubkey>,
}

impl StreamingClient {
    pub fn new(endpoint: String) -> Self {
        Self {
            endpoint,
            program_id: None,
        }
    }

    pub fn with_program_filter(mut self, program_id: Pubkey) -> Self {
        self.program_id = Some(program_id);
        self
    }

    pub async fn connect_and_stream(
        self,
        tx: mpsc::Sender<StreamingUpdate>,
    ) -> Result<()> {
        info!("Connecting to streaming endpoint: {}", self.endpoint);

        // For SSE endpoint
        let stream_url = format!("{}/stream", self.endpoint.trim_end_matches('/'));
        
        let client = Client::new();
        let response = client
            .get(&stream_url)
            .header("Accept", "text/event-stream")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to connect to stream: {}",
                response.status()
            ));
        }

        info!("Connected to streaming endpoint, processing events...");

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    buffer.push_str(&text);

                    // Process complete SSE events
                    while let Some(pos) = buffer.find("\n\n") {
                        let event = buffer.drain(..pos + 2).collect::<String>();
                        
                        // Parse SSE event
                        if let Some(data) = event.strip_prefix("data: ") {
                            let data = data.trim();
                            if !data.is_empty() && data != "[DONE]" {
                                if let Ok(update) = serde_json::from_str::<StreamUpdate>(data) {
                                    if let Some(streaming_update) = self.convert_update(update) {
                                        if let Err(e) = tx.send(streaming_update).await {
                                            error!("Failed to send update: {}", e);
                                            return Ok(());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Stream error: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    fn convert_update(&self, update: StreamUpdate) -> Option<StreamingUpdate> {
        match update.update_type.as_str() {
            "slot" => {
                let parent = update.data.get("parent")
                    .and_then(|v| v.as_u64());
                
                Some(StreamingUpdate::Slot(SlotUpdate {
                    slot: update.slot,
                    parent,
                    status: CommitmentLevel::Confirmed,
                }))
            }
            "account" => {
                if let Some(pubkey_str) = update.data.get("pubkey").and_then(|v| v.as_str()) {
                    if let Ok(pubkey) = Pubkey::from_str(pubkey_str) {
                        let program = update.data.get("program")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        
                        return Some(StreamingUpdate::Account(AccountUpdate {
                            pubkey,
                            slot: update.slot,
                            program,
                        }));
                    }
                }
                None
            }
            "transaction" => {
                if let Some(signature) = update.data.get("signature").and_then(|v| v.as_str()) {
                    Some(StreamingUpdate::Transaction(TransactionUpdate {
                        signature: signature.to_string(),
                        slot: update.slot,
                    }))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}