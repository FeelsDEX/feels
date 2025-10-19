// Utility functions for CLI commands

use anyhow::{Context, Result};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair},
};
use std::str::FromStr;

/// Load a keypair from a file path, expanding ~ if needed
pub fn load_keypair(path: &str) -> Result<Keypair> {
    let expanded_path = if path.starts_with("~") {
        let home = std::env::var("HOME").context("HOME environment variable not set")?;
        path.replacen("~", &home, 1)
    } else {
        path.to_string()
    };

    read_keypair_file(&expanded_path)
        .map_err(|e| anyhow::anyhow!("Failed to load keypair from {}: {}", expanded_path, e))
}

/// Parse a pubkey from string
pub fn parse_pubkey(s: &str) -> Result<Pubkey> {
    Pubkey::from_str(s).context("Invalid public key")
}

/// Get program ID from option or use default
pub fn get_program_id(program_id_str: Option<&str>) -> Result<Pubkey> {
    match program_id_str {
        Some(id) => parse_pubkey(id),
        None => Ok(feels_sdk::core::program_id()),
    }
}

/// Print success message with checkmark
pub fn success(msg: &str) {
    println!("[OK] {}", msg);
}

/// Print info message
pub fn info(msg: &str) {
    println!("[INFO] {}", msg);
}

/// Print warning message
pub fn warn(msg: &str) {
    eprintln!("[WARN] {}", msg);
}

/// Print error message
#[allow(dead_code)]
pub fn error(msg: &str) {
    eprintln!("[ERROR] {}", msg);
}
