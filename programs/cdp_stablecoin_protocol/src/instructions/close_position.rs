use anchor_lang::{prelude::*, solana_program::native_token::LAMPORTS_PER_SOL};
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{burn, transfer, Burn, Mint, Token, TokenAccount, Transfer},
};
use pyth_solana_receiver_sdk::price_update::{get_feed_id_from_hex, PriceUpdateV2};

use crate::{
    constants::MAX_LTV,
    errors::{ArithmeticError, PositionError},
    state::{CollateralConfig, Position, ProtocolConfig},
};

#[derive(Accounts)]
pub struct ClosePosition<'info> {
    #[account(mut)]
    user: Signer<'info>,

    collateral_mint: Account<'info, Mint>,

    #[account(
        mut,
        address = protocol_config.stable_mint,
        mint::decimals = 6,
        mint::authority = auth,
    )]
    stable_mint: Account<'info, Mint>,

    protocol_config: Account<'info, ProtocolConfig>,

    /// CHECK: This is an auth acc for the vault
    #[account(
        mut,
        seeds = [b"auth"],
        bump = protocol_config.auth_bump
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
        mut,
        seeds = [b"collateral", collateral_mint.key().as_ref()],
        bump = collateral_vault_config.bump
    )]
    collateral_vault_config: Account<'info, CollateralConfig>,
    #[account(
        mut,
        close = user,
        seeds = [b"position", user.key().as_ref(), collateral_mint.key().as_ref()],
        bump,
    )]
    position: Account<'info, Position>,

    #[account(owner = pyth_solana_receiver_sdk::ID)]
    price_feed: Account<'info, PriceUpdateV2>,

    #[account(
        mut,
        seeds = [b"collateral_vault", collateral_mint.key().as_ref()],
        token::mint = collateral_mint,
        token::authority = auth,
        bump = collateral_vault_config.vault_bump
    )]
    collateral_vault: Account<'info, TokenAccount>,

    token_program: Program<'info, Token>,
    associated_token_program: Program<'info, AssociatedToken>,
    system_program: Program<'info, System>,
}

impl<'info> ClosePosition<'info> {
    pub fn close_position(&mut self) -> Result<()> {
        // require!(MIN_INTEREST_RATE<= interest_rate && interest_rate <= MAX_INTEREST_RATE, PositionError::InvalidInterestRate);
        // require!(MIN_LTV <= ltv && ltv <= MAX_LTV, PositionError::InvalidLTV);

        let current_debt = self
            .protocol_config
            .calculate_current_debt(&self.position)?;

        let price_feed = &self.price_feed;
        // let maximum_age: u64 = 30;
        let feed_id: [u8; 32] =
            get_feed_id_from_hex(&self.collateral_vault_config.collateral_price_feed)?;
        // let price = price_feed.get_price_no_older_than(&Clock::get()?, maximum_age, &feed_id)?;
        let price = price_feed.get_price_unchecked(&feed_id)?;

        let collateral_value = (price.price as u128)
            .checked_mul(self.position.collateral_amount as u128)
            .ok_or(ArithmeticError::ArithmeticOverflow)?
            .checked_div(10_u128.pow(price.exponent.abs() as u32))
            .ok_or(ArithmeticError::ArithmeticOverflow)?
            .checked_div(LAMPORTS_PER_SOL as u128)
            .ok_or(ArithmeticError::ArithmeticOverflow)?;

        let ltv = (current_debt as u128)
            .checked_mul(10000)
            .ok_or(ArithmeticError::ArithmeticOverflow)?
            .checked_div(collateral_value as u128)
            .ok_or(ArithmeticError::ArithmeticOverflow)? as u16;

        if MAX_LTV >= ltv {
            let collateral_transfer_cpi_accounts = Transfer {
                from: self.collateral_vault.to_account_info(),
                to: self.user_ata.to_account_info(),
                authority: self.auth.to_account_info(),
            };
            let seeds = &[&b"auth"[..], &[self.protocol_config.auth_bump]];

            let signer_seeds = &[&seeds[..]];

            let collateral_transfer_cpi_ctx = CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                collateral_transfer_cpi_accounts,
                signer_seeds,
            );

            transfer(collateral_transfer_cpi_ctx, self.position.collateral_amount)?;

            self.collateral_vault_config.collateral_amount = self
                .collateral_vault_config
                .collateral_amount
                .checked_sub(self.position.collateral_amount)
                .ok_or(ArithmeticError::ArithmeticOverflow)?;

            // Calculate current debt with accrued interest

            let accounts = Burn {
                mint: self.stable_mint.to_account_info(),
                from: self.user_stable_ata.to_account_info(),
                authority: self.user.to_account_info(),
            };

            let stable_burn_cpi_ctx =
                CpiContext::new(self.token_program.to_account_info(), accounts);
            burn(stable_burn_cpi_ctx, current_debt)?; // Use current_debt with accrued interest

            self.protocol_config.update_totals(-(current_debt as i64))?;
        } else {
            return err!(PositionError::InvalidLTV);
        }

        Ok(())
    }
}
