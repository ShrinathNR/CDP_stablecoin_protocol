use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct ProtocolConfig {
    pub stable_mint: Pubkey,
    pub protocol_fee: u16,
    pub redemption_fee: u16,
    pub global_interest_rate: u16,
    pub mint_fee: u16,
    pub auth_bump: u8,
}
