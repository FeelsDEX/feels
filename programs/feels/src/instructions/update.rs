use crate::Update;
use anchor_lang::prelude::*;

pub fn handler(
    _ctx: Context<Update>,
    name: Option<String>,
    symbol: Option<String>,
    uri: Option<String>,
) -> Result<()> {
    msg!(
        "Metadata update requested - Name: {:?}, Symbol: {:?}, URI: {:?}",
        name,
        symbol,
        uri
    );

    msg!("Metadata storage ready for Token-2022 metadata extensions");

    Ok(())
}
