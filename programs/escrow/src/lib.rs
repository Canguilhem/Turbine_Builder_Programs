pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("HSQjtk3WCLWicX9Mku925tHpq19B8x8bj39WZXCMrUNR");

#[program]
pub mod escrow {
    use super::*;

    #[instruction(discriminator = 0)]
    pub fn make(
        ctx: Context<Make>,
        seed: u64,
        receive: u64,
        deposit: u64,
        expiration: u64,
    ) -> Result<()> {
        ctx.accounts
            .init_escrow(seed, receive, expiration, &ctx.bumps)?;
        ctx.accounts.deposit(deposit)?;

        Ok(())
    }

    #[instruction(discriminator = 1)]
    pub fn take(ctx: Context<Take>) -> Result<()> {
        ctx.accounts.deposit()?;
        ctx.accounts.withdraw_and_close()?;

        Ok(())
    }

    #[instruction(discriminator = 2)]
    pub fn refund(ctx: Context<Refund>) -> Result<()> {
        ctx.accounts.refund_and_close()?;

        Ok(())
    }
}
