use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Position {
    pub user: Pubkey,
    pub collateral_amount: u64,
    pub debt_amount: u64,
    pub prev_cumulative_interest_rate: u128,
}
