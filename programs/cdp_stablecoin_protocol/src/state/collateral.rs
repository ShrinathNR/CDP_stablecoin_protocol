use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct CollateralConfig {
    pub mint: Pubkey,
    #[max_len(100)]
    pub collateral_price_feed: String,
    pub vault: Pubkey,
    pub collateral_amount: u64,
    pub stability_pool_rewards_amount: u64,
    pub gain_summation: u128,
    pub bump: u8,
    pub vault_bump: u8,
}
