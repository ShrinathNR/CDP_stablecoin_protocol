use anchor_lang::prelude::*;
use crate::{
    errors::ArithmeticError,
    state::Position
};

#[account]
#[derive(InitSpace)]
pub struct ProtocolConfig {
    pub stable_mint: Pubkey,
    pub protocol_fee: u16,
    pub redemption_fee: u16,
    pub mint_fee: u16,
    pub base_rate: u16,
    pub sigma: u16,
    pub auth_bump: u8,
    pub interest_index: u64,
    pub last_index_update: i64,
    #[max_len(64)]
    pub stablecoin_price_feed: String,
    pub total_debt: u64,
    pub stake_points: u64
}

#[account]
pub struct ProtocolState {
    pub total_collateral: u64,
    pub total_stablecoin: u64,
    pub collateral_ratio: u64,
    pub liquidation_threshold: u64,
    pub price_feed_id: String,
}

impl ProtocolConfig {
    pub const INITIAL_INTEREST_INDEX: u64 = 1_000_000;

    pub fn calculate_current_debt(&self, position: &Position) -> Result<u64> {
        let current_debt = (position.debt_amount as u128)
            .checked_mul(self.interest_index as u128)
            .ok_or(ArithmeticError::ArithmeticOverflow)?
            .checked_div(position.initial_interest_index as u128)
            .ok_or(ArithmeticError::ArithmeticOverflow)? as u64;
            
        Ok(current_debt)
    }

    pub fn update_totals(&mut self, _debt_change: i64) -> Result<()> {
        if _debt_change > 0 {
            self.total_debt = self.total_debt
                .checked_add(_debt_change as u64)
                .ok_or(ArithmeticError::ArithmeticOverflow)?;
        } else {
            self.total_debt = self.total_debt
                .checked_sub((-_debt_change) as u64)
                .ok_or(ArithmeticError::ArithmeticOverflow)?;
        }
        
        Ok(())
    }
}
