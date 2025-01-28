use anchor_lang::{prelude::*, solana_program::native_token::LAMPORTS_PER_SOL};
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{burn, transfer, Burn, Mint, Token, TokenAccount, Transfer},
};
use pyth_solana_receiver_sdk::price_update::{get_feed_id_from_hex, PriceUpdateV2};

use crate::{
    constants::{BPS_SCALE, MAX_LTV},
    errors::{ArithmeticError, PositionError},
    state::{CollateralConfig, Position, ProtocolConfig},
};

#[derive(Accounts)]
pub struct LiquidatePosition<'info> {
    #[account(mut)]
    liquidator: Signer<'info>,

    #[account(mut)]
    user: SystemAccount<'info>,

    collateral_mint: Account<'info, Mint>,

    #[account(
        mut,
        address = protocol_config.stable_mint
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

    #[account(
        mut,
        seeds = [b"liquidation_rewards_vault", collateral_mint.key().as_ref()],
        token::mint = collateral_mint,
        token::authority = auth,
        bump
    )]
    liquidation_rewards_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [b"stake_vault", stable_mint.key().as_ref(), collateral_mint.key().as_ref()],
        token::mint = stable_mint,
        token::authority = auth,
        bump
    )]
    pub stake_vault: Account<'info, TokenAccount>,

    token_program: Program<'info, Token>,
    associated_token_program: Program<'info, AssociatedToken>,
    system_program: Program<'info, System>,
}

impl<'info> LiquidatePosition<'info> {
    pub fn liquidate_position(&mut self) -> Result<()> {
        self.collateral_vault_config.claim_pending_rewards(
            &self.protocol_config,
            &self.stable_mint,
            &self.stake_vault,
            &self.auth,
            &self.token_program,
            self.protocol_config.auth_bump,
        )?;
        
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

        require!(MAX_LTV <= ltv, PositionError::InvalidLTV);

        let collateral_transfer_cpi_accounts = Transfer {
            from: self.collateral_vault.to_account_info(),
            to: self.liquidation_rewards_vault.to_account_info(),
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

        self.collateral_vault_config.stability_pool_rewards_amount = self
            .collateral_vault_config
            .stability_pool_rewards_amount
            .checked_add(self.position.collateral_amount)
            .ok_or(ArithmeticError::ArithmeticOverflow)?;

        self.collateral_vault_config.gain_summation = self
            .collateral_vault_config
            .gain_summation
            .checked_add(
                (self.position.collateral_amount as u128)
                    .checked_mul(self.collateral_vault_config.deposit_depletion_factor as u128)
                    .ok_or(ArithmeticError::ArithmeticOverflow)?
                    .checked_div(BPS_SCALE as u128)
                    .ok_or(ArithmeticError::ArithmeticOverflow)?
                    .checked_div(self.collateral_vault_config.total_stake_amount)
                    .ok_or(ArithmeticError::ArithmeticOverflow)?,
            )
            .ok_or(ArithmeticError::ArithmeticOverflow)?;

        self.collateral_vault_config.deposit_depletion_factor =
            self.collateral_vault_config.deposit_depletion_factor
                .checked_mul(
                    (self.collateral_vault_config.total_stake_amount)
                        .checked_sub(current_debt as u128)
                        .ok_or(ArithmeticError::ArithmeticOverflow)?,
                )
                .ok_or(ArithmeticError::ArithmeticOverflow)?
                .checked_div(self.collateral_vault_config.total_stake_amount)
                .ok_or(ArithmeticError::ArithmeticOverflow)?;
        
        self.collateral_vault_config.compound_total_debt(&self.protocol_config)?;
        self.collateral_vault_config.total_stake_amount = self
            .collateral_vault_config
            .total_stake_amount
            .checked_sub(current_debt as u128)
            .ok_or(ArithmeticError::ArithmeticOverflow)?;

        // Calculate current debt with accrued interest

        let accounts = Burn {
            mint: self.stable_mint.to_account_info(),
            from: self.stake_vault.to_account_info(),
            authority: self.auth.to_account_info(),
        };

        let signer_seeds = &[&b"auth"[..], &[self.protocol_config.auth_bump]];
        let binding = [&signer_seeds[..]];

        let stable_burn_cpi_ctx =
            CpiContext::new_with_signer(self.token_program.to_account_info(), accounts, &binding);

        burn(stable_burn_cpi_ctx, current_debt)?; // Use current_debt with accrued interest

        self.protocol_config.update_totals(-(current_debt as i64))?;

        Ok(())
    }
}
