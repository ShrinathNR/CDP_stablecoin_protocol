// src/instructions/liquidate.rs
use anchor_lang::prelude::*;
use crate::state::{ProtocolState, UserPosition};
use crate::errors::{LiquidationError, ArithmeticError};

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
}

pub fn liquidate(ctx: Context<Liquidate>, liquidation_type: LiquidationType) -> Result<()> {
    let user_position = &mut ctx.accounts.user_position;
    let protocol_state = &mut ctx.accounts.protocol_state;

    let collateral_ratio = calculate_collateral_ratio(user_position);

    if collateral_ratio >= protocol_state.liquidation_threshold {
        return Err(LiquidationError::LiquidationThresholdNotReached.into());
    }

    match liquidation_type {
        LiquidationType::Soft => soft_liquidate(user_position, protocol_state),
        LiquidationType::Hard => hard_liquidate(user_position, protocol_state),
    }?;

    update_state(user_position, protocol_state)
}

fn calculate_collateral_ratio(user_position: &UserPosition) -> u64 {
    (user_position.collateral_amount * 100) / user_position.debt_amount
}

fn soft_liquidate(user_position: &mut UserPosition, protocol_state: &mut ProtocolState) -> Result<()> {
    let target_ratio = protocol_state.liquidation_threshold + 5; // 5% buffer
    let collateral_to_liquidate = calculate_collateral_to_liquidate(user_position, target_ratio);

    user_position.collateral_amount -= collateral_to_liquidate;
    protocol_state.total_collateral -= collateral_to_liquidate;

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
        .checked_mul(5)
        .ok_or(ArithmeticError::ArithmeticOperationError)?
        .checked_div(100)
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

#[event]
pub struct HardLiquidationEvent {
    pub user: Pubkey,
    pub liquidated_amount: u64,
    pub debt_amount: u64,
    pub liquidator_receives: u64,
}

fn calculate_collateral_to_liquidate(user_position: &UserPosition, target_ratio: u64) -> u64 {
    let current_value = user_position.collateral_amount;
    let target_value = (user_position.debt_amount * target_ratio) / 100;
    if current_value > target_value {
        current_value - target_value
    } else {
        0
    }
}

fn update_state(user_position: &mut UserPosition, protocol_state: &mut ProtocolState) -> Result<()> {
    protocol_state.total_collateral = protocol_state.total_collateral
        .checked_sub(user_position.collateral_amount)
        .ok_or(LiquidationError::InsufficientCollateral)?;

    protocol_state.total_stablecoin = protocol_state.total_stablecoin
        .checked_sub(user_position.debt_amount)
        .ok_or(LiquidationError::InsufficientCollateral)?;

    if user_position.collateral_amount == 0 {
        user_position.debt_amount = 0;
    }

    Ok(())
}