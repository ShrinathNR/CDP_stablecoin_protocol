use anchor_lang::prelude::*;
use crate::{constants::INTEREST_SCALE, errors::ArithmeticError, state::ProtocolConfig};
use anchor_spl::token::{mint_to, Mint, MintTo, Token, TokenAccount};


#[account]
#[derive(InitSpace)]
pub struct CollateralConfig {
    pub mint: Pubkey,
    #[max_len(100)]
    pub collateral_price_feed: String,
    pub vault: Pubkey,
    pub collateral_amount: u64,
    pub stability_pool_rewards_amount: u64,
    pub deposit_depletion_factor: u128,
    pub gain_summation: u128,
    pub total_stake_amount: u128,
    pub bump: u8,
    pub vault_bump: u8,
    pub total_debt: u128,
    pub last_reward_per_debt: u128,  // S_0 - snapshot of S when debt last changed
    pub last_compound_cumulative_rate: u128,  // Track last time debt was compounded. for updating total debt for collateral vaults
}

impl CollateralConfig {
    pub fn claim_pending_rewards<'info>(
        &mut self,
        protocol_config: &ProtocolConfig,
        stable_mint: &Account<'info, Mint>,
        stake_vault: &Account<'info, TokenAccount>,
        auth: &AccountInfo<'info>,
        token_program: &Program<'info, Token>,
        auth_bump: u8,
    ) -> Result<()> {
        if self.total_debt == 0 {
            return Ok(());
        }
        let pending_reward = (self.total_debt)
            .checked_mul(
                protocol_config.cumulative_reward_per_debt
                    .checked_sub(self.last_reward_per_debt)
                    .ok_or(ArithmeticError::ArithmeticOverflow)?
            )
            .ok_or(ArithmeticError::ArithmeticOverflow)?
            .checked_div(INTEREST_SCALE)
            .ok_or(ArithmeticError::ArithmeticOverflow)? as u64;
        
        // increment deposit depletion factor to account for pending rewards for stakers
        self.deposit_depletion_factor = self.deposit_depletion_factor
            .checked_add(
                self.deposit_depletion_factor
                    .checked_mul(pending_reward as u128)
                    .ok_or(ArithmeticError::ArithmeticOverflow)?
                    .checked_div(self.total_debt)
                    .ok_or(ArithmeticError::ArithmeticOverflow)?
            ).ok_or(ArithmeticError::ArithmeticOverflow)?;

        if pending_reward > 0 {
            let mint_accounts = MintTo {
                mint: stable_mint.to_account_info(),
                to: stake_vault.to_account_info(),
                authority: auth.to_account_info(),
            };

            mint_to(
                CpiContext::new_with_signer(
                    token_program.to_account_info(),
                    mint_accounts,
                    &[&[b"auth", &[auth_bump]]]
                ),
                pending_reward
            )?;
        }

        self.last_reward_per_debt = protocol_config.cumulative_reward_per_debt;
        Ok(())
    }

    pub fn compound_total_debt(&mut self, protocol_config: &ProtocolConfig) -> Result<()> {
        // Calculate compound factor since last update
        let compound_factor = protocol_config
            .cumulative_interest_rate
            .checked_mul(INTEREST_SCALE)
            .ok_or(ArithmeticError::ArithmeticOverflow)?
            .checked_div(self.last_compound_cumulative_rate)
            .ok_or(ArithmeticError::ArithmeticOverflow)?;

        // Update total debt
        self.total_debt = self.total_debt
            .checked_mul(compound_factor)
            .ok_or(ArithmeticError::ArithmeticOverflow)?
            .checked_div(INTEREST_SCALE)
            .ok_or(ArithmeticError::ArithmeticOverflow)?;

        // Update last compound rate
        self.last_compound_cumulative_rate = protocol_config.cumulative_interest_rate;

        Ok(())
    }
}