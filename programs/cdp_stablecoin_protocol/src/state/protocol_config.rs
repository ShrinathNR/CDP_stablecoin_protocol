use crate::{constants::INTEREST_SCALE, errors::ArithmeticError, state::Position};
use anchor_lang::prelude::*;
use anchor_spl::token::{mint_to, Mint, MintTo, Token, TokenAccount};

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
    pub total_stake_amount: u128,
    pub deposit_depletion_factor: u16,
    pub revenue_share_to_stability_pool: u16, // in bps (e.g. 5000 = 50%)
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
    
    /// Distributes revenue between treasury and stability pool vaults
    /// If there are no stakers (total_stake_amount = 0), all revenue goes to treasury
    pub fn distribute_revenue<'info>(
        &self,
        revenue_amount: u64,
        stable_mint: &Account<'info, Mint>,
        treasury_vault: &Account<'info, TokenAccount>,
        stake_vault: &Account<'info, TokenAccount>,
        auth: &UncheckedAccount<'info>,
        token_program: &Program<'info, Token>,
    ) -> Result<()> {
        // If no stakers, all revenue goes to treasury
        if self.total_stake_amount == 0 {
            let accounts = MintTo {
                mint: stable_mint.to_account_info(),
                to: treasury_vault.to_account_info(),
                authority: auth.to_account_info(),
            };
            let seeds = &[&b"auth"[..], &[self.auth_bump]];
            let signer_seeds = &[&seeds[..]];
            let cpi_ctx = CpiContext::new_with_signer(
                token_program.to_account_info(),
                accounts,
                signer_seeds,
            );
            mint_to(cpi_ctx, revenue_amount)?;
            return Ok(());
        }
        // Calculate split based on revenue_share_to_stability_pool
        let stability_pool_amount = (revenue_amount as u128)
            .checked_mul(self.revenue_share_to_stability_pool as u128)
            .ok_or(ArithmeticError::ArithmeticOverflow)?
            .checked_div(10000)
            .ok_or(ArithmeticError::ArithmeticOverflow)? as u64;
        let treasury_amount = revenue_amount
            .checked_sub(stability_pool_amount)
            .ok_or(ArithmeticError::ArithmeticOverflow)?;
        // Mint to stability pool
        let accounts = MintTo {
            mint: stable_mint.to_account_info(),
            to: stake_vault.to_account_info(),
            authority: auth.to_account_info(),
        };
        let seeds = &[&b"auth"[..], &[self.auth_bump]];
        let signer_seeds = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(
            token_program.to_account_info(),
            accounts,
            signer_seeds,
        );
        mint_to(cpi_ctx, stability_pool_amount)?;
        // Mint to treasury
        let accounts = MintTo {
            mint: stable_mint.to_account_info(),
            to: treasury_vault.to_account_info(),
            authority: auth.to_account_info(),
        };
        let seeds = &[&b"auth"[..], &[self.auth_bump]];
        let signer_seeds = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(
            token_program.to_account_info(),
            accounts,
            signer_seeds,
        );
        mint_to(cpi_ctx, treasury_amount)?;
        Ok(())
    }
}
