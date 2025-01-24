use anchor_lang::prelude::*;

pub const MAX_LTV: u16 = 8000;
pub const JITO_SOL: Pubkey = pubkey!("J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn");

// Fixed point scale factors
pub const YEAR_IN_SECONDS: u64 = 365 * 24 * 60 * 60; // Seconds in a year
pub const PRICE_SCALE: u64 = 1_000_000; // 6 decimals for price
pub const BPS_SCALE: u16 = 10_000; // Basis points (100% = 10000)
pub const INTEREST_SCALE: u128 = 1_000_000_000_000_000_000; // 1e18 for interest
pub const MIN_INTEREST_RATE: u128 = INTEREST_SCALE / 100; // 1% APR
pub const MAX_INTEREST_RATE: u128 = INTEREST_SCALE * 30 / 100; // 30% APR
