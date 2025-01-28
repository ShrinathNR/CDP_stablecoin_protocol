use crate::{
    constants::{BPS_SCALE, INTEREST_SCALE, MAX_INTEREST_RATE, MIN_INTEREST_RATE, YEAR_IN_SECONDS},
    errors::ArithmeticError,
    state::ProtocolConfig,
};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};
use pyth_solana_receiver_sdk::price_update::{get_feed_id_from_hex, PriceUpdateV2};

#[derive(Accounts)]
pub struct UpdateInterestRate<'info> {
    #[account(mut)]
    user: Signer<'info>,
    collateral_mint: Account<'info, Mint>,

    #[account(
        mut,
        seeds = [b"config"],
        bump = protocol_config.bump
    )]
    pub protocol_config: Account<'info, ProtocolConfig>,

    #[account(owner = pyth_solana_receiver_sdk::ID)]
    pub stablecoin_price_feed: Account<'info, PriceUpdateV2>,
    #[account(
        mut,
        address = protocol_config.stable_mint,
        mint::decimals = 6,
        mint::authority = auth,
    )]
    stable_mint: Account<'info, Mint>,
    #[account(
        mut,
        seeds = [b"treasury", stable_mint.key().as_ref()],
        token::mint = stable_mint,
        token::authority = auth,
        bump
    )]
    pub treasury_vault: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        seeds = [b"stake_vault", stable_mint.key().as_ref(), collateral_mint.key().as_ref()],
        token::mint = stable_mint,
        token::authority = auth,
        bump
    )]
    pub stake_vault: Box<Account<'info, TokenAccount>>,
    /// CHECK: This is an auth acc for the vault
    #[account(
        mut,
        seeds = [b"auth"],
        bump = protocol_config.auth_bump
    )]
    auth: UncheckedAccount<'info>,
    token_program: Program<'info, Token>,
    associated_token_program: Program<'info, AssociatedToken>,
    system_program: Program<'info, System>,
}

impl<'info> UpdateInterestRate<'info> {
    // Taylor series approximation of e^x up to 3rd order
    fn exponential_approximation(x: i128, scale: u128) -> Result<u128> {
        // first term: 1
        let mut result = scale as i128;

        // second term: x
        result = result
            .checked_add(x)
            .ok_or(ArithmeticError::ArithmeticOverflow)?;

        // third term: x^2/2!
        let x_squared = x
            .checked_mul(x)
            .ok_or(ArithmeticError::ArithmeticOverflow)?;

        let x_squared_scaled = x_squared
            .checked_div(scale as i128)
            .ok_or(ArithmeticError::ArithmeticOverflow)?;

        result = result
            .checked_add(x_squared_scaled / 2)
            .ok_or(ArithmeticError::ArithmeticOverflow)?;

        // fourth term: x^3/3!
        let x_cubed = x_squared_scaled
            .checked_mul(x)
            .ok_or(ArithmeticError::ArithmeticOverflow)?;

        let x_cubed_scaled: i128 = x_cubed
            .checked_div(scale as i128)
            .ok_or(ArithmeticError::ArithmeticOverflow)?;

        let x_cubed_final = x_cubed_scaled / 6;

        result = result
            .checked_add(x_cubed_final)
            .ok_or(ArithmeticError::ArithmeticOverflow)?;

        Ok(result as u128)
    }

    // Calculate interest rate based on price deviation from peg
    fn calculate_interest_rate(
        stablecoin_price: i64,
        stablecoin_exponent: i32,
        base_rate_bps: u16,
        sigma_bps: u16,
    ) -> Result<u128> {
        // Calculate price deviation from peg
        let peg = 10_i128.pow(stablecoin_exponent.abs() as u32);

        let price_deviation: i128 = peg
            .checked_sub(stablecoin_price as i128)
            .ok_or(ArithmeticError::ArithmeticOverflow)?;

        // exponent calculation
        let bps_scale_i128 = BPS_SCALE as i128;

        let mut x = price_deviation
            .checked_mul(bps_scale_i128)
            .ok_or(ArithmeticError::ArithmeticOverflow)?;

        x = x / sigma_bps as i128;

        // convert exponent from price scale to interest scale
        x = x
            .checked_mul(INTEREST_SCALE as i128)
            .ok_or(ArithmeticError::ArithmeticOverflow)?;

        x = x / peg;

        // Calculate rate = base_rate * e^x
        let exp_result = Self::exponential_approximation(x, INTEREST_SCALE)?;

        let interest_rate: u128 = exp_result
            .checked_mul(base_rate_bps as u128)
            .ok_or(ArithmeticError::ArithmeticOverflow)?
            .checked_div(BPS_SCALE as u128)
            .ok_or(ArithmeticError::ArithmeticOverflow)?;

        Ok(interest_rate.clamp(MIN_INTEREST_RATE, MAX_INTEREST_RATE))
    }

