// src/state.rs
use anchor_lang::prelude::*;

#[account]
pub struct ProtocolState {
    pub total_collateral: u64,
    pub total_stablecoin: u64,
    pub collateral_ratio: u64,
    pub liquidation_threshold: u64,
}

#[account]
pub struct UserPosition {
    pub owner: Pubkey,
    pub collateral_amount: u64,
    pub debt_amount: u64,
}