// src/errors.rs
use anchor_lang::prelude::*;

#[error_code]
pub enum CustomError {
    #[msg("Insufficient collateral")]
    InsufficientCollateral,
    #[msg("Liquidation threshold reached")]
    LiquidationThresholdReached,
    #[msg("Liquidation threshold not reached")]
    LiquidationThresholdNotReached,
}