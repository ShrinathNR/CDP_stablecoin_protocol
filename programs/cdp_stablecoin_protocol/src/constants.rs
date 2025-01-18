use anchor_lang::prelude::*;

pub const MIN_INTEREST_RATE: u16 = 100;
pub const MAX_INTEREST_RATE: u16 = 10000;
pub const MAX_LTV: u16 = 8000;
pub const JITO_SOL: Pubkey = pubkey!("J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn");

// Interest rate limits in basis points
pub const MIN_RATE_BPS: u16 = 100;   // 1% APR
pub const MAX_RATE_BPS: u16 = 3000;  // 30% APR

// Fixed point scale factors
pub const INTEREST_SCALE: u64 = 1_000_000_000;   // 9 decimals for interest (1.0 = 1_000_000_000)
pub const BPS_SCALE: u64 = 10_000;               // Basis points scale (100% = 10000 bps)
pub const PRICE_SCALE: u64 = 1_000_000;  // 6 decimals for price (1.0 = 1_000_000)
pub const YEAR_IN_SECONDS: u64 = 365 * 24 * 60 * 60;  // 365 days * 24 hours * 60 minutes * 60 seconds