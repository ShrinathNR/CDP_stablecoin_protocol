use crate::{constants::INTEREST_SCALE, errors::ArithmeticError, state::Position};
use anchor_lang::prelude::*;

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
    pub bump: u8,
    pub cumulative_interest_rate: u128,
    pub last_interest_rate_update: i64,
    #[max_len(64)]
    pub stablecoin_price_feed: String,
    pub total_debt: u128,
    pub stake_points: u64,
}

impl ProtocolConfig {
    pub const INITIAL_CUMULATIVE_RATE: u128 = INTEREST_SCALE;

    pub fn calculate_current_debt(&self, position: &Position) -> Result<u64> {
        let current_debt = (position.debt_amount as u128)
            .checked_mul(self.cumulative_interest_rate as u128)
            .ok_or(ArithmeticError::ArithmeticOverflow)?
            .checked_div(position.prev_cumulative_interest_rate as u128)
            .ok_or(ArithmeticError::ArithmeticOverflow)? as u64;

        Ok(current_debt)
    }

    pub fn update_totals(&mut self, _debt_change: i64) -> Result<()> {
        if _debt_change > 0 {
            self.total_debt = self
                .total_debt
                .checked_add(_debt_change as u128)
                .ok_or(ArithmeticError::ArithmeticOverflow)?;
        } else {
            self.total_debt = self
                .total_debt
                .checked_sub((-_debt_change) as u128)
                .ok_or(ArithmeticError::ArithmeticOverflow)?;
        }

        Ok(())
    }
}
