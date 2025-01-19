// src/instructions/liquidate.rs
use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use pyth_solana_receiver_sdk::price_update::{get_feed_id_from_hex, PriceUpdateV2};


use crate::{
    constants::{PRICE_SCALE, LIQUIDATION_THRESHOLD, LIQUIDATION_BONUS, LIQUIDATION_SPREAD},
    errors::{ArithmeticError, LiquidationError},
    state::{ProtocolState, UserPosition},
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
    #[account(mut)]
    pub user_position: Account<'info, UserPosition>,
    
    #[account(mut)]
    pub liquidator: Signer<'info>,

    #[account(owner = pyth_solana_receiver_sdk::ID)]
    pub price_feed: Account<'info, PriceUpdateV2>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

pub fn liquidate(ctx: Context<Liquidate>, liquidation_type: LiquidationType) -> Result<()> {
    let user_position = &mut ctx.accounts.user_position;
    let protocol_state = &mut ctx.accounts.protocol_state;

    // Fetch the current price from Pyth oracle
    let price_feed: &PriceUpdateV2 = ctx.accounts.price_feed.as_ref();
    let maximum_age = 30u64;
    let feed_id: [u8; 32]  = get_feed_id_from_hex(&protocol_state.price_feed_id)?;    
    let price = price_feed.get_price_no_older_than(&Clock::get()?, maximum_age, &feed_id)?;
   
    // Calculate the current collateral value
        let collateral_value = user_position.collateral_amount
        .checked_mul(price.price.try_into().unwrap())
        .ok_or(ArithmeticError::ArithmeticOverflow)?
        .checked_div(10u64.pow(price.exponent as u32))
        .ok_or(ArithmeticError::ArithmeticOverflow)?;

    // Calculate the current LTV
    let ltv = user_position.debt_amount
        .checked_mul(PRICE_SCALE)
        .ok_or(ArithmeticError::ArithmeticOverflow)?
        .checked_div(collateral_value)
        .ok_or(ArithmeticError::ArithmeticOverflow)?;

    // Check if the position is eligible for liquidation
    if ltv <= LIQUIDATION_THRESHOLD {
        return Err(LiquidationError::LiquidationThresholdNotReached.into());
    }

    match liquidation_type {
        LiquidationType::Soft => soft_liquidate(user_position, protocol_state, price.price.try_into().unwrap())?,
        LiquidationType::Hard => hard_liquidate(user_position, protocol_state)?,
    }

    update_state(user_position, protocol_state)?;

    Ok(())
}

fn soft_liquidate(user_position: &mut UserPosition, protocol_state: &mut ProtocolState, current_price: u64) -> Result<()> {
    let target_ratio = protocol_state.liquidation_threshold.checked_add(LIQUIDATION_SPREAD).ok_or(ArithmeticError::ArithmeticOverflow)?; // 5% buffer
    let collateral_to_liquidate = calculate_collateral_to_liquidate(user_position, target_ratio, current_price)?;

    user_position.collateral_amount = user_position.collateral_amount
    .checked_sub(collateral_to_liquidate)
    .ok_or(LiquidationError::InsufficientCollateral)?;

protocol_state.total_collateral = protocol_state.total_collateral
    .checked_sub(collateral_to_liquidate)
    .ok_or(LiquidationError::InsufficientCollateral)?;

    // bonding curve pricing and incentives for liquidators/stakers goes here
    Ok(())
}

fn hard_liquidate(user_position: &mut UserPosition, protocol_state: &mut ProtocolState) -> Result<()> {
    let liquidated_amount = user_position.collateral_amount;
    let debt_amount = user_position.debt_amount;

    user_position.collateral_amount = 0;
    user_position.debt_amount = 0;

    protocol_state.total_collateral = protocol_state.total_collateral
        .checked_sub(liquidated_amount)
        .ok_or(ArithmeticError::ArithmeticOperationError)?;
    protocol_state.total_stablecoin = protocol_state.total_stablecoin
        .checked_sub(debt_amount)
        .ok_or(ArithmeticError::ArithmeticOperationError)?;

    let liquidation_bonus = liquidated_amount
        .checked_mul(LIQUIDATION_BONUS)
        .ok_or(ArithmeticError::ArithmeticOperationError)?
        .checked_div(10000)
        .ok_or(ArithmeticError::ArithmeticOperationError)?;

    let liquidator_receives = liquidated_amount
        .checked_add(liquidation_bonus)
        .ok_or(ArithmeticError::ArithmeticOperationError)?;

    // logic to transfer liquidated collateral plus bonus to liquidator/stakers
    // and burn corresponding stablecoins

    emit!(HardLiquidationEvent {
        user: user_position.owner,
        liquidated_amount,
        debt_amount,
        liquidator_receives,
    });

    Ok(())
}

fn calculate_collateral_to_liquidate(
    user_position: &UserPosition,
    target_ratio: u64,
    current_price: u64,
) -> Result<u64> {
    let current_collateral_value = user_position.collateral_amount
        .checked_mul(current_price)
        .ok_or(ArithmeticError::ArithmeticOverflow)?
        .checked_div(PRICE_SCALE)
        .ok_or(ArithmeticError::ArithmeticOverflow)?;

    let target_collateral_value = user_position.debt_amount
        .checked_mul(target_ratio)
        .ok_or(ArithmeticError::ArithmeticOverflow)?
        .checked_div(100)
        .ok_or(ArithmeticError::ArithmeticOverflow)?;

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


fn update_state(user_position: &mut UserPosition, protocol_state: &mut ProtocolState) -> Result<()> {
    protocol_state.total_collateral = protocol_state.total_collateral
        .checked_sub(user_position.collateral_amount)
        .ok_or(LiquidationError::InsufficientCollateral)?;

    protocol_state.total_stablecoin = protocol_state.total_stablecoin
        .checked_sub(user_position.debt_amount)
        .ok_or(LiquidationError::InsufficientCollateral)?;

    if user_position.collateral_amount == 0 && user_position.debt_amount == 0 {
        // Close the position if it's empty
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