    // Calculate compound interest using Taylor series for efficiency for (1+r)^t
    fn compound_interest(interest_rate: u128, time_elapsed: u128) -> Result<u128> {
        // Handle small periods (t <= 4) directly for better precision
        match time_elapsed {
            0 => return Ok(INTEREST_SCALE),
            1 => return Ok(INTEREST_SCALE + interest_rate),
            2 => {
                return Ok(
                    (INTEREST_SCALE + interest_rate) * (INTEREST_SCALE + interest_rate)
                        / INTEREST_SCALE,
                )
            }
            3 => {
                return Ok((INTEREST_SCALE + interest_rate)
                    * (INTEREST_SCALE + interest_rate)
                    * (INTEREST_SCALE + interest_rate)
                    / (INTEREST_SCALE * INTEREST_SCALE))
            }
            4 => {
                let pow_two = (INTEREST_SCALE + interest_rate) * (INTEREST_SCALE + interest_rate)
                    / INTEREST_SCALE;
                return Ok((pow_two * pow_two) / INTEREST_SCALE);
            }
            _ => (),
        }

        // For larger periods use Taylor expansion
        let exp = time_elapsed;
        let exp_minus_one = time_elapsed - 1;
        let exp_minus_two = time_elapsed - 2;

        let base = interest_rate;
        let base_pow_two = interest_rate * interest_rate / INTEREST_SCALE;
        let base_pow_three = base_pow_two * base / INTEREST_SCALE;

        let first_term = INTEREST_SCALE; // denoting 1
        let second_term = exp * base;
        let third_term = exp * exp_minus_one * base_pow_two / 2;
        let fourth_term = exp * exp_minus_one * exp_minus_two * base_pow_three / 6;

        Ok(first_term + second_term + third_term + fourth_term)
    }

    // Main function to update protocol interest rate
    pub fn update_interest_rate(&mut self) -> Result<()> {
        // Calculate time elapsed
        let current_timestamp = Clock::get()?.unix_timestamp;
        let time_elapsed =
            (current_timestamp - self.protocol_config.last_interest_rate_update_time) as u64;

        match time_elapsed {
            0 => return Ok(()),
            _ => {}
        };

        // Get current stablecoin price
        let price_feed = &self.stablecoin_price_feed;

        let feed_id: [u8; 32] = get_feed_id_from_hex(&self.protocol_config.stablecoin_price_feed)?;

        let stablecoin_price = price_feed.get_price_unchecked(&feed_id)?;

        // Calculate yearly interest rate

        let new_interest_rate_yearly = Self::calculate_interest_rate(
            stablecoin_price.price,
            stablecoin_price.exponent,
            self.protocol_config.base_rate,
            self.protocol_config.sigma,
        )?;
        self.protocol_config.last_interest_rate = new_interest_rate_yearly;

        // Calculate compound interest
        let per_second_rate = new_interest_rate_yearly / YEAR_IN_SECONDS as u128;

        let compounded_interest_rate =
            Self::compound_interest(per_second_rate, time_elapsed as u128)?;
        
        // Calculate interest revenue
        let old_total_debt = self.protocol_config.total_debt as u128;
        let new_total_debt = old_total_debt
            .checked_mul(compounded_interest_rate)
            .ok_or(ArithmeticError::ArithmeticOverflow)?
            / INTEREST_SCALE;
        let interest_revenue = new_total_debt
            .checked_sub(old_total_debt)
            .ok_or(ArithmeticError::ArithmeticOverflow)? as u64;
        
        // Distribute revenue
        self.protocol_config.distribute_revenue(
            interest_revenue,
            &self.stable_mint,
            &self.treasury_vault,
            &self.stake_vault,
            &self.auth,
            &self.token_program,
        )?;

        // Update protocol state
        self.protocol_config.cumulative_interest_rate =
            (self.protocol_config.cumulative_interest_rate)
                .checked_mul(compounded_interest_rate)
                .ok_or(ArithmeticError::ArithmeticOverflow)?
                / INTEREST_SCALE;

        self.protocol_config.total_debt = new_total_debt;
        self.protocol_config.last_interest_rate_update_time  = current_timestamp;
        
        Ok(())
    }
}
