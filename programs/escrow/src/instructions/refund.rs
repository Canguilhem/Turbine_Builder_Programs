use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    close_account, transfer_checked, CloseAccount, Mint, TokenAccount, TokenInterface,
    TransferChecked,
};

use crate::{Escrow, ESCROW_SEED};

#[derive(Accounts)]
pub struct Refund<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(mut, mint::token_program=token_program)]
    pub mint_a: InterfaceAccount<'info, Mint>,

    #[account(mut, associated_token::mint= mint_a, associated_token::authority= maker, associated_token::token_program= token_program)]
    pub maker_ata_a: InterfaceAccount<'info, TokenAccount>,

    #[account(mut, associated_token::mint=mint_a, associated_token::authority= escrow, associated_token::token_program= token_program )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    #[account(mut, 
        close= maker, 
        has_one=mint_a, 
        has_one=maker,
        seeds= [ESCROW_SEED, maker.key().as_ref(), &escrow.seed.to_le_bytes()],
        bump= escrow.bump
    )]
    pub escrow: Account<'info, Escrow>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> Refund<'info> {
    pub fn refund_and_close(&mut self) -> Result<()> {
        // transfer token A from vault to maker

        let cpi_accounts = TransferChecked {
            from: self.vault.to_account_info(),
            mint: self.mint_a.to_account_info(),
            to: self.maker_ata_a.to_account_info(),
            authority: self.escrow.to_account_info(),
        };

        let signer_seeds: &[&[&[u8]]] = &[&[
            ESCROW_SEED,
            self.escrow.maker.as_ref(),
            &self.escrow.seed.to_le_bytes(),
            &[self.escrow.bump],
        ]];

        let cpi_ctx =
            CpiContext::new_with_signer(self.token_program.key(), cpi_accounts, signer_seeds);
        transfer_checked(cpi_ctx, self.vault.amount, self.mint_a.decimals)?;

        // close the vault
        let close_accounts = CloseAccount {
            account: self.vault.to_account_info(),
            destination: self.maker.to_account_info(),
            authority: self.escrow.to_account_info(),
        };

        // Cpi context with escrow as signer
        let close_cpi_ctx =
            CpiContext::new_with_signer(self.token_program.key(), close_accounts, signer_seeds);

        close_account(close_cpi_ctx)?;
        Ok(())
    }
}
