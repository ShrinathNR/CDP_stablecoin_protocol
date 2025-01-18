use anchor_lang::prelude::*;
use pyth_solana_receiver_sdk::price_update::{get_feed_id_from_hex, PriceUpdateV2};
use crate::{
    state::ProtocolConfig,
    errors::ArithmeticError,
    constants::{MIN_RATE_BPS, MAX_RATE_BPS, PRICE_SCALE, INTEREST_SCALE, BPS_SCALE, YEAR_IN_SECONDS}
};

#[derive(Accounts)]
pub struct UpdateInterestRate<'info> {
    #[account(mut)]
    pub protocol_config: Account<'info, ProtocolConfig>,
    
    // #[account(
    //     // owner = 
    // )]
    pub price_feed: Account<'info, PriceUpdateV2>,
}

impl<'info> UpdateInterestRate<'info> {
    pub fn update_interest_rate(&mut self) -> Result<()> {
        let current_timestamp = Clock::get()?.unix_timestamp;
        
        // Get current stablecoin price
        let price_feed = &mut self.price_feed;
        let maximum_age: u64 = 30;
        let feed_id: [u8; 32] = get_feed_id_from_hex(&self.protocol_config.stablecoin_price_feed)?;
        let price = price_feed.get_price_no_older_than(&Clock::get()?, maximum_age, &feed_id)?;
        
        // Convert price to fixed point with 6 decimals (following Pyth docs https://docs.pyth.network/price-feeds/best-practices)
        let stablecoin_price = (price.price as u64)
            .checked_mul(10_u64.pow(price.exponent as u32))
            .ok_or(ArithmeticError::ArithmeticOverflow)?;
        
        // Calculate price deviation from peg
        let price_deviation = PRICE_SCALE
            .checked_sub(stablecoin_price)
            .ok_or(ArithmeticError::ArithmeticOverflow)?;
        
        // TODO THIS IS USING FLOATING POINT STUFF. NO GOOD ?? HOW TO DO EXP FUNCTION IN ANCHOR ??
        let base_rate = self.protocol_config.base_rate as f64 / BPS_SCALE as f64;  // Convert from bps to float
        let sigma = self.protocol_config.sigma as f64 / BPS_SCALE as f64;  // Convert from bps to float
        let deviation = price_deviation as f64 / PRICE_SCALE as f64;
        
        let power = deviation / sigma;
        let new_rate = base_rate * power.exp();
        
        // Convert back to bps and clamp
        let new_rate_bps = (new_rate * BPS_SCALE as f64) as u16;
        let new_rate_bps = new_rate_bps.clamp(MIN_RATE_BPS, MAX_RATE_BPS);
        
        // Calculate time elapsed
        let time_elapsed = (current_timestamp - self.protocol_config.last_index_update) as u64;
        
        // multiplier = 1 + (rate * time / YEAR_IN_SECONDS)
        let interest_multiplier = (INTEREST_SCALE as u128)
            .checked_add(
                (new_rate_bps as u128)
                    .checked_mul(time_elapsed as u128)
                    .ok_or(ArithmeticError::ArithmeticOverflow)?
                    .checked_mul(INTEREST_SCALE as u128)
                    .ok_or(ArithmeticError::ArithmeticOverflow)?
                    .checked_div(BPS_SCALE as u128)  // Convert from bps
                    .ok_or(ArithmeticError::ArithmeticOverflow)?
                    .checked_div(YEAR_IN_SECONDS as u128)
                    .ok_or(ArithmeticError::ArithmeticOverflow)?
            )
            .ok_or(ArithmeticError::ArithmeticOverflow)?;
            
        // Calculate new index
        let new_index = (self.protocol_config.interest_index as u128)
            .checked_mul(interest_multiplier)
            .ok_or(ArithmeticError::ArithmeticOverflow)?
            .checked_div(INTEREST_SCALE as u128)
            .ok_or(ArithmeticError::ArithmeticOverflow)? as u64;
            
        // Update total debt
        self.protocol_config.total_debt = (self.protocol_config.total_debt as u128)
            .checked_mul(interest_multiplier)
            .ok_or(ArithmeticError::ArithmeticOverflow)?
            .checked_div(INTEREST_SCALE as u128)
            .ok_or(ArithmeticError::ArithmeticOverflow)? as u64;
            
        // Update state
        self.protocol_config.interest_index = new_index;
        self.protocol_config.base_rate = new_rate_bps;
        self.protocol_config.last_index_update = current_timestamp;
        
        Ok(())
    }
} 