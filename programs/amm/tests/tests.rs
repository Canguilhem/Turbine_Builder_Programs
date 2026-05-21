use anchor_lang::AccountDeserialize;
use anchor_spl::{associated_token};
use litesvm::LiteSVM;
use litesvm_token::{CreateMint, MintTo};
use solana_keypair::Keypair;
use solana_message::{Instruction, Message, VersionedMessage};
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::versioned::VersionedTransaction;
mod utils;
use utils::create_initialize_ix;

use crate::utils::{
    create_deposit_ix, create_swap_ix, create_withdraw_ix, get_user_atas, token_balance, update_config_ix,
};

fn send(
    svm: &mut LiteSVM,
    ixs: &[Instruction],
    payer: &Keypair,
    signers: &[&Keypair],
) -> litesvm::types::TransactionResult {
    svm.expire_blockhash();
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(ixs, Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), signers).unwrap();
    svm.send_transaction(tx)
}

fn setup() -> (
    LiteSVM,
    Keypair,
    Pubkey,
    Pubkey,
    Pubkey,
    Pubkey,
    Pubkey,
    Pubkey,
) {
    let program_id = amm::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!("../../../target/deploy/amm.so");
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&payer.pubkey(), 1_000_000_000).unwrap();

    // create mints
    let mint_x = CreateMint::new(&mut svm, &payer)
        .decimals(6)
        .authority(&payer.pubkey())
        .send()
        .unwrap();
    let mint_y = CreateMint::new(&mut svm, &payer)
        .decimals(6)
        .authority(&payer.pubkey())
        .send()
        .unwrap();

    let config = Pubkey::find_program_address(&[b"config", &123u64.to_le_bytes()], &program_id).0;
    let mint_lp = Pubkey::find_program_address(&[b"lp", config.as_ref()], &program_id).0;

    // vaults derivation
    let vault_x = associated_token::get_associated_token_address(&config, &mint_x);
    let vault_y = associated_token::get_associated_token_address(&config, &mint_y);

    (
        svm, payer, mint_x, mint_y, config, mint_lp, vault_x, vault_y,
    )
}

#[test]
fn test_initialize() {
    let (mut svm, payer, mint_x, mint_y, config, mint_lp, vault_x, vault_y) = setup();
    let seed = 123;
    let fee = 30;
    let init = create_initialize_ix(
        &mut svm, &payer, mint_x, mint_y, config, seed, fee, mint_lp, vault_x, vault_y,
    );

    let result = send(&mut svm, &[init], &payer, &[&payer]);

    let config_account = svm.get_account(&config).unwrap();
    let config_state =
        amm::state::Config::try_deserialize(&mut config_account.data.as_ref()).unwrap();
    assert!(result.is_ok());

    assert_eq!(config_state.authority, Some(payer.pubkey()));
    assert_eq!(config_state.locked, false);
    assert_eq!(config_state.mint_x, mint_x);
    assert_eq!(config_state.mint_y, mint_y);
    assert_eq!(config_state.fee, fee);
    assert_eq!(config_state.seed, seed);

    // bump assertion
    let (config_pda, config_bump) =
        Pubkey::find_program_address(&[b"config", &123u64.to_le_bytes()], &amm::id());
    let (_mint_lp_pda, lp_bump) =
        Pubkey::find_program_address(&[b"lp", config_pda.as_ref()], &amm::id());
    assert_eq!(config_state.config_bump, config_bump);
    assert_eq!(config_state.lp_bump, lp_bump);
}

