use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct Close<'info>{
    pub maker: Signer<'info>,
}

impl <'info>Close<'info> {

    pub fn close(&mut self) -> Result<()> {
        
        Ok(())
    }

}
