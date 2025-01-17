use anchor_lang::prelude::*;
use pyth_solana_receiver_sdk::price_update::{get_feed_id_from_hex, PriceUpdateV2};
use crate::{
    state::ProtocolConfig,
    errors::ArithmeticError,
};

// Interest rate limits in basis points
const MIN_RATE_BPS: u16 = 100;   // 1% APR
const MAX_RATE_BPS: u16 = 3000;  // 30% APR

// Fixed point scale factors ?? NOT SURE PLS CHECK
const PRICE_SCALE: u64 = 1_000_000;  // 6 decimals for price (1.0 = 1_000_000)
const YEAR_IN_SECONDS: u64 = 365 * 24 * 60 * 60;  // 365 days * 24 hours * 60 minutes * 60 seconds

#[derive(Accounts)]
pub struct UpdateInterestRate<'info> {
    #[account(mut)]
    pub protocol_config: Account<'info, ProtocolConfig>,
    
    #[account(
        constraint = stablecoin_price_feed.key() == protocol_config.stablecoin_price_feed
    )]
    pub stablecoin_price_feed: Account<'info, PriceUpdateV2>,
}

impl<'info> UpdateInterestRate<'info> {
    pub fn update_interest_rate(&mut self) -> Result<()> {
        let current_timestamp = Clock::get()?.unix_timestamp;
        
        // Get current stablecoin price
        let stablecoin_price_feed = &mut self.stablecoin_price_feed;
        let maximum_age: u64 = 30;
        let feed_id: [u8; 32] = get_feed_id_from_hex(&self.protocol_config.stablecoin_price_feed.to_string())?;
        let price = stablecoin_price_feed.get_price_no_older_than(&Clock::get()?, maximum_age, &feed_id)?;
        
        // Convert price to fixed point with 6 decimals (following Pyth docs https://docs.pyth.network/price-feeds/best-practices)
        let stablecoin_price = (price.price as u64)
            .checked_mul(10_u64.pow(price.exponent as u32))
            .ok_or(ArithmeticError::ArithmeticOverflow)?;
        
        // Calculate price deviation from peg
        let price_deviation = PRICE_SCALE
            .checked_sub(stablecoin_price)
            .ok_or(ArithmeticError::ArithmeticOverflow)?;
        
        // TODO THIS IS USING FLOATING POINT STUFF. NO GOOD ?? HOW TO DO EXP FUNCTION IN ANCHOR ??
        let base_rate = self.protocol_config.base_rate as f64 / 10000.0;  // Convert from bps to float
        let sigma = self.protocol_config.sigma as f64 / 10000.0;  // Convert from bps to float
        let deviation = price_deviation as f64 / PRICE_SCALE as f64;
        
        let power = deviation / sigma;
        let new_rate = base_rate * power.exp();
        
        // Convert back to bps and clamp
        let new_rate_bps = (new_rate * 10000.0) as u16;
        let new_rate_bps = new_rate_bps.clamp(MIN_RATE_BPS, MAX_RATE_BPS);
        
        // Calculate time elapsed
        let time_elapsed = (current_timestamp - self.protocol_config.last_index_update) as u64;
        
        // multiplier = 1 + (rate * time / YEAR_IN_SECONDS)
        let interest_multiplier = (PRICE_SCALE as u128)
            .checked_add(
                (new_rate_bps as u128)
                    .checked_mul(time_elapsed as u128)
                    .ok_or(ArithmeticError::ArithmeticOverflow)?
                    .checked_mul(PRICE_SCALE as u128)
                    .ok_or(ArithmeticError::ArithmeticOverflow)?
                    .checked_div(10000)  // Convert from bps
                    .ok_or(ArithmeticError::ArithmeticOverflow)?
                    .checked_div(YEAR_IN_SECONDS as u128)
                    .ok_or(ArithmeticError::ArithmeticOverflow)?
            )
            .ok_or(ArithmeticError::ArithmeticOverflow)?;
            
        // Calculate new index
        let new_index = (self.protocol_config.interest_index as u128)
            .checked_mul(interest_multiplier)
            .ok_or(ArithmeticError::ArithmeticOverflow)?
            .checked_div(PRICE_SCALE as u128)
            .ok_or(ArithmeticError::ArithmeticOverflow)? as u64;
            
        self.protocol_config.interest_index = new_index;
        self.protocol_config.base_rate = new_rate_bps;
        self.protocol_config.last_index_update = current_timestamp;
        
        Ok(())
    }
} 