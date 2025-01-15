use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct CollateralConfig {
    pub mint: Pubkey,
    pub vault: Pubkey,
}
