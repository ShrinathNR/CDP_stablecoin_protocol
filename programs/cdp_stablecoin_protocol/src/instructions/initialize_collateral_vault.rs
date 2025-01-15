use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::state::CollateralConfig;

#[derive(Accounts)]
#[instruction(auth_bump: u8)]
pub struct InitializeCollateralVault<'info> {
    #[account(mut)]
    admin: Signer<'info>,

    mint: Account<'info, Mint>,
    #[account(
        init,
        space = 8 + CollateralConfig::INIT_SPACE,
        payer = admin,
        seeds = [b"collateral", mint.key().as_ref()],
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
        token::mint = mint,
        token::authority = auth,
    )]
    vault: Account<'info, TokenAccount>,
    token_program: Program<'info, Token>,
    system_program: Program<'info, System>,
}

impl<'info> InitializeCollateralVault<'info> {
    pub fn initialize_collateral_vault(&mut self) -> Result<()> {
        self.collateral_vault_config.set_inner(CollateralConfig {
            mint: self.mint.key(),
            vault: self.vault.key(),
        });

        Ok(())
    }
}
