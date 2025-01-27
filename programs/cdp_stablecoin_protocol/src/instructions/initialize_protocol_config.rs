use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token};

use crate::{constants::BPS_SCALE, state::ProtocolConfig};

#[derive(Accounts)]
pub struct InitializeProtocolConfig<'info> {
    #[account(mut)]
    admin: Signer<'info>,
    #[account(
        init,
        space = 8 + ProtocolConfig::INIT_SPACE,
        payer = admin,
        seeds = [b"config"],
        bump
    )]
    protocol_config: Account<'info, ProtocolConfig>,
    #[account(
        init,
        payer = admin,
        seeds = [b"stable"],
        mint::decimals = 6,
        mint::authority = auth,
        mint::token_program = token_program,
        bump,
    )]
    stable_mint: Box<Account<'info, Mint>>,
    /// CHECK: This is an auth acc for the vault
    #[account(
        mut,
        seeds = [b"auth"],
        bump
    )]
    auth: UncheckedAccount<'info>,
    token_program: Program<'info, Token>,
    system_program: Program<'info, System>,
}

impl<'info> InitializeProtocolConfig<'info> {
    pub fn initialize_protocol_config(
        &mut self,
        protocol_fee: u16,
        redemption_fee: u16,
        mint_fee: u16,
        base_rate: u16,
        sigma: u16,
        stablecoin_price_feed: String,
        bumps: &InitializeProtocolConfigBumps,
    ) -> Result<()> {
        self.protocol_config.set_inner(ProtocolConfig {
            stable_mint: self.stable_mint.key(),
            protocol_fee,
            redemption_fee,
            mint_fee,
            base_rate,
            sigma,
            auth_bump: bumps.auth,
            bump: bumps.protocol_config,
            cumulative_interest_rate: ProtocolConfig::INITIAL_CUMULATIVE_RATE,
            stablecoin_price_feed,
            last_interest_rate_update: Clock::get()?.unix_timestamp,
            deposit_depletion_factor: BPS_SCALE,
            total_debt: 0,
            total_stake_amount: 0,
        });

        Ok(())
    }
}
