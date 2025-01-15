use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct CollateralConfig {
    pub mint: Pubkey,
    pub vault: Pubkey,
    pub amount: u64,
    pub bump: u8,
    pub vault_bump: u8,
}
