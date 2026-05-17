use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct Make<'info>{
    pub maker: Signer<'info>,
}

impl <'info>Make<'info> {

    pub fn make(&mut self) -> Result<()> {
        
        Ok(())
    }

}

