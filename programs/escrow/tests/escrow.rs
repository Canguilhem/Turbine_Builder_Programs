use anchor_lang::{
    solana_program::msg, solana_program::program_pack::Pack,
    system_program::ID as SYSTEM_PROGRAM_ID, AccountDeserialize, InstructionData, ToAccountMetas,
};
use anchor_spl::{associated_token, token, token::spl_token::state::Account as SplTokenAccount};
use litesvm::{types::FailedTransactionMetadata, LiteSVM};
use litesvm_token::{CreateAssociatedTokenAccount, CreateMint, MintTo};
use solana_clock::Clock;
use solana_keypair::Keypair;
use solana_message::Instruction;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::Transaction;

const RECEIVE: u64 = 10_000_000;
const DEPOSIT: u64 = 10_000_000;
const MINT_TO_MAKER: u64 = 1_000_000_000;

/// Deterministic “now” in tests (Unix seconds).
const T0_UNIX: i64 = 1_730_000_000;

fn set_unix_timestamp(svm: &mut LiteSVM, unix_ts: i64) {
    let mut clock = svm.get_sysvar::<Clock>();
    clock.unix_timestamp = unix_ts;
    svm.set_sysvar(&clock);
}

/// Escrow `expiration` field: absolute Unix timestamp (must be **greater** than on-chain clock).
fn ts_after_secs(offset_secs: i64) -> u64 {
    (T0_UNIX.saturating_add(offset_secs)).max(0) as u64
}

fn setup() -> (LiteSVM, Keypair) {
    let program_id = escrow::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!("../../../target/deploy/escrow.so");
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap(); // 10 Sol Airdrop
    set_unix_timestamp(&mut svm, T0_UNIX);
    (svm, payer)
}

#[derive(Debug)]
/// Addresses and amounts after a successful `make` instruction.
struct EscrowAfterMake {
    maker: Pubkey,
    seed: u64,
    mint_a: Pubkey,
    mint_b: Pubkey,
    maker_ata_a: Pubkey,
    escrow: Pubkey,
    vault: Pubkey,
    expiration: u64,
}

fn token_balance(svm: &LiteSVM, ata: &Pubkey) -> u64 {
    let acc = svm.get_account(ata).expect("SPL token account missing");
    let mut data: &[u8] = acc.data.as_ref();
    SplTokenAccount::unpack(&mut data)
        .expect("unpack SPL token account")
        .amount
}

/// Escrow remains “active” for takers until this absolute Unix time (well after `T0_UNIX`).
fn default_escrow_expiration() -> u64 {
    ts_after_secs(3600)
}

