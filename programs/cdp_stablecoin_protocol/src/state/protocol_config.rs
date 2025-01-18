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
    pub stablecoin_price_feed: Pubkey,
    pub total_debt: u64,
    pub total_collateral: u64,
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

    pub fn update_totals(&mut self, debt_change: i64, collateral_change: i64) -> Result<()> {
        // Update total debt
        if debt_change > 0 {
            self.total_debt = self.total_debt
                .checked_add(debt_change as u64)
                .ok_or(ArithmeticError::ArithmeticOverflow)?;
        } else {
            self.total_debt = self.total_debt
                .checked_sub((-debt_change) as u64)
                .ok_or(ArithmeticError::ArithmeticOverflow)?;
        }

        // Update total collateral
        if collateral_change > 0 {
            self.total_collateral = self.total_collateral
                .checked_add(collateral_change as u64)
                .ok_or(ArithmeticError::ArithmeticOverflow)?;
        } else {
            self.total_collateral = self.total_collateral
                .checked_sub((-collateral_change) as u64)
                .ok_or(ArithmeticError::ArithmeticOverflow)?;
        }
        
        Ok(())
    }
}
