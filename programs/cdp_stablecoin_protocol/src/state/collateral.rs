use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct CollateralConfig {
    pub mint: Pubkey,
    #[max_len(64)]
    pub collateral_price_feed: String,
    pub vault: Pubkey,
    pub amount: u64,
    pub bump: u8,
    pub vault_bump: u8,
}
