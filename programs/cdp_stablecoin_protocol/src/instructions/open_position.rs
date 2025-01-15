use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{mint_to, transfer, Mint, MintTo, Token, TokenAccount, Transfer},
};

use crate::{
    constants::{MAX_LTV, MIN_LTV},
    errors::{ArithmeticError, PositionError},
    state::{CollateralConfig, Position},
};

#[derive(Accounts)]
#[instruction(auth_bump: u8)]
pub struct OpenPosition<'info> {
    #[account(mut)]
    user: Signer<'info>,

    collateral_mint: Account<'info, Mint>,

    // #[account(
    //     //address constraint
    // )]
    stable_mint: Account<'info, Mint>,
    /// CHECK: This is an auth acc for the vault
    #[account(
        seeds = [b"auth"],
        bump = auth_bump
    )]
    auth: UncheckedAccount<'info>,
    #[account(
        mut,
        associated_token::mint = collateral_mint,
        associated_token::authority = user,
    )]
    user_ata: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = stable_mint,
        associated_token::authority = user,
    )]
    user_stable_ata: Box<Account<'info, TokenAccount>>,
    #[account(
        seeds = [b"collateral", collateral_mint.key().as_ref()],
        bump = collateral_vault_config.bump
    )]
    collateral_vault_config: Account<'info, CollateralConfig>,
    #[account(
        init,
        payer = user,
        space = 8 + Position::INIT_SPACE,
        seeds = [b"position", collateral_mint.key().as_ref()],
        bump
    )]
    position: Account<'info, Position>,
    #[account(
        mut,
        seeds = [b"vault", collateral_mint.key().as_ref()],
        token::mint = collateral_mint,
        token::authority = auth,
        bump = collateral_vault_config.vault_bump
    )]
    vault: Account<'info, TokenAccount>,
    token_program: Program<'info, Token>,
    associated_token_program: Program<'info, AssociatedToken>,
    system_program: Program<'info, System>,
}

impl<'info> OpenPosition<'info> {
    pub fn open_position(&mut self, auth_bump: u8, amount: u64, ltv: u16) -> Result<()> {
        // require!(MIN_INTEREST_RATE<= interest_rate && interest_rate <= MAX_INTEREST_RATE, PositionError::InvalidInterestRate);
        require!(MIN_LTV <= ltv && ltv <= MAX_LTV, PositionError::InvalidLTV);

        self.position.set_inner(Position {
            user: self.user.key(),
            mint: self.collateral_mint.key(),
            amount,
            current_debt: amount,
            // interest_rate,
            last_debt_update_time: Clock::get()?.unix_timestamp,
        });

        let collateral_transfer_cpi_accounts = Transfer {
            from: self.user_ata.to_account_info(),
            to: self.vault.to_account_info(),
            authority: self.user.to_account_info(),
        };

        let collateral_transfer_cpi_ctx = CpiContext::new(
            self.token_program.to_account_info(),
            collateral_transfer_cpi_accounts,
        );

        transfer(collateral_transfer_cpi_ctx, amount)?;

        let stable_transfer_cpi_accounts = Transfer {
            from: self.user_ata.to_account_info(),
            to: self.vault.to_account_info(),
            authority: self.user.to_account_info(),
        };

        let stable_transfer_cpi_ctx = CpiContext::new(
            self.token_program.to_account_info(),
            stable_transfer_cpi_accounts,
        );

        transfer(stable_transfer_cpi_ctx, amount)?;

        self.collateral_vault_config
            .amount
            .checked_add(amount)
            .ok_or(ArithmeticError::ArithmeticOverflow)?;

        // implement mint amount with oracle

        // let accounts = MintTo {
        //     mint: self.stable_mint.to_account_info(),
        //     to: self.vault.to_account_info(),
        //     authority: self.auth.to_account_info(),
        // };

        // let seeds = &[
        //     &b"auth"[..],
        //     &[auth_bump],
        // ];

        // let signer_seeds = &[&seeds[..]];

        // let stable_mint_cpi_ctx = CpiContext::new_with_signer(
        //     self.token_program.to_account_info(),
        //     accounts,
        //     signer_seeds,
        // );

        // mint_to(stable_mint_cpi_ctx, )?;

        Ok(())
    }
}
