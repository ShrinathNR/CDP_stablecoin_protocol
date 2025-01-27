use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{transfer, Mint, Token, TokenAccount, Transfer},
};

use crate::state::{CollateralConfig, ProtocolConfig, StakeAccount};

#[derive(Accounts)]
pub struct UnStake<'info> {
    #[account(mut)]
    user: Signer<'info>,
    #[account(
        mut,
        close = user,
        seeds = [b"stake", user.key().as_ref(), collateral_vault_config.mint.key().as_ref()],
        bump,
    )]
    stake_account: Account<'info, StakeAccount>,
    #[account(
        address = protocol_config.stable_mint
    )]
    stable_mint: Account<'info, Mint>,
    #[account(
        mut,
        associated_token::mint = stable_mint,
        associated_token::authority = user,
    )]
    user_stable_ata: Account<'info, TokenAccount>,

    /// CHECK: This is an auth acc for the vault
    #[account(
        seeds = [b"auth"],
        bump
    )]
    auth: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [b"stake_vault", stable_mint.key().as_ref(), collateral_vault_config.mint.key().as_ref()],
        token::mint = stable_mint,
        token::authority = auth,
        bump
    )]
    stake_vault: Account<'info, TokenAccount>,
    #[account(
        seeds = [b"collateral", collateral_vault_config.mint.key().as_ref()],
        bump = collateral_vault_config.bump
    )]
    collateral_vault_config: Box<Account<'info, CollateralConfig>>,
    protocol_config: Account<'info, ProtocolConfig>,
    system_program: Program<'info, System>,
    token_program: Program<'info, Token>,
    associated_token_program: Program<'info, AssociatedToken>,
}

impl<'info> UnStake<'info> {
    pub fn withdraw_tokens(&mut self, bumps: &UnStakeBumps) -> Result<()> {
        // Transfer tokens
        let cpi_program = self.token_program.to_account_info();

        let cpi_accounts = Transfer {
            from: self.stake_vault.to_account_info(),
            to: self.user_stable_ata.to_account_info(),
            authority: self.auth.to_account_info(),
        };
        let signer_seeds = &[&b"auth"[..], &[bumps.auth]];
        let binding = [&signer_seeds[..]];

        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, &binding);

        transfer(cpi_ctx, self.stake_vault.amount)?;

        let current_timestamp = Clock::get()?.unix_timestamp;

        // Update last staked timestamp
        self.stake_account.last_staked = current_timestamp;

        // Update staked amount
        self.stake_account.amount = 0;

        Ok(())
    }
}
