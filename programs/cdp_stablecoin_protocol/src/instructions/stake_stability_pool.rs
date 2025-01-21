use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{transfer, Mint, Token, TokenAccount, Transfer},
};

use crate::{errors::{ArithmeticError, StakeError}, state::{ProtocolConfig, StakeAccount}};

#[derive(Accounts)]
pub struct Stake<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        init_if_needed,
        payer = user,
        seeds = [b"stake", user.key().as_ref()],
        space = StakeAccount::INIT_SPACE,
        bump,
    )]
    pub stake_account: Account<'info, StakeAccount>,
    #[account(
        address = protocol_config.stable_mint
    )]
    pub stable_mint: Account<'info, Mint>,
    #[account(
        mut,
        associated_token::mint = stable_mint,
        associated_token::authority = user,
    )]
    pub user_ata: Account<'info, TokenAccount>,

    /// CHECK: This is an auth acc for the vault
    #[account(
        seeds = [b"auth"],
        bump
    )]
    pub auth: UncheckedAccount<'info>,
    #[account(
        init_if_needed,
        payer = user,
        seeds = [b"stake_vault", stable_mint.key().as_ref()],
        token::mint = stable_mint,
        token::authority = auth,
        bump
    )]
    pub stake_vault: Account<'info, TokenAccount>,
    protocol_config: Account<'info, ProtocolConfig>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl<'info> Stake<'info> {
    pub fn init_stake_account(&mut self, amount: u64, bumps: &StakeBumps) -> Result<()> {
        // Set the stake account
        self.stake_account.set_inner(StakeAccount {
            user: self.user.key(),
            amount,
            points: 0,
            last_staked: Clock::get()?.unix_timestamp,
            bump: bumps.stake_account,
        });

        Ok(())
    }
    pub fn deposit_tokens(&mut self, amount: u64) -> Result<()> {

        // Transfer tokens
        let cpi_program = self.token_program.to_account_info();

        let cpi_accounts = Transfer {
            from: self.user_ata.to_account_info(),
            to: self.stake_vault.to_account_info(),
            authority: self.user.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        transfer(cpi_ctx, amount)?;

        let current_timestamp = Clock::get()?.unix_timestamp;

        let points = ((current_timestamp
            .checked_sub(self.stake_account.last_staked)
            .ok_or(ArithmeticError::ArithmeticOverflow)?)
        .checked_div(86400)
        .ok_or(ArithmeticError::ArithmeticOverflow)? as u64)
            .checked_mul(self.stake_account.amount)
            .ok_or(ArithmeticError::ArithmeticOverflow)?;

        self.stake_account.points += points;
        
        self.protocol_config.stake_points += points;
        
        // Update last staked timestamp
        self.stake_account.last_staked = current_timestamp;

        // Update staked amount
        self.stake_account.amount += amount;


        Ok(())
    }

    pub fn withdraw_tokens(&mut self, amount: u64, bumps: &StakeBumps) -> Result<()> {
        // Perform checks
        require!(
            amount <= self.stake_account.amount,
            StakeError::InsufficientFunds
        );

        // Transfer tokens
        let cpi_program = self.token_program.to_account_info();

        let cpi_accounts = Transfer {
            from: self.stake_vault.to_account_info(),
            to: self.user_ata.to_account_info(),
            authority: self.auth.to_account_info(), //to be updated to Compute Labs wallet
        };
        let signer_seeds = &[
            &b"auth"[..],
            &[bumps.auth],
        ];
        let binding = [&signer_seeds[..]];

        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, &binding);

        transfer(cpi_ctx, amount)?;

        let current_timestamp = Clock::get()?.unix_timestamp;

        let points = ((current_timestamp
            .checked_sub(self.stake_account.last_staked)
            .ok_or(ArithmeticError::ArithmeticOverflow)?)
        .checked_div(86400)
        .ok_or(ArithmeticError::ArithmeticOverflow)? as u64)
            .checked_mul(self.stake_account.amount)
            .ok_or(ArithmeticError::ArithmeticOverflow)?;

        self.stake_account.points += points;

        self.protocol_config.stake_points += points;

        // Update last staked timestamp
        self.stake_account.last_staked = current_timestamp;

        // Update staked amount
        self.stake_account.amount -= amount;

        Ok(())
    }
}