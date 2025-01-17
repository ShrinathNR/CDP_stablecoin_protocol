use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Position {
    pub user: Pubkey,
    pub collateral_amount: u64,
    pub debt_amount: u64,
    pub initial_interest_index: u64,
}
