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

    #[instruction(discriminator = 1)]
    pub fn make(ctx: Context<Make>, seed: u64, deposit: u64, receive: u64) -> Result<()> {
        ctx.accounts.make()?;
        
        Ok(())
    }

    #[instruction(discriminator = 1)]
    pub fn take(ctx: Context<Take>) -> Result<()> {
        ctx.accounts.take()?;
        
        Ok(())
    }

    #[instruction(discriminator = 1)]
    pub fn close(ctx: Context<Close>) -> Result<()> {
        ctx.accounts.close()?;
        
        Ok(())
    }

    #[instruction(discriminator = 1)]
    pub fn refund(ctx: Context<Refund>) -> Result<()> {
        ctx.accounts.refund()?;
        
        Ok(())
    }
}
