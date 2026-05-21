use anchor_lang::prelude::*;
use anchor_spl::{associated_token::AssociatedToken, token::{Burn, Mint, Token, TokenAccount, Transfer, burn, transfer},};
use constant_product_curve::{ConstantProduct};

use crate::{CONFIG_SEED, Config, LP_SEED, error::AmmError};

#[derive(Accounts)]
pub struct Withdraw <'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    pub mint_x: Account <'info,Mint>,
    pub mint_y: Account <'info,Mint>,
    #[account(
        has_one= mint_x,
        has_one= mint_y,
        seeds= [CONFIG_SEED,config.seed.to_le_bytes().as_ref()],
        bump
    )]
    pub config: Account <'info,Config>,
    #[account(
        mut,
        seeds=[LP_SEED,config.key().as_ref()],
        bump= config.lp_bump
    )]
    pub mint_lp:Account <'info,Mint>,
    // VAULT X/Y ATAs 
    #[account(
        mut,
        associated_token::mint= mint_x,
        associated_token::authority= config
    )]
    pub vault_x:Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint= mint_y,
        associated_token::authority= config
    )]
    pub vault_y:Box<Account<'info, TokenAccount>>,

    // User X/Y ATAs 
    #[account(
        mut,
        associated_token::mint= mint_x,
        associated_token::authority= user
    )]
    pub user_x:Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint= mint_y,
        associated_token::authority= user
    )]
    pub user_y:Box<Account<'info, TokenAccount>>,
    #[account(
        init_if_needed,
        payer=user,
        associated_token::mint= mint_lp,
        associated_token::authority= user
    )]
    pub user_lp: Box<Account<'info, TokenAccount>>,

    pub token_program:Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub associated_token_program:Program<'info, AssociatedToken>
}

impl <'info> Withdraw <'info> {

    pub fn withdraw(
        &mut self,
        amount:u64, // amount of LP that user wants to "burn"
        min_x:u64,  // min amount of X that user is willing to receive
        min_y:u64   // min amount of Y that user is willing to receive
     )-> Result<()>{
        require!(!self.config.locked, AmmError::PoolLocked);
        require_neq!(amount,0, AmmError::InvalidAmount);

        let (x, y) = if self.mint_lp.supply == 0 && self.vault_x.amount == 0 && self.vault_y.amount == 0 {
            (min_x, min_y)
        }else {
            let amounts = ConstantProduct::xy_withdraw_amounts_from_l(
                self.vault_x.amount, self.vault_y.amount, self.mint_lp.supply, amount, 6
            ).unwrap();

            require!(amounts.x >= min_x && amounts.y >= min_y, AmmError::SlippageLimitExceeded);

            (amounts.x, amounts.y)
        };

        // deposit X
        self.withdraw_tokens(true, x)?;
        // deposit Y
        self.withdraw_tokens(false, y)?;
        // mint LP
        self.burn_lp_tokens(amount)
    }

    pub fn withdraw_tokens(&self,is_x:bool,amount:u64)-> Result<()>{
        let program= self.token_program.key();
        
        let(from,to)= match is_x {
            true=>(
                self.vault_x.to_account_info(),
                self.user_x.to_account_info(),
            ),
            false=>(
                self.vault_y.to_account_info(),
                self.user_y.to_account_info(),
            )
        };


        let accounts= Transfer{
            from,
            to,
            authority: self.config.to_account_info()
        };

        let signer_seeds: &[&[&[u8]]]= &[&[
            CONFIG_SEED,
            &self.config.seed.to_le_bytes(),
            &[self.config.config_bump]
        ]];

        let ctx= CpiContext::new_with_signer(program, accounts, signer_seeds);
        transfer(ctx,amount)
    }

    pub fn burn_lp_tokens(&self,amount:u64)-> Result<()>{
        let program= self.token_program.key();

        let accounts= Burn{
            mint: self.mint_lp.to_account_info(),
            authority: self.user.to_account_info(),
            from: self.user_lp.to_account_info(),
        };

        let ctx= CpiContext::new(program, accounts);

        burn(ctx, amount)
    }
}