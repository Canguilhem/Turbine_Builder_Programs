use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct Refund<'info>{
    pub maker: Signer<'info>,
}

impl <'info>Refund<'info> {

    pub fn refund(&mut self) -> Result<()> {
        
        Ok(())
    }

}
