use anchor_lang::{prelude::*, system_program::{Transfer, transfer}};

use crate::VaultState;

#[derive(Accounts)]
pub struct Initialize <'info> {

    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        init,
        payer=user,
        seeds=[b"state", user.key().as_ref()],
        bump,
        space= VaultState::DISCRIMINATOR.len() + VaultState::INIT_SPACE
    )]
    pub vault_state:Account<'info,VaultState>,

    #[account(
        mut,
        seeds=[b"vault",vault_state.key().as_ref()],
        bump
    )]    
    pub vault: SystemAccount<'info>,
    pub system_program: Program<'info,System>
}

impl <'info> Initialize<'info> {
    pub fn initialize(&mut self, bump: &InitializeBumps) -> Result<()> {
        let rent_exempt=Rent::get()?.minimum_balance(self.vault.to_account_info().data_len());
        
        let cpi_accounts= Transfer {
            from:self.user.to_account_info(),
            to:self.vault.to_account_info()
        };

        // using System::id() instead of  self.system_program.key()
        let cpi_ctx= CpiContext::new(System::id(), cpi_accounts);
        
        self.vault_state.state_bump= bump.vault_state;
        self.vault_state.vault_bump= bump.vault;

        Ok(transfer(cpi_ctx,rent_exempt)?)
    }
}