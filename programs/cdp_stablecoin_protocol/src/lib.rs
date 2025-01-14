use anchor_lang::prelude::*;

declare_id!("BvMWoXUSWLR4udmcmPX5M9DM89kCCJJvQWvxKE55uBGH");

#[program]
pub mod cdp_stablecoin_protocol {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
