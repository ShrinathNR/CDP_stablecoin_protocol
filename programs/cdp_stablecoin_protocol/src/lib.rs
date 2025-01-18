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
        mint_fee: u16,
        base_rate: u16,
        sigma: u16,
        stablecoin_price_feed: String
    ) -> Result<()> {
        ctx.accounts.initialize_protocol_config(
            protocol_fee,
            redemption_fee,
            mint_fee,
            base_rate,
            sigma,
            stablecoin_price_feed,
            &ctx.bumps,
        )
    }

    pub fn initialize_collateral_vault(
        ctx: Context<InitializeCollateralVault>,
        collateral_price_feed: String,
    ) -> Result<()> {
        ctx.accounts.initialize_collateral_vault(collateral_price_feed, &ctx.bumps)
    }

    pub fn open_position(
        ctx: Context<OpenPosition>,
        auth_bump: u8,
        collateral_amount: u64,
        debt_amount: u64,
    ) -> Result<()> {
        ctx.accounts.open_position(
            auth_bump,
            collateral_amount,
            debt_amount,
        )
    }

    pub fn close_position(
        ctx: Context<ClosePosition>,
        auth_bump: u8,
    ) -> Result<()> {
        ctx.accounts.close_position(auth_bump)
    }

    pub fn update_interest_rate(
        ctx: Context<UpdateInterestRate>,
    ) -> Result<()> {
        ctx.accounts.update_interest_rate()
    }

    pub fn liquidate(ctx: Context<Liquidate>, liquidation_type: LiquidationType) -> Result<()> {
        instructions::liquidate::liquidate(ctx, liquidation_type)
    }
}