#[test]
fn test_deposit() {
    let (mut svm, payer, mint_x, mint_y, config, mint_lp, vault_x, vault_y) = setup();
    let init = create_initialize_ix(
        &mut svm, &payer, mint_x, mint_y, config, 123, 30, mint_lp, vault_x, vault_y,
    );

    let (user_x, user_y, user_lp) = get_user_atas(&mut svm, &payer, mint_x, mint_y, mint_lp);

    // Mint 1000 X & Y tokens to user ATAs
    MintTo::new(&mut svm, &payer, &mint_x, &user_x, 1_000_000_000)
        .send()
        .unwrap();
    MintTo::new(&mut svm, &payer, &mint_y, &user_y, 1_000_000_000)
        .send()
        .unwrap();

    let user_x_balance_before = token_balance(&svm, &user_x);
    let user_y_balance_before = token_balance(&svm, &user_y);

    let deposit_amount = 100_000_000;

    let deposit = create_deposit_ix(
        &mut svm,
        &payer,
        mint_x,
        mint_y,
        config,
        mint_lp,
        vault_x,
        vault_y,
        user_x,
        user_y,
        user_lp,
        deposit_amount,
        deposit_amount,
        deposit_amount,
    );

    let res = send(&mut svm, &[init, deposit], &payer, &[&payer]);
    assert!(res.is_ok());

    // user balances
    let user_x_balance = token_balance(&svm, &user_x);
    let user_y_balance = token_balance(&svm, &user_y);
    // vault balances
    let vault_x_balance = token_balance(&svm, &vault_x);
    let vault_y_balance = token_balance(&svm, &vault_y);
    //

    assert_eq!(
        vault_x_balance,
        user_x_balance_before - user_x_balance,
        "deposit X: should equal balance_before minus balance_after"
    );
    assert_eq!(
        vault_y_balance,
        user_y_balance_before - user_y_balance,
        "deposit Y: should equal balance_before minus balance_after"
    );
}

#[test]
fn test_withdraw() {
    let (mut svm, payer, mint_x, mint_y, config, mint_lp, vault_x, vault_y) = setup();
    let init = create_initialize_ix(
        &mut svm, &payer, mint_x, mint_y, config, 123, 30, mint_lp, vault_x, vault_y,
    );

    let (user_x, user_y, user_lp) = get_user_atas(&mut svm, &payer, mint_x, mint_y, mint_lp);

    // Mint 1000 X & Y tokens to user ATAs
    MintTo::new(&mut svm, &payer, &mint_x, &user_x, 1_000_000_000)
        .send()
        .unwrap();
    MintTo::new(&mut svm, &payer, &mint_y, &user_y, 1_000_000_000)
        .send()
        .unwrap();

    let deposit_amount = 100_000_000;

    let deposit = create_deposit_ix(
        &mut svm,
        &payer,
        mint_x,
        mint_y,
        config,
        mint_lp,
        vault_x,
        vault_y,
        user_x,
        user_y,
        user_lp,
        deposit_amount,
        deposit_amount,
        deposit_amount,
    );

    let withdraw_amount = 25_000_000;

    // withdraw 25% when no swap happened
    let withdraw = create_withdraw_ix(
        &mut svm,
        &payer,
        mint_x,
        mint_y,
        config,
        mint_lp,
        vault_x,
        vault_y,
        user_x,
        user_y,
        user_lp,
        withdraw_amount,
        withdraw_amount,
        withdraw_amount,
    );

    let result = send(&mut svm, &[init, deposit, withdraw], &payer, &[&payer]);
    assert!(result.is_ok());

    // user balances
    let user_lp_balance = token_balance(&svm, &user_lp);
    // vault balances
    let vault_x_balance = token_balance(&svm, &vault_x);
    let vault_y_balance = token_balance(&svm, &vault_y);

    //   As solo LPer
    assert_eq!(
        user_lp_balance,
        deposit_amount - withdraw_amount,
        "current lp should equal deposit minus withdrew amount"
    );
    assert_eq!(
        vault_x_balance, vault_y_balance,
        "no swap ie: ratio is still 1:1"
    );
}

