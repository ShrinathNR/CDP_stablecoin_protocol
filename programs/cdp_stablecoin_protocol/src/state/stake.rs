use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct StakeAccount {
    pub user: Pubkey,
    pub amount: u64,
    pub init_deposit_depletion_factor: u128,
    pub init_gain_summation: u128,
    pub last_staked: i64,
    pub bump: u8,
}
