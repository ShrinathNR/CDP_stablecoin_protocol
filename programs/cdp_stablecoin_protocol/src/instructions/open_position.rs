use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{mint_to, transfer, Mint, MintTo, Token, TokenAccount, Transfer},
};
use pyth_solana_receiver_sdk::price_update::{get_feed_id_from_hex, PriceUpdateV2};

use crate::{
    constants::MAX_LTV,
    errors::{ArithmeticError, PositionError},
    state::{CollateralConfig, Position, ProtocolConfig},
};

#[derive(Accounts)]
pub struct OpenPosition<'info> {
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
    #[account(
        mut,
        seeds = [b"config"],
        bump = protocol_config.bump
    )]
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
        init_if_needed,
        payer = user,
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
        init,
        payer = user,
        space = 8 + Position::INIT_SPACE,
        seeds = [b"position", user.key().as_ref(), collateral_mint.key().as_ref()],
        bump
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

impl<'info> OpenPosition<'info> {
    pub fn open_position(&mut self, collateral_amount: u64, debt_amount: u64) -> Result<()> {
        // require!(MIN_INTEREST_RATE<= interest_rate && interest_rate <= MAX_INTEREST_RATE, PositionError::InvalidInterestRate);
        // get_price_no_older_than will fail if the price update is more than 30 seconds old
        let price_feed = &mut self.price_feed;
        // let maximum_age: u64 = 30;
        let feed_id: [u8; 32] =
            get_feed_id_from_hex(&self.collateral_vault_config.collateral_price_feed)?;

        // msg!("feed_id is {}", feed_id);
        // let price = price_feed.get_price_no_older_than(&Clock::get()?, maximum_age, &feed_id)?;
        let price = price_feed.get_price_unchecked(&feed_id)?;

        msg!("The price is ({} ± {}) * 10^{}", price.price, price.conf, price.exponent);

        let collateral_value = (price.price as u128)
            .checked_mul(10_u128.pow(price.exponent.abs() as u32))
            .ok_or(ArithmeticError::ArithmeticOverflow)?
            .checked_mul(collateral_amount as u128)
            .ok_or(ArithmeticError::ArithmeticOverflow)?;

        let ltv = (debt_amount as u128)
            .checked_mul(10000)
            .ok_or(ArithmeticError::ArithmeticOverflow)?
            .checked_div(collateral_value as u128)
            .ok_or(ArithmeticError::ArithmeticOverflow)? as u16;

        require!(ltv <= MAX_LTV, PositionError::InvalidLTV);

        self.position.set_inner(Position {
            user: self.user.key(),
            collateral_amount,
            debt_amount,
            prev_cumulative_interest_rate: self.protocol_config.cumulative_interest_rate,
        });

        let collateral_transfer_cpi_accounts = Transfer {
            from: self.user_ata.to_account_info(),
            to: self.collateral_vault.to_account_info(),
            authority: self.user.to_account_info(),
        };

        let collateral_transfer_cpi_ctx = CpiContext::new(
            self.token_program.to_account_info(),
            collateral_transfer_cpi_accounts,
        );

        transfer(collateral_transfer_cpi_ctx, collateral_amount)?;

        self.collateral_vault_config.amount = self
            .collateral_vault_config
            .amount
            .checked_add(collateral_amount)
            .ok_or(ArithmeticError::ArithmeticOverflow)?;

        self.protocol_config.update_totals(debt_amount as i64)?;

        let accounts = MintTo {
            mint: self.stable_mint.to_account_info(),
            to: self.user_stable_ata.to_account_info(),
            authority: self.auth.to_account_info(),
        };

        let seeds = &[&b"auth"[..], &[self.protocol_config.auth_bump]];

        let signer_seeds = &[&seeds[..]];

        let stable_mint_cpi_ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            accounts,
            signer_seeds,
        );

        mint_to(stable_mint_cpi_ctx, debt_amount)?;

        Ok(())
    }
}
