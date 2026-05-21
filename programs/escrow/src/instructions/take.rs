use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::get_associated_token_address_with_program_id,
    token_interface::{
        close_account, transfer_checked, CloseAccount, Mint, TokenInterface, TransferChecked,
    },
};

use crate::{error::EscrowErrors, Escrow, ESCROW_SEED};

#[derive(Accounts)]
pub struct Take<'info> {
    #[account(mut)]
    pub taker: Signer<'info>,

    #[account(mut)]
    pub maker: SystemAccount<'info>,

    #[account(mut, mint::token_program = token_program)]
    pub mint_a: InterfaceAccount<'info, Mint>,

    #[account(mut, mint::token_program = token_program)]
    pub mint_b: InterfaceAccount<'info, Mint>,

    /// CHECK: ATA address verified below.
    #[account(mut,
        constraint = taker_ata_a.key() == get_associated_token_address_with_program_id(
            &taker.key(),
            &mint_a.key(),
            &token_program.key(),
        )
    )]
    pub taker_ata_a: UncheckedAccount<'info>,

    /// CHECK: ATA address verified below.
    #[account(mut,
        constraint = taker_ata_b.key() == get_associated_token_address_with_program_id(
            &taker.key(),
            &mint_b.key(),
            &token_program.key(),
        )
    )]
    pub taker_ata_b: UncheckedAccount<'info>,

    /// CHECK: ATA address verified below.
    #[account(mut,
        constraint = maker_ata_b.key() == get_associated_token_address_with_program_id(
            &maker.key(),
            &mint_b.key(),
            &token_program.key(),
        )
    )]
    pub maker_ata_b: UncheckedAccount<'info>,

    #[account(
        mut,
        close = maker,
        seeds = [ESCROW_SEED, escrow.maker.as_ref(), escrow.seed.to_le_bytes().as_ref()],
        bump = escrow.bump,
        has_one = mint_a,
        has_one = mint_b,
        has_one = maker
    )]
    pub escrow: Account<'info, Escrow>,

    /// CHECK: ATA (vault) address verified below.
    #[account(mut,
        constraint =
            vault.key() == get_associated_token_address_with_program_id(
                &escrow.key(),
                &mint_a.key(),
                &token_program.key(),
            )
    )]
    pub vault: UncheckedAccount<'info>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> Take<'info> {
    pub fn deposit(&mut self) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;
        require!(
            now < self.escrow.expiration as i64,
            EscrowErrors::EscrowExpired,
        );

        let cpi_accounts = TransferChecked {
            from: self.taker_ata_b.to_account_info(),
            mint: self.mint_b.to_account_info(),
            to: self.maker_ata_b.to_account_info(),
            authority: self.taker.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(self.token_program.key(), cpi_accounts);

        transfer_checked(cpi_ctx, self.escrow.receive_amount, self.mint_b.decimals)?;

        Ok(())
    }

    pub fn withdraw_and_close(&mut self) -> Result<()> {
        let cpi_accounts = TransferChecked {
            from: self.vault.to_account_info(),
            mint: self.mint_a.to_account_info(),
            to: self.taker_ata_a.to_account_info(),
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

        transfer_checked(cpi_ctx, self.escrow.receive_amount, self.mint_a.decimals)?;

        let close_accounts = CloseAccount {
            account: self.vault.to_account_info(),
            destination: self.maker.to_account_info(),
            authority: self.escrow.to_account_info(),
        };
        let close_cpi_ctx =
            CpiContext::new_with_signer(self.token_program.key(), close_accounts, signer_seeds);
        close_account(close_cpi_ctx)?;

        Ok(())
    }
}
