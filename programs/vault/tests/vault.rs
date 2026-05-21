use anchor_lang::{
    solana_program::msg, system_program::ID as SYSTEM_PROGRAM_ID, AccountDeserialize,
    InstructionData, ToAccountMetas,
};
use litesvm::LiteSVM;
use solana_keypair::Keypair;
use solana_message::Instruction;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::Transaction;

fn setup() -> (LiteSVM, Keypair) {
    let program_id = vault::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!("../../../target/deploy/vault.so");
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap(); // 10 Sol Airdrop
    (svm, payer)
}

fn init_vault(mut svm: LiteSVM, payer: &Keypair) -> (LiteSVM, Pubkey, Pubkey, u8, u8) {
    let user = payer.pubkey();
    let (vault_state_pda, state_bump) =
        Pubkey::find_program_address(&[b"state", user.as_ref()], &vault::id());
    let (vault_pda, vault_bump) =
        Pubkey::find_program_address(&[b"vault", vault_state_pda.as_ref()], &vault::id());

    let init_ix = Instruction {
        program_id: vault::id(),
        accounts: vault::accounts::Initialize {
            user,
            vault_state: vault_state_pda,
            vault: vault_pda,
            system_program: SYSTEM_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: vault::instruction::Initialize {}.data(),
    };

    // using doc approach https://www.litesvm.com/docs/api-reference/transactions
    let tx = Transaction::new_signed_with_payer(
        &[init_ix],
        Some(&payer.pubkey()),
        &[&payer],
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx).unwrap();
    msg!("init ok, logs: {:?}", result.logs);

    (svm, vault_state_pda, vault_pda, vault_bump, state_bump)
}

#[test]
fn test_vault_initialize() {
    let (svm, payer) = setup();

    let (svm, vault_state_pda, _vault_pda, vault_bump, state_bump) = init_vault(svm, &payer);

    // pda should exist
    let vault_state_account = svm.get_account(&vault_state_pda).unwrap();

    let vault_state =
        vault::state::VaultState::try_deserialize(&mut vault_state_account.data.as_ref()).unwrap();

    // checking bumps
    assert_eq!(vault_state.vault_bump, vault_bump);
    assert_eq!(vault_state.state_bump, state_bump);
}

#[test]
fn test_deposit_widthraw_close() {
    let (svm, payer) = setup();

    let (mut svm, vault_state_pda, vault_pda, _vault_bump, _state_bump) = init_vault(svm, &payer);

    // pda should exist
    svm.get_account(&vault_state_pda)
        .expect("vault_state should exist after initialize");

    let user = payer.pubkey();

    let vault_balance_before = svm.get_balance(&vault_pda).unwrap();

    let deposit_amount: u64 = 1_000_000_000;
    let deposit_ix = Instruction {
        program_id: vault::id(),
        accounts: vault::accounts::Deposit {
            user,
            vault_state: vault_state_pda,
            vault: vault_pda,
            system_program: SYSTEM_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: vault::instruction::Deposit {
            amount: deposit_amount,
        }
        .data(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[deposit_ix],
        Some(&payer.pubkey()),
        &[&payer],
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx).unwrap();
    msg!("deposit ok, logs: {:?}", result.logs);

    // balance_after = balance_before + deposit amount
    let vault_balance_after = svm.get_balance(&vault_pda).unwrap();
    assert_eq!(
        vault_balance_after
            .checked_sub(vault_balance_before)
            .expect("deposit should increase vault lamports"),
        deposit_amount
    );
    // withdraw
    let withdraw_amount: u64 = 1_000_000_000;
    let withdraw_ix = Instruction {
        program_id: vault::id(),
        accounts: vault::accounts::Withdraw {
            user,
            vault_state: vault_state_pda,
            vault: vault_pda,
            system_program: SYSTEM_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: vault::instruction::Withdraw {
            amount: withdraw_amount,
        }
        .data(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[withdraw_ix],
        Some(&payer.pubkey()),
        &[&payer],
        svm.latest_blockhash(),
    );

    let vault_balance_before_withdraw = svm.get_balance(&vault_pda).unwrap();

    let result = svm.send_transaction(tx).unwrap();
    msg!("withdraw ok, logs: {:?}", result.logs);

    let vault_balance_after_withdraw = svm.get_balance(&vault_pda).unwrap();
    assert_eq!(
        vault_balance_before_withdraw
            .checked_sub(vault_balance_after_withdraw)
            .expect("withdraw should decrease vault lamports"),
        withdraw_amount
    );

    // close

    let close_ix = Instruction {
        program_id: vault::id(),
        accounts: vault::accounts::Close {
            user,
            vault_state: vault_state_pda,
            vault: vault_pda,
            system_program: SYSTEM_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: vault::instruction::Close {}.data(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[close_ix],
        Some(&payer.pubkey()),
        &[&payer],
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx).unwrap();
    msg!("close ok, logs: {:?}", result.logs);

    // accounts have been closed
    assert!(svm.get_account(&vault_pda).is_none());
    assert!(svm.get_account(&vault_state_pda).is_none());

    let user_balance_after_close = svm.get_balance(&user).unwrap();
    assert!(user_balance_after_close > vault_balance_after_withdraw);
    msg!("Balance after close {} ", user_balance_after_close)
}
