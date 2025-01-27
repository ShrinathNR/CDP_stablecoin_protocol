use anchor_lang::prelude::*;
pub mod instructions;
pub use instructions::*;
pub mod constants;
pub mod errors;
pub mod state;

declare_id!("3xYBiBikqqFRLKJbctJ1ByaKr1cHGbBdhj9BSUTuTECa");

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
        stablecoin_price_feed: String,
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
        ctx.accounts
            .initialize_collateral_vault(collateral_price_feed, &ctx.bumps)
    }

    pub fn open_position(
        ctx: Context<OpenPosition>,
        collateral_amount: u64,
        debt_amount: u64,
    ) -> Result<()> {
        ctx.accounts.open_position(collateral_amount, debt_amount)
    }

    pub fn close_position(ctx: Context<ClosePosition>) -> Result<()> {
        ctx.accounts.close_position()
    }

    pub fn update_interest_rate(ctx: Context<UpdateInterestRate>) -> Result<()> {
        ctx.accounts.update_interest_rate()
    }

    pub fn stake_stable_tokens(ctx: Context<Stake>, amount: u64) -> Result<()> {
        ctx.accounts.init_stake_account(&ctx.bumps)?;
        ctx.accounts.deposit_tokens(amount)
    }

    // claim stake rewards before unstaking
    pub fn unstake_stable_tokens(ctx: Context<UnStake>) -> Result<()> {
        ctx.accounts.withdraw_tokens(&ctx.bumps)
    }

    pub fn liquidate_position(ctx: Context<LiquidatePosition>) -> Result<()> {
        ctx.accounts.liquidate_position()
    }

    pub fn claim_stake_reward(ctx: Context<ClaimStakeRewards>) -> Result<()> {
        ctx.accounts.claim_stake_reward()
    }
}
