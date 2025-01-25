use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::{
    constants::BPS_SCALE,
    state::{CollateralConfig, ProtocolConfig},
};

#[derive(Accounts)]
pub struct InitializeCollateralVault<'info> {
    #[account(mut)]
    admin: Signer<'info>,

    collateral_mint: Account<'info, Mint>,
    #[account(
        init,
        space = 8 + CollateralConfig::INIT_SPACE,
        payer = admin,
        seeds = [b"collateral", collateral_mint.key().as_ref()],
        bump
    )]
    collateral_vault_config: Account<'info, CollateralConfig>,

    #[account(
        mut,
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
        init,
        payer = admin,
        seeds = [b"collateral_vault", collateral_mint.key().as_ref()],
        token::mint = collateral_mint,
        token::authority = auth,
        bump
    )]
    collateral_vault: Account<'info, TokenAccount>,
    #[account(
        init,
        payer = admin,
        seeds = [b"liquidation_rewards_vault", collateral_mint.key().as_ref()],
        token::mint = collateral_mint,
        token::authority = auth,
        bump
    )]
    liquidation_rewards_vault: Account<'info, TokenAccount>,
    token_program: Program<'info, Token>,
    system_program: Program<'info, System>,
}

impl<'info> InitializeCollateralVault<'info> {
    pub fn initialize_collateral_vault(
        &mut self,
        collateral_price_feed: String,
        bumps: &InitializeCollateralVaultBumps,
    ) -> Result<()> {
        self.collateral_vault_config.set_inner(CollateralConfig {
            mint: self.collateral_mint.key(),
            collateral_price_feed,
            vault: self.collateral_vault.key(),
            collateral_amount: 0,
            stability_pool_rewards_amount: 0,
            deposit_depletion_factor: BPS_SCALE,
            gain_summation: 0,
            bump: bumps.collateral_vault_config,
            vault_bump: bumps.collateral_vault,
        });

        Ok(())
    }
}
