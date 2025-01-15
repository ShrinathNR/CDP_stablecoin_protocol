use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct ProtocolConfig {
    pub protocol_fee: u16,
    pub redemption_fee: u16,
    pub mint_fee: u16,
    pub min_interest_rate: u16,
    pub max_interest_rate: u16,
    pub auth_bump: u8,
}