#[test]
fn test_swap() {
    let (mut svm, payer, mint_x, mint_y, config, mint_lp, vault_x, vault_y) = setup();
    let init = create_initialize_ix(
        &mut svm, &payer, mint_x, mint_y, config, 123, 30, mint_lp, vault_x, vault_y,
    );

    let (user_x, user_y, user_lp) = get_user_atas(&mut svm, &payer, mint_x, mint_y, mint_lp);

    // Mint 1000 X & Y tokens to user ATAs
    MintTo::new(&mut svm, &payer, &mint_x, &user_x, 1_000_000_000)
        .send()
        .unwrap();
    MintTo::new(&mut svm, &payer, &mint_y, &user_y, 1_000_000_000)
        .send()
        .unwrap();

    let user_x_balance_before = token_balance(&svm, &user_x);
    let user_y_balance_before = token_balance(&svm, &user_y);

    let deposit_amount = 100_000_000;

    let deposit = create_deposit_ix(
        &mut svm,
        &payer,
        mint_x,
        mint_y,
        config,
        mint_lp,
        vault_x,
        vault_y,
        user_x,
        user_y,
        user_lp,
        deposit_amount,
        deposit_amount,
        deposit_amount,
    );
    // X1 = Y1 = 100, k = X1·Y1
    // Swap in Δx = 10 → X2 = X1 + Δx = 110
    // dY = dX_eff·Y1 / (X1 + dX_eff),  dX_eff = 10·0.997
    // Y2 = Y1 − dY ≈ 90.934
    let swap_amount = 10_000_000;
    let min_amount = 9_066_000;

    let swap = create_swap_ix(
        &mut svm,
        &payer,
        mint_x,
        mint_y,
        config,
        mint_lp,
        vault_x,
        vault_y,
        user_x,
        user_y,
        true,
        swap_amount,
        min_amount,
    );

    let result = send(&mut svm, &[init, deposit, swap], &payer, &[&payer]);
    result.expect("init + deposit + swap");
    // user balances
    let user_x_balance = token_balance(&svm, &user_x);
    let user_y_balance = token_balance(&svm, &user_y);
    let user_lp_balance = token_balance(&svm, &user_lp);
    // vault balances
    let vault_x_balance = token_balance(&svm, &vault_x);
    let vault_y_balance = token_balance(&svm, &vault_y);

    assert_eq!(
        user_x_balance,
        user_x_balance_before - deposit_amount - swap_amount,
        "Used X to deposit and swap"
    );

    let y_from_swap = user_y_balance
        .saturating_add(deposit_amount)
        .saturating_sub(user_y_balance_before);
    assert!(
        y_from_swap >= min_amount,
        "should receive at least min Y from swap"
    );

    // after: vault -> before - after
    assert_eq!(
        vault_y_balance,
        user_y_balance_before - user_y_balance,
        "compare vault Y balance after"
    );
    assert_eq!(
        vault_x_balance,
        user_x_balance_before - user_x_balance,
        "compare vault X balance after"
    );

    // fee accumulates - k increased
    let k_before = deposit_amount as u128 * deposit_amount as u128;
    let k_after = vault_x_balance as u128 * vault_y_balance as u128;
    assert!(k_after >= k_before, "K increased after swap");
    assert_eq!(user_lp_balance, deposit_amount, "LP should be unchanged");
}


#[test]
fn test_update_config() {
    let (mut svm, payer, mint_x, mint_y, config, mint_lp, vault_x, vault_y) = setup();
    
    let seed= 123;
    let fee=30;
    
    let init = create_initialize_ix(
        &mut svm, &payer, mint_x, mint_y, config, seed, fee, mint_lp, vault_x, vault_y,
    );
    let new_admin = Keypair::new().pubkey();
    
    let update = update_config_ix(
        &payer,
        config,
        seed,
        true,
        Some(new_admin),
        fee * 2,
    );
    
    let result = send(&mut svm, &[init,update], &payer, &[&payer]);
    result.expect("update config");
    
    let config_account = svm.get_account(&config).unwrap();
    let config_state =
        amm::state::Config::try_deserialize(&mut config_account.data.as_ref()).unwrap();
    
    assert_eq!(config_state.authority, Some(new_admin));
    assert_eq!(config_state.locked, true);
    assert_eq!(config_state.fee, 2 * fee);
}
