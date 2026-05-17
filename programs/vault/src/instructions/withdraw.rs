use anchor_lang::{prelude::*, system_program::{Transfer, transfer}};

use crate::VaultState;

#[derive(Accounts)]
pub struct Withdraw <'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        seeds=[b"state",user.key().as_ref()],
        bump= vault_state.state_bump,
    )]
    pub vault_state: Account<'info, VaultState>,
    
    #[account(
        mut,
        seeds=[b"vault",vault_state.key().as_ref()],
        bump= vault_state.vault_bump,
    )]
    pub vault: SystemAccount<'info>,
    pub system_program: Program<'info, System>,

}

impl <'info> Withdraw<'info> {
    pub fn withdraw(&mut self, amount:u64) -> Result<()> {
        let cpi_accounts= Transfer {
            to:self.user.to_account_info(),
            from:self.vault.to_account_info()
        };


        // Funds will move from vault to user so vaultPda should be signing the transfer 
        let signer_seeds: &[&[&[u8]]]= &[&[
            b"vault",
            self.vault_state.to_account_info().key.as_ref(),
            &[self.vault_state.vault_bump]
        ]];
        

        let cpi_ctx= CpiContext::new_with_signer(System::id(), cpi_accounts, signer_seeds);
        

        Ok(transfer(cpi_ctx,amount)?)
    }
}