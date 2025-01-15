use std::str::FromStr;

use anchor_lang::prelude::*;

pub const MIN_INTEREST_RATE: u16 = 100;
pub const MAX_INTEREST_RATE: u16 = 10000;
pub const MAX_LTV: u16 = 100;
pub const MIN_LTV: u16 = 8000;
pub const JITO_SOL: Pubkey = pubkey!("J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn");