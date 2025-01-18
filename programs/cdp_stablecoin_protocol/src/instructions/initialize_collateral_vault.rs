use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

use crate::{constants::JITO_SOL, errors::CollateralError, state::CollateralConfig};

#[derive(Accounts)]
#[instruction(auth_bump: u8)]
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
    /// CHECK: This is an auth acc for the vault
    #[account(
        seeds = [b"auth"],
        bump = auth_bump
    )]
    auth: UncheckedAccount<'info>,
    #[account(
        init,
        payer = admin,
        seeds = [b"vault", collateral_mint.key().as_ref()],
        token::mint = collateral_mint,
        token::authority = auth,
        bump
    )]
    vault: Account<'info, TokenAccount>,
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
            vault: self.vault.key(),
            amount: 0,
            bump: bumps.collateral_vault_config,
            vault_bump: bumps.vault,
        });

        Ok(())
    }
}
