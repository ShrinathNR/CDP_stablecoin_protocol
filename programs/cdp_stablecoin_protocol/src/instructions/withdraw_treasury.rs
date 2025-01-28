use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, MintTo, Token, TokenAccount, mint_to},
};

use crate::{errors::StakeError, state::ProtocolConfig};

#[derive(Accounts)]
pub struct WithdrawTreasury<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        init,
        space = 8 + ProtocolConfig::INIT_SPACE,
        payer = admin,
        seeds = [b"config"],
        bump
    )]
    pub protocol_config: Account<'info, ProtocolConfig>,
    #[account(
        mut,
        address = protocol_config.stable_mint,
        mint::decimals = 6,
        mint::authority = auth,
    )]
    pub stable_mint: Account<'info, Mint>,
    #[account(
        init_if_needed,
        payer = admin,
        associated_token::mint = stable_mint,
        associated_token::authority = admin,
    )]
    pub admin_ata: Account<'info, TokenAccount>,
    /// CHECK: This is an auth acc for the vault
    #[account(
        mut,
        seeds = [b"auth"],
        bump = protocol_config.auth_bump
    )]
    pub auth: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> WithdrawTreasury<'info> {
    pub fn withdraw_treasury(&mut self) -> Result<()> {
        let pending_amount = self.protocol_config.pending_treasury_rewards;
        require!(pending_amount > 0, StakeError::InsufficientFunds);

        // Mint pending rewards
        let mint_accounts = MintTo {
            mint: self.stable_mint.to_account_info(),
            to: self.admin_ata.to_account_info(),
            authority: self.auth.to_account_info(),
        };

        let seeds = &[&b"auth"[..], &[self.protocol_config.auth_bump]];
        let signer_seeds = &[&seeds[..]];

        mint_to(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                mint_accounts,
                signer_seeds,
            ),
            pending_amount
        )?;

        // Reset pending rewards
        self.protocol_config.pending_treasury_rewards = 0;

        Ok(())
    }
} 