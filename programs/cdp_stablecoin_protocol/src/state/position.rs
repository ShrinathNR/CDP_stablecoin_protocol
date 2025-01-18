use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Position {
    pub user: Pubkey,
    pub mint: Pubkey,
    pub collateral_amount: u64,
    pub current_debt: u64,
    pub ltv: u16,
    // pub interest_rate: u16,
    pub last_debt_update_time: i64,
}
