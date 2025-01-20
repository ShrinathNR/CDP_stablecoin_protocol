// src/instructions/liquidate.rs
// Liquidation Module for CDP Stablecoin Protocol
// Handles both soft and hard liquidation strategies for under-collateralized positions

use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use pyth_solana_receiver_sdk::price_update::{get_feed_id_from_hex, PriceUpdateV2};

use crate::{
    constants::{PRICE_SCALE, LIQUIDATION_THRESHOLD, LIQUIDATION_BONUS, LIQUIDATION_SPREAD},
    errors::{ArithmeticError, LiquidationError},
    state::{ProtocolState, Position, CollateralConfig},
};

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum LiquidationType {
    Soft,
    Hard,
}

#[derive(Accounts)]
pub struct Liquidate<'info> {
    #[account(mut)]
    pub protocol_state: Account<'info, ProtocolState>,
    
    // Specific position being liquidated
    #[account(mut)]
    pub position: Account<'info, Position>,
    
    // Account initiating the liquidation
    #[account(mut)]
    pub liquidator: Signer<'info>,

    // Pyth price feed for current market price
    #[account(owner = pyth_solana_receiver_sdk::ID)]
    pub price_feed: Account<'info, PriceUpdateV2>,
    
    // Config for specific collateral type
    #[account(mut)]
    pub collateral_config: Account<'info, CollateralConfig>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

/// Main liquidation function
/// 
/// # Arguments
/// * `ctx` - Liquidation context containing all required accounts
/// * `liquidation_type` - Determines soft or hard liquidation strategy

pub fn liquidate(ctx: Context<Liquidate>, liquidation_type: LiquidationType) -> Result<()> {

    // Retrieve accounts
    let position = &mut ctx.accounts.position;
    let protocol_state = &mut ctx.accounts.protocol_state;
    let collateral_config = &ctx.accounts.collateral_config;

    // Fetch the current price from Pyth oracle
    let price_feed: &PriceUpdateV2 = ctx.accounts.price_feed.as_ref();
    let maximum_age = 30u64;
    
    // Retrieve collateral-specific price feed
    let feed_id: [u8; 32] = get_feed_id_from_hex(&collateral_config.collateral_price_feed)?;    
    let price = price_feed.get_price_no_older_than(&Clock::get()?, maximum_age, &feed_id)?;
   
    // Calculate the current collateral value
    let scaled_price = (price.price as u64)
        .checked_mul(10u64.pow(price.exponent.abs() as u32))
        .ok_or(ArithmeticError::ArithmeticOverflow)?;
    
    let collateral_value = scaled_price
        .checked_mul(position.collateral_amount)
        .ok_or(ArithmeticError::ArithmeticOverflow)?;
    
    
    // Calculate the current LTV ratio
    let ltv = position.debt_amount
        .checked_mul(PRICE_SCALE)
        .ok_or(ArithmeticError::ArithmeticOverflow)?
        .checked_div(collateral_value)
        .ok_or(ArithmeticError::ArithmeticOverflow)?;

    // Check if the position is eligible for liquidation
    if ltv <= LIQUIDATION_THRESHOLD {
        return Err(LiquidationError::LiquidationThresholdNotReached.into());
    }

    match liquidation_type {
        LiquidationType::Soft => soft_liquidate(position, protocol_state, price.price.try_into().unwrap())?,
        LiquidationType::Hard => hard_liquidate(position, protocol_state)?,
    }

    // Update protocol state after liquidation
    update_state(position, protocol_state)?;

    Ok(())
}

fn soft_liquidate(position: &mut Position, protocol_state: &mut ProtocolState, current_price: u64) -> Result<()> {
    let target_ratio = protocol_state.liquidation_threshold.checked_add(LIQUIDATION_SPREAD).ok_or(ArithmeticError::ArithmeticOverflow)?; // 5% buffer
    let collateral_to_liquidate = calculate_collateral_to_liquidate(position, target_ratio, current_price)?;

    position.collateral_amount = position.collateral_amount
    .checked_sub(collateral_to_liquidate)
    .ok_or(LiquidationError::InsufficientCollateral)?;

protocol_state.total_collateral = protocol_state.total_collateral
    .checked_sub(collateral_to_liquidate)
    .ok_or(LiquidationError::InsufficientCollateral)?;

    // bonding curve pricing and incentives for liquidators/stakers goes here
    Ok(())
}

