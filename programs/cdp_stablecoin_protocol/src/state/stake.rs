use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct StakeAccount {
    pub user: Pubkey,
    pub amount: u64,
    pub points: u64,
    pub last_staked: i64,
    pub bump: u8,
}