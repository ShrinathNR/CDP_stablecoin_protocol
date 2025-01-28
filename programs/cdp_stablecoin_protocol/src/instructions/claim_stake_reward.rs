use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{transfer, Mint, Token, TokenAccount, Transfer},
};

use crate::{
    constants::BPS_SCALE,
    errors::ArithmeticError,
    state::{CollateralConfig, ProtocolConfig, StakeAccount},
};

#[derive(Accounts)]
pub struct ClaimStakeRewards<'info> {
    #[account(mut)]
    user: Signer<'info>,

    collateral_mint: Account<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = collateral_mint,
        associated_token::authority = user,
    )]
    user_ata: Box<Account<'info, TokenAccount>>,

    #[account(
        seeds = [b"config"],
        bump = protocol_config.bump
    )]
    protocol_config: Account<'info, ProtocolConfig>,

    /// CHECK: This is an auth acc for the vault
    #[account(
        seeds = [b"auth"],
        bump = protocol_config.auth_bump
    )]
    auth: UncheckedAccount<'info>,

    #[account(
        seeds = [b"collateral", collateral_mint.key().as_ref()],
        bump = collateral_vault_config.bump
    )]
    collateral_vault_config: Account<'info, CollateralConfig>,

    #[account(
        mut,
        seeds = [b"liquidation_rewards_vault", collateral_mint.key().as_ref()],
        token::mint = collateral_mint,
        token::authority = auth,
        bump
    )]
    liquidation_rewards_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [b"stake", user.key().as_ref(), collateral_vault_config.mint.key().as_ref()],
        bump,
    )]
    pub stake_account: Account<'info, StakeAccount>,

    token_program: Program<'info, Token>,
    associated_token_program: Program<'info, AssociatedToken>,
    system_program: Program<'info, System>,
}

impl<'info> ClaimStakeRewards<'info> {
    pub fn claim_stake_reward(&mut self) -> Result<()> {
        let stake_reward_transfer_cpi_accounts = Transfer {
            from: self.liquidation_rewards_vault.to_account_info(),
            to: self.user_ata.to_account_info(),
            authority: self.auth.to_account_info(),
        };
        let seeds = &[&b"auth"[..], &[self.protocol_config.auth_bump]];

        let signer_seeds = &[&seeds[..]];

        let stake_reward_transfer_cpi_ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            stake_reward_transfer_cpi_accounts,
            signer_seeds,
        );

        let amount = (self.stake_account.amount as u128)
            .checked_mul(
                self.collateral_vault_config
                    .gain_summation
                    .checked_sub(self.stake_account.init_gain_summation)
                    .ok_or(ArithmeticError::ArithmeticOverflow)?,
            )
            .ok_or(ArithmeticError::ArithmeticOverflow)?
            .checked_mul(BPS_SCALE as u128)
            .ok_or(ArithmeticError::ArithmeticOverflow)?
            .checked_div(self.stake_account.init_deposit_depletion_factor as u128)
            .ok_or(ArithmeticError::ArithmeticOverflow)? as u64;

        transfer(stake_reward_transfer_cpi_ctx, amount)?;

        let updated_stake_amount = self
            .stake_account
            .amount
            .checked_mul(self.collateral_vault_config.deposit_depletion_factor as u64)
            .ok_or(ArithmeticError::ArithmeticOverflow)?
            .checked_div(self.stake_account.init_deposit_depletion_factor as u64)
            .ok_or(ArithmeticError::ArithmeticOverflow)?;

        self.stake_account.init_deposit_depletion_factor = self.collateral_vault_config.deposit_depletion_factor;

        self.stake_account.init_gain_summation = self.collateral_vault_config.gain_summation;

        self.stake_account.amount = updated_stake_amount;

        Ok(())
    }
}
