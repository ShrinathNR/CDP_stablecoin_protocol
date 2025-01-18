// src/instructions/liquidate.rs
use anchor_lang::prelude::*;
use crate::state::{ProtocolState, UserPosition};
use crate::errors::LiquidationError;

#[derive(Accounts)]
pub struct Liquidate<'info> {
    #[account(mut)]
    pub protocol_config: Account<'info, ProtocolState>,
    #[account(mut)]
    pub user_position: Account<'info, UserPosition>,
    #[account(mut)]
    pub liquidator: Signer<'info>,
}

pub fn liquidate(ctx: Context<Liquidate>) -> Result<()> {
    let user_position = &mut ctx.accounts.user_position;
    
    // Implement liquidation logic based on collateral ratio and thresholds
    if user_position.debt_amount > (user_position.collateral_amount * ctx.accounts.protocol_config.liquidation_threshold / 100) {
        // Liquidate a portion of the collateral or all based on your strategy
        // For example:
        let liquidated_amount = user_position.collateral_amount; // Simplified logic for demo purposes
        
        user_position.collateral_amount = 0; // Reset user's collateral after liquidation
        
        // Update total collateral in protocol state
        let protocol_state = &mut ctx.accounts.protocol_config;
        protocol_state.total_collateral = protocol_state.total_collateral.checked_sub(liquidated_amount)
            .ok_or(LiquidationError::InsufficientCollateral)?;
        
        Ok(())
    } else {
        Err(LiquidationError::LiquidationThresholdNotReached.into())
    }
}