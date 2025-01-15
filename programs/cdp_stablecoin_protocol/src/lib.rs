use anchor_lang::prelude::*;
pub mod instructions;
pub use instructions::*;
pub mod constants;
pub mod errors;
pub mod state;

declare_id!("BvMWoXUSWLR4udmcmPX5M9DM89kCCJJvQWvxKE55uBGH");

#[program]
pub mod cdp_stablecoin_protocol {
    use super::*;

    pub fn initialize_protocol_config(
        ctx: Context<InitializeProtocolConfig>,
        protocol_fee: u16,
        redemption_fee: u16,
        global_interest_rate: u16,
        mint_fee: u16,
    ) -> Result<()> {
        ctx.accounts.initialize_protocol_config(
            protocol_fee,
            redemption_fee,
            global_interest_rate,
            mint_fee,
            &ctx.bumps,
        )
    }

    pub fn initialize_collateral_vault(ctx: Context<InitializeCollateralVault>, price_feed: String) -> Result<()> {
        ctx.accounts.initialize_collateral_vault(price_feed, &ctx.bumps)
    }
}
