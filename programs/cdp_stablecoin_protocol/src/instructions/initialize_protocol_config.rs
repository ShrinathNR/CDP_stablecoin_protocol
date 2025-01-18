use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token};

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
        mint::decimals = 6,
        mint::authority = auth,
        mint::token_program = token_program
    )]
    stable_mint: Account<'info, Mint>,
    /// CHECK: This is an auth acc for the vault
    #[account(
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
        global_interest_rate: u16,
        bumps: &InitializeProtocolConfigBumps,
    ) -> Result<()> {
        self.protocol_config.set_inner(ProtocolConfig {
            stable_mint: self.stable_mint.key(),
            protocol_fee,
            redemption_fee,
            global_interest_rate,
            mint_fee,
            auth_bump: bumps.auth,
        });

        Ok(())
    }
}
