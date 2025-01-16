use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{burn, transfer, Burn, Mint, Token, TokenAccount, Transfer},
};
use pyth_solana_receiver_sdk::price_update::{get_feed_id_from_hex, PriceUpdateV2};

use crate::{
    constants::MAX_LTV,
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
        mut,
        close = user,
        seeds = [b"position", user.key().as_ref(), collateral_mint.key().as_ref()],
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
    price_update: Account<'info, PriceUpdateV2>,
    token_program: Program<'info, Token>,
    associated_token_program: Program<'info, AssociatedToken>,
    system_program: Program<'info, System>,
}

impl<'info> OpenPosition<'info> {
    pub fn open_position(&mut self, auth_bump: u8) -> Result<()> {
        // require!(MIN_INTEREST_RATE<= interest_rate && interest_rate <= MAX_INTEREST_RATE, PositionError::InvalidInterestRate);
        // require!(MIN_LTV <= ltv && ltv <= MAX_LTV, PositionError::InvalidLTV);

        let collateral_transfer_cpi_accounts = Transfer {
            from: self.vault.to_account_info(),
            to: self.user_ata.to_account_info(),
            authority: self.user.to_account_info(),
        };
        let seeds = &[&b"auth"[..], &[auth_bump]];

        let signer_seeds = &[&seeds[..]];

        let collateral_transfer_cpi_ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            collateral_transfer_cpi_accounts,
            signer_seeds,
        );

        transfer(collateral_transfer_cpi_ctx, self.position.collateral_amount)?;

        self.collateral_vault_config
            .amount
            .checked_sub(self.position.collateral_amount)
            .ok_or(ArithmeticError::ArithmeticOverflow)?;

        let price_update = &mut self.price_update;
        // get_price_no_older_than will fail if the price update is more than 30 seconds old
        let maximum_age: u64 = 30;
        let feed_id: [u8; 32] = get_feed_id_from_hex(&self.collateral_vault_config.price_feed)?;
        let price = price_update.get_price_no_older_than(&Clock::get()?, maximum_age, &feed_id)?;

        let current_ltv = (self.position.current_debt)
            .checked_div(price.price as u64)
            .ok_or(ArithmeticError::ArithmeticOverflow)?
            .checked_div(10 ^ price.exponent as u64)
            .ok_or(ArithmeticError::ArithmeticOverflow)?
            .checked_div(self.position.collateral_amount)
            .ok_or(ArithmeticError::ArithmeticOverflow)? as u16;

        if MAX_LTV >= current_ltv {
            let accounts = Burn {
                mint: self.stable_mint.to_account_info(),
                from: self.user_stable_ata.to_account_info(),
                authority: self.user.to_account_info(),
            };

            let stable_burn_cpi_ctx =
                CpiContext::new(self.token_program.to_account_info(), accounts);

            burn(stable_burn_cpi_ctx, self.position.current_debt)?;
        } else {
            return err!(PositionError::InvalidLTV);
        }

        Ok(())
    }
}
