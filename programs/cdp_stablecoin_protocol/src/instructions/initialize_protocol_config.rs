use anchor_lang::prelude::*;

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
    /// CHECK: This is an auth acc for the vault
    #[account(
        seeds = [b"auth"],
        bump
    )]
    auth: UncheckedAccount<'info>,
    system_program: Program<'info, System>,
}

impl<'info> InitializeProtocolConfig<'info> {
    pub fn initialize_protocol_config(
        &mut self,
        protocol_fee: u16,
        redemption_fee: u16,
        mint_fee: u16,
        min_interest_rate: u16,
        max_interest_rate: u16,
        bumps: &InitializeProtocolConfigBumps,
    ) -> Result<()> {
        self.protocol_config.set_inner(ProtocolConfig {
            protocol_fee,
            redemption_fee,
            mint_fee,
            min_interest_rate,
            max_interest_rate,
            auth_bump: bumps.auth,
        });

        Ok(())
    }
}