fn try_make_escrow(
    program: &mut LiteSVM,
    payer: &Keypair,
    seed: u64,
    expiration_unix: u64,
) -> Result<EscrowAfterMake, FailedTransactionMetadata> {
    let maker = payer.pubkey();

    let mint_a = CreateMint::new(program, payer)
        .decimals(6)
        .authority(&maker)
        .send()
        .unwrap();
    msg!("Mint A: {}", mint_a);

    let mint_b = CreateMint::new(program, payer)
        .decimals(6)
        .authority(&maker)
        .send()
        .unwrap();
    msg!("Mint B: {}", mint_b);

    let maker_ata_a = CreateAssociatedTokenAccount::new(program, payer, &mint_a)
        .owner(&maker)
        .send()
        .unwrap();
    msg!("Maker ATA_A: {}", maker_ata_a);

    let escrow_addr = Pubkey::find_program_address(
        &[b"escrow", maker.as_ref(), &seed.to_le_bytes()],
        &escrow::id(),
    )
    .0;
    msg!("Escrow PDA: {}", escrow_addr);

    let vault = associated_token::get_associated_token_address(&escrow_addr, &mint_a);
    msg!("Vault PDA: {}", vault);

    MintTo::new(program, payer, &mint_a, &maker_ata_a, MINT_TO_MAKER)
        .send()
        .unwrap();

    let mint_a_pk: Pubkey = mint_a.into();
    let mint_b_pk: Pubkey = mint_b.into();
    let maker_ata_a_pk: Pubkey = maker_ata_a.into();

    let make_ix = Instruction {
        program_id: escrow::id(),
        accounts: escrow::accounts::Make {
            maker,
            mint_a: mint_a_pk,
            mint_b: mint_b_pk,
            maker_ata_a: maker_ata_a_pk,
            escrow: escrow_addr,
            vault,
            token_program: token::ID,
            associated_token_program: associated_token::ID,
            system_program: SYSTEM_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: escrow::instruction::Make {
            seed,
            receive: RECEIVE,
            deposit: DEPOSIT,
            expiration: expiration_unix,
        }
        .data(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[make_ix],
        Some(&payer.pubkey()),
        &[payer],
        program.latest_blockhash(),
    );

    let result = program.send_transaction(tx)?;
    msg!("make ok, logs: {:?}", result.logs);

    Ok(EscrowAfterMake {
        maker,
        seed,
        mint_a: mint_a_pk,
        mint_b: mint_b_pk,
        maker_ata_a: maker_ata_a_pk,
        escrow: escrow_addr,
        vault,
        expiration: expiration_unix,
    })
}

fn make_escrow(
    program: &mut LiteSVM,
    payer: &Keypair,
    seed: u64,
    expiration_unix: u64,
) -> EscrowAfterMake {
    try_make_escrow(program, payer, seed, expiration_unix).expect("make escrow")
}

#[test]
fn test_escrow_make() {
    let (mut program, payer) = setup();
    let seed: u64 = 123;

    let fx = make_escrow(&mut program, &payer, seed, default_escrow_expiration());

    let escrow_data = program.get_account(&fx.escrow).expect("escrow PDA");
    let escrow_state =
        escrow::Escrow::try_deserialize(&mut escrow_data.data.as_ref()).expect("escrow state");
    assert_eq!(escrow_state.seed, fx.seed);
    assert_eq!(escrow_state.maker, fx.maker);
    assert_eq!(escrow_state.mint_a, fx.mint_a);
    assert_eq!(escrow_state.mint_b, fx.mint_b);
    assert_eq!(escrow_state.expiration, fx.expiration);
}

#[test]
fn test_escrow_refund() {
    let (mut program, payer) = setup();
    let seed: u64 = 456;

    let fx = make_escrow(&mut program, &payer, seed, default_escrow_expiration());
    let escrow_data = program
        .get_account(&fx.escrow)
        .expect("escrow before refund");
    let escrow_state =
        escrow::Escrow::try_deserialize(&mut escrow_data.data.as_ref()).expect("escrow state");
    assert_eq!(escrow_state.seed, fx.seed);

    assert_eq!(
        token_balance(&program, &fx.maker_ata_a),
        MINT_TO_MAKER - DEPOSIT,
        "after make, maker ATA should have minted amount minus vault deposit"
    );
    assert_eq!(
        token_balance(&program, &fx.vault),
        DEPOSIT,
        "vault should hold the deposited tokens"
    );

    let refund_ix = Instruction {
        program_id: escrow::id(),
        accounts: escrow::accounts::Refund {
            maker: fx.maker,
            mint_a: fx.mint_a,
            maker_ata_a: fx.maker_ata_a,
            vault: fx.vault,
            escrow: fx.escrow,
            token_program: token::ID,
            system_program: SYSTEM_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: escrow::instruction::Refund {}.data(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[refund_ix],
        Some(&payer.pubkey()),
        &[&payer],
        program.latest_blockhash(),
    );

    let result = program.send_transaction(tx).unwrap();
    msg!("refund ok, logs: {:?}", result.logs);

    assert!(
        program.get_account(&fx.escrow).is_none(),
        "escrow account should be closed to maker"
    );
    assert!(
        program.get_account(&fx.vault).is_none(),
        "vault ATA should be closed"
    );
    assert_eq!(
        token_balance(&program, &fx.maker_ata_a),
        MINT_TO_MAKER,
        "maker should recover the full minted balance after refund"
    );
}

#[test]
fn test_escrow_take() {
    let (mut program, maker_kp) = setup();
    let seed: u64 = 789;

    let fx = make_escrow(&mut program, &maker_kp, seed, default_escrow_expiration());

    let taker = Keypair::new();
    program
        .airdrop(&taker.pubkey(), 10_000_000_000)
        .expect("taker SOL for fees and ATAs");

    let taker_ata_a_addr = CreateAssociatedTokenAccount::new(&mut program, &taker, &fx.mint_a)
        .owner(&taker.pubkey())
        .send()
        .expect("taker ATA for mint A");
    let taker_ata_a: Pubkey = taker_ata_a_addr.into();

    let maker_ata_b_addr = CreateAssociatedTokenAccount::new(&mut program, &maker_kp, &fx.mint_b)
        .owner(&fx.maker)
        .send()
        .expect("maker ATA for mint B");
    let maker_ata_b: Pubkey = maker_ata_b_addr.into();

    let taker_ata_b_addr = CreateAssociatedTokenAccount::new(&mut program, &taker, &fx.mint_b)
        .owner(&taker.pubkey())
        .send()
        .expect("taker ATA for mint B");
    MintTo::new(
        &mut program,
        &maker_kp,
        &fx.mint_b,
        &taker_ata_b_addr,
        RECEIVE + 5_000_000,
    )
    .send()
    .expect("mint B to taker ATA (maker is mint authority)");

    let taker_ata_b: Pubkey = taker_ata_b_addr.into();

    let take_ix = Instruction {
        program_id: escrow::id(),
        accounts: escrow::accounts::Take {
            taker: taker.pubkey(),
            maker: fx.maker,
            mint_a: fx.mint_a,
            mint_b: fx.mint_b,
            taker_ata_a,
            taker_ata_b,
            maker_ata_b,
            escrow: fx.escrow,
            vault: fx.vault,
            token_program: token::ID,
            system_program: SYSTEM_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: escrow::instruction::Take {}.data(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[take_ix],
        Some(&taker.pubkey()),
        &[&taker],
        program.latest_blockhash(),
    );

    let result = program.send_transaction(tx).expect("take");
    msg!("take ok, logs: {:?}", result.logs);

    assert!(
        program.get_account(&fx.escrow).is_none(),
        "escrow should be closed to maker"
    );
    assert!(
        program.get_account(&fx.vault).is_none(),
        "vault should be closed"
    );

    assert_eq!(token_balance(&program, &taker_ata_a), RECEIVE);
    assert_eq!(token_balance(&program, &maker_ata_b), RECEIVE);
    assert_eq!(
        token_balance(&program, &fx.maker_ata_a),
        MINT_TO_MAKER - DEPOSIT,
        "maker ATA for mint A is unchanged by take"
    );
}

fn assert_log_contains(failure: &FailedTransactionMetadata, needle: &str) {
    let joined = failure.meta.logs.join("\n");
    assert!(
        joined.contains(needle),
        "expected logs to contain {needle:?}, got:\n{joined}"
    );
}

#[test]
fn test_make_rejects_expiration_not_in_future() {
    let (mut program, payer) = setup();
    let bad_exp = ts_after_secs(-60);
    let err = try_make_escrow(&mut program, &payer, 999, bad_exp).unwrap_err();
    assert_log_contains(&err, "Expiration must be in the future");
}

#[test]
fn test_maker_refund_succeeds_after_expiration() {
    let (mut program, payer) = setup();
    let seed: u64 = 42;
    let fx = make_escrow(&mut program, &payer, seed, ts_after_secs(30));

    assert!(fx.expiration > T0_UNIX as u64);
    set_unix_timestamp(&mut program, T0_UNIX + 100);

    let refund_ix = Instruction {
        program_id: escrow::id(),
        accounts: escrow::accounts::Refund {
            maker: fx.maker,
            mint_a: fx.mint_a,
            maker_ata_a: fx.maker_ata_a,
            vault: fx.vault,
            escrow: fx.escrow,
            token_program: token::ID,
            system_program: SYSTEM_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: escrow::instruction::Refund {}.data(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[refund_ix],
        Some(&payer.pubkey()),
        &[&payer],
        program.latest_blockhash(),
    );

    program
        .send_transaction(tx)
        .expect("maker refund after expiry should succeed");
    assert!(program.get_account(&fx.escrow).is_none());
    assert!(program.get_account(&fx.vault).is_none());
    assert_eq!(token_balance(&program, &fx.maker_ata_a), MINT_TO_MAKER);
}

#[test]
fn test_taker_take_fails_after_expiration() {
    let (mut program, maker_kp) = setup();
    let seed: u64 = 800;
    let fx = make_escrow(&mut program, &maker_kp, seed, ts_after_secs(30));

    let taker = Keypair::new();
    program.airdrop(&taker.pubkey(), 10_000_000_000).unwrap();

    let taker_ata_a_addr = CreateAssociatedTokenAccount::new(&mut program, &taker, &fx.mint_a)
        .owner(&taker.pubkey())
        .send()
        .unwrap();
    let taker_ata_a: Pubkey = taker_ata_a_addr.into();

    let maker_ata_b_addr = CreateAssociatedTokenAccount::new(&mut program, &maker_kp, &fx.mint_b)
        .owner(&fx.maker)
        .send()
        .unwrap();
    let maker_ata_b: Pubkey = maker_ata_b_addr.into();

    let taker_ata_b_addr = CreateAssociatedTokenAccount::new(&mut program, &taker, &fx.mint_b)
        .owner(&taker.pubkey())
        .send()
        .unwrap();
    MintTo::new(
        &mut program,
        &maker_kp,
        &fx.mint_b,
        &taker_ata_b_addr,
        RECEIVE + 5_000_000,
    )
    .send()
    .unwrap();
    let taker_ata_b: Pubkey = taker_ata_b_addr.into();

    set_unix_timestamp(&mut program, T0_UNIX + 100);

    let take_ix = Instruction {
        program_id: escrow::id(),
        accounts: escrow::accounts::Take {
            taker: taker.pubkey(),
            maker: fx.maker,
            mint_a: fx.mint_a,
            mint_b: fx.mint_b,
            taker_ata_a,
            taker_ata_b,
            maker_ata_b,
            escrow: fx.escrow,
            vault: fx.vault,
            token_program: token::ID,
            system_program: SYSTEM_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: escrow::instruction::Take {}.data(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[take_ix],
        Some(&taker.pubkey()),
        &[&taker],
        program.latest_blockhash(),
    );

    let err = program.send_transaction(tx).unwrap_err();
    assert_log_contains(&err, "Escrow expired");
    assert!(program.get_account(&fx.escrow).is_some());
}