/// Hard Liquidation: Completely closes an under-collateralized position
/// 
/// Transfers entire collateral, applies liquidation bonus, and closes position
fn hard_liquidate(position: &mut Position, protocol_state: &mut ProtocolState) -> Result<()> {
    
    // Capture current position state before closure    
    let liquidated_amount = position.collateral_amount;
    let debt_amount = position.debt_amount;

    // Zero out position
    position.collateral_amount = 0;
    position.debt_amount = 0;

    // Update protocol state
    protocol_state.total_collateral = protocol_state.total_collateral
        .checked_sub(liquidated_amount)
        .ok_or(ArithmeticError::ArithmeticOperationError)?;
    protocol_state.total_stablecoin = protocol_state.total_stablecoin
        .checked_sub(debt_amount)
        .ok_or(ArithmeticError::ArithmeticOperationError)?;

    // Calculate liquidation bonus (incentive for liquidators)
    let liquidation_bonus = liquidated_amount
        .checked_mul(LIQUIDATION_BONUS)
        .ok_or(ArithmeticError::ArithmeticOperationError)?
        .checked_div(10000)
        .ok_or(ArithmeticError::ArithmeticOperationError)?;

    // Total amount liquidator receives
    let liquidator_receives = liquidated_amount
        .checked_add(liquidation_bonus)
        .ok_or(ArithmeticError::ArithmeticOperationError)?;

    // TODO: logic to transfer liquidated collateral plus bonus to liquidator/stakers
    // TODO: Implement stablecoin burning mechanism

    // Emit event for hard liquidation tracking
    emit!(HardLiquidationEvent {
        user: position.user,
        liquidated_amount,
        debt_amount,
        liquidator_receives,
    });

    Ok(())
}

/// Calculates the amount of collateral to liquidate during a soft liquidation
///
/// # Arguments
/// * `position` - Reference to the position being liquidated
/// * `target_ratio` - Target collateralization ratio after liquidation
/// * `current_price` - Current market price of the collateral
fn calculate_collateral_to_liquidate(
    position: &Position,
    target_ratio: u64,
    current_price: u64,
) -> Result<u64> {
    let current_collateral_value = position.collateral_amount
        .checked_mul(current_price)
        .ok_or(ArithmeticError::ArithmeticOverflow)?
        .checked_div(PRICE_SCALE)
        .ok_or(ArithmeticError::ArithmeticOverflow)?;

    let target_collateral_value = position.debt_amount
        .checked_mul(target_ratio)
        .ok_or(ArithmeticError::ArithmeticOverflow)?
        .checked_div(100)
        .ok_or(ArithmeticError::ArithmeticOverflow)?;

      // Determine if liquidation is necessary
    if current_collateral_value < target_collateral_value {
        let value_to_liquidate = target_collateral_value
            .checked_sub(current_collateral_value)
            .ok_or(ArithmeticError::ArithmeticOverflow)?;

        let collateral_to_liquidate = value_to_liquidate
            .checked_mul(PRICE_SCALE)
            .ok_or(ArithmeticError::ArithmeticOverflow)?
            .checked_div(current_price)
            .ok_or(ArithmeticError::ArithmeticOverflow)?;

        Ok(collateral_to_liquidate)
    } else {
        Ok(0)
    }
}

fn update_state(position: &mut Position, protocol_state: &mut ProtocolState) -> Result<()> {
    
    // Reduce total protocol collateral by position's remaining collateral    
    protocol_state.total_collateral = protocol_state.total_collateral
        .checked_sub(position.collateral_amount)
        .ok_or(LiquidationError::InsufficientCollateral)?;

    // Reduce total stablecoin by position's remaining debt
    // Maintains protocol's stablecoin supply
    protocol_state.total_stablecoin = protocol_state.total_stablecoin
        .checked_sub(position.debt_amount)
        .ok_or(LiquidationError::InsufficientCollateral)?;

    if position.collateral_amount == 0 && position.debt_amount == 0 {        
        // TODO: Implement position closure mechanism
        // Potentially mark position as closed or remove from storage
    }

    Ok(())    
}

#[event]
pub struct HardLiquidationEvent {
    pub user: Pubkey,
    pub liquidated_amount: u64,
    pub debt_amount: u64,
    pub liquidator_receives: u64,
}