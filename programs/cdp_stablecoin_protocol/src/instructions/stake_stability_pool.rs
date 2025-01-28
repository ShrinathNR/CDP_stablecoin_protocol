use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{transfer, Mint, Token, TokenAccount, Transfer},
};

use crate::state::{CollateralConfig, ProtocolConfig, StakeAccount};

#[derive(Accounts)]
pub struct Stake<'info> {
    #[account(mut)]
    user: Signer<'info>,
    #[account(
        init,
        payer = user,
        seeds = [b"stake", user.key().as_ref(), collateral_vault_config.mint.key().as_ref()],
        space = 8 + StakeAccount::INIT_SPACE,
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

    #[account(
        mut,
        seeds = [b"config"],
        bump = protocol_config.bump
    )]
    protocol_config: Account<'info, ProtocolConfig>,
    system_program: Program<'info, System>,
    token_program: Program<'info, Token>,
    associated_token_program: Program<'info, AssociatedToken>,
}

impl<'info> Stake<'info> {
    pub fn init_stake_account(&mut self, bumps: &StakeBumps) -> Result<()> {
        // Set the stake account
        self.stake_account.set_inner(StakeAccount {
            user: self.user.key(),
            amount: 0,
            init_deposit_depletion_factor: self.collateral_vault_config.deposit_depletion_factor,
            init_gain_summation: self.collateral_vault_config.gain_summation,
            last_staked: Clock::get()?.unix_timestamp,
            bump: bumps.stake_account,
        });

        Ok(())
    }
    pub fn deposit_tokens(&mut self, amount: u64) -> Result<()> {
        self.collateral_vault_config.claim_pending_rewards(
            &self.protocol_config,
            &self.stable_mint,
            &self.stake_vault,
            &self.auth,
            &self.token_program,
            self.protocol_config.auth_bump,
        )?;
        
        // Transfer tokens
        let cpi_program = self.token_program.to_account_info();

        let cpi_accounts = Transfer {
            from: self.user_stable_ata.to_account_info(),
            to: self.stake_vault.to_account_info(),
            authority: self.user.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        transfer(cpi_ctx, amount)?;

        let current_timestamp = Clock::get()?.unix_timestamp;

        // Update last staked timestamp
        self.stake_account.last_staked = current_timestamp;

        // Update staked amount
        self.stake_account.amount += amount;

        self.collateral_vault_config.total_stake_amount += amount as u128;

        Ok(())
    }
}
