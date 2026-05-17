use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct Take<'info>{
    pub maker: Signer<'info>,
}

impl <'info>Take<'info> {

    pub fn take(&mut self) -> Result<()> {
        
        Ok(())
    }

}
