use anchor_lang::prelude::*;
pub mod instructions;
pub use instructions::*;
pub mod state;
pub mod errors;

declare_id!("BvMWoXUSWLR4udmcmPX5M9DM89kCCJJvQWvxKE55uBGH");

#[program]
pub mod cdp_stablecoin_protocol {
    use super::*;

    pub fn initialize_protocol_config(
        ctx: Context<InitializeProtocolConfig>,
        protocol_fee: u16,
        redemption_fee: u16,
        mint_fee: u16,
        min_interest_rate: u16,
        max_interest_rate: u16,
    ) -> Result<()> {
        ctx.accounts.initialize_protocol_config(
            protocol_fee,
            redemption_fee,
            mint_fee,
            min_interest_rate,
            max_interest_rate,
            &ctx.bumps,
        )
    }

    pub fn initialize_collateral_vault(
        ctx: Context<InitializeCollateralVault>,
    ) -> Result<()> {
        ctx.accounts.initialize_collateral_vault()
    }

    pub fn liquidate(ctx: Context<Liquidate>) -> Result<()> {
        instructions::liquidate::liquidate(ctx)
    }
}
