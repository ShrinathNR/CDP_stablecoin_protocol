use anchor_lang::prelude::*;
use anchor_spl::{
    token::{Mint, Token, TokenAccount},
    associated_token::AssociatedToken,
};

use crate::state::ProtocolConfig;

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
    stable_mint: Account<'info, Mint>,
    /// CHECK: This is an auth acc for the vault
    #[account(
        seeds = [b"auth"],
        bump
    )]
    auth: UncheckedAccount<'info>,
    #[account(
        init,
        payer = admin,
        seeds = [b"treasury", stable_mint.key().as_ref()],
        token::mint = stable_mint,
        token::authority = auth,
        bump
    )]
    treasury_vault: Account<'info, TokenAccount>,
    #[account(
        init,
        payer = admin,
        seeds = [b"stake_vault", stable_mint.key().as_ref()],
        token::mint = stable_mint,
        token::authority = auth,
        bump
    )]
    stake_vault: Account<'info, TokenAccount>,
    token_program: Program<'info, Token>,
    associated_token_program: Program<'info, AssociatedToken>,
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
        revenue_share_to_stability_pool: u16,
        bumps: &InitializeProtocolConfigBumps,
    ) -> Result<()> {
        self.protocol_config.set_inner(ProtocolConfig {
            admin: self.admin.key(),
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
            last_interest_rate_update_time: Clock::get()?.unix_timestamp,
            last_interest_rate: 0,
            total_debt: 0,
            total_stake_amount: 0,
            stake_points: 0,
            revenue_share_to_stability_pool,
        });

        Ok(())
    }
}
