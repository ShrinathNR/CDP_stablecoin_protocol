use crate::{constants::{BPS_SCALE, INTEREST_SCALE}, errors::ArithmeticError, state::Position};
use anchor_lang::prelude::*;
// use anchor_spl::token::{mint_to, Mint, MintTo, Token, TokenAccount};

#[account]
#[derive(InitSpace)]
pub struct ProtocolConfig {
    pub admin: Pubkey,
    pub stable_mint: Pubkey,
    pub protocol_fee: u16,
    pub redemption_fee: u16,
    pub mint_fee: u16,
    pub base_rate: u16,
    pub sigma: u16,
    pub auth_bump: u8,
    pub bump: u8,
    pub cumulative_interest_rate: u128,
    pub last_interest_rate_update_time: i64,
    pub last_interest_rate: u128, // yearly rate
    #[max_len(64)]
    pub stablecoin_price_feed: String,
    pub total_debt: u128,
    pub revenue_share_to_stability_pool: u16, // in bps (e.g. 5000 = 50%)
    pub cumulative_reward_per_debt: u128,  // S - tracks accumulated (reward/total_debt) / for collaterals
    pub pending_treasury_rewards: u64,     // Track unminted treasury portion
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
    
    pub fn accumulate_reward(&mut self, reward_amount: u64) -> Result<()> {
        if self.total_debt > 0 {
            // Calculate stability pool portion (in BPS)
            let stability_pool_amount = (reward_amount as u128)
                .checked_mul(self.revenue_share_to_stability_pool as u128)
                .ok_or(ArithmeticError::ArithmeticOverflow)?
                .checked_div(BPS_SCALE as u128)
                .ok_or(ArithmeticError::ArithmeticOverflow)? as u64;

            // Calculate and accumulate treasury portion
            let treasury_amount = reward_amount
                .checked_sub(stability_pool_amount)
                .ok_or(ArithmeticError::ArithmeticOverflow)?;

            self.pending_treasury_rewards = self.pending_treasury_rewards
                .checked_add(treasury_amount)
                .ok_or(ArithmeticError::ArithmeticOverflow)?;

            // Accumulate stability pool portion to collateral vaults
            if stability_pool_amount > 0 {
                self.cumulative_reward_per_debt = self
                    .cumulative_reward_per_debt
                    .checked_add(
                        (stability_pool_amount as u128)
                            .checked_mul(INTEREST_SCALE)
                            .ok_or(ArithmeticError::ArithmeticOverflow)?
                            .checked_div(self.total_debt)
                            .ok_or(ArithmeticError::ArithmeticOverflow)?
                    )
                    .ok_or(ArithmeticError::ArithmeticOverflow)?;
            }
        }
        Ok(())
    }
}
