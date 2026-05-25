# AMM (Anchor)

Two-token constant-product AMM on Solana. Liquidity providers deposit **mint X** and **mint Y**, receive LP tokens, and withdraw pro-rata. Swappers trade against pool vaults. All curve math lives in the workspace [`cpmm`](../../crates/cpmm/README.md) crate (Uniswap V2-style, no external curve dependency).

**Program ID:** `ExzLNn8DtryFZcebCuLXebVht54Wkd6m7EqYS8UFAjSS`

## Architecture

Each pool is identified by a **`seed`** (u64). One initializer creates:

| Account               | Role                                                           | Derivation                           |
| --------------------- | -------------------------------------------------------------- | ------------------------------------ |
| **Config** (`Config`) | Pool metadata: mints, fee, authority, lock flag, stored bumps. | `[b"config", seed.to_le_bytes()]`    |
| **LP mint**           | Pro-rata share of vault reserves; mint authority = config PDA. | `[b"lp", config]`                    |
| **Vault X / Y**       | ATAs holding pool token reserves; authority = config PDA.      | ATA for `(mint_x \| mint_y, config)` |

The config PDA signs token transfers out of the vaults and mints LP tokens.

```
Client                    Program                         CPMM (pure math)
  │                          │                                  │
  │  deposit / swap / etc.   │  read vault + LP supply          │
  │ ───────────────────────► │ ────────────────────────────────►│
  │                          │◄──────── quote ──────────────────│
  │                          │  CPI: transfer / mint / burn     │
  │◄─────────────────────────│                                  │
```

## On-chain state

`Config` stores:

| Field                    | Meaning                                                                  |
| ------------------------ | ------------------------------------------------------------------------ |
| `seed`                   | Pool identifier; part of the config PDA seeds.                           |
| `authority`              | Optional admin pubkey; required to match signer on `update_config`.      |
| `mint_x`, `mint_y`       | The two pool token mints (SPL Token, not Token-2022 in this crate).      |
| `fee`                    | Swap fee in basis points (e.g. `30` = 0.30%). Must be `< 10_000`.        |
| `locked`                 | When `true`, `deposit`, `withdraw`, and `swap` reject with `PoolLocked`. |
| `config_bump`, `lp_bump` | Stored bumps for PDA validation and CPI signers.                         |

Constant seed prefixes: `CONFIG_SEED = b"config"`, `LP_SEED = b"lp"` (`constants.rs`).

## Instructions

### `initialize`

**Arguments:** `seed`, `fee`, `authority: Option<Pubkey>`.

1. Creates **config** PDA, **LP mint** (6 decimals, authority = config), and **vault X / Y** ATAs.
2. Writes pool metadata; `locked` starts as `false`.

The **initializer** pays rent for all new accounts.

### `deposit`

**Arguments:** `token_x: Option<u64>`, `token_y: Option<u64>`, `side: OperationSide`, `min_lp`.

| `side`     | `token_x` / `token_y`        | Behavior                                      |
| ---------- | ---------------------------- | --------------------------------------------- |
| `Balanced` | `Some(x)`, `Some(y)`         | Double-sided add liquidity (primary path)     |
| `X`        | `Some(x)`, `None`            | Single-sided X (SwapDeposit; math in cpmm)      |
| `Y`        | `None`, `Some(y)`            | Single-sided Y (SwapDeposit; math in cpmm)     |

1. Rejects if pool is locked.
2. Builds `PoolState` from vault balances + LP supply, calls `PoolState::deposit`.
3. Transfers quoted X/Y from user → vaults; mints quoted LP to user (`init_if_needed` on user LP ATA).
4. Fails with `SlippageLimitExceeded` if minted LP `< min_lp`.

**First deposit:** LP minted = `isqrt(x * y)` (Uniswap-style initial liquidity).

### `withdraw`

**Arguments:** `lp_amount`, `side: OperationSide`, `min_x`, `min_y`.

| `side`     | Behavior                                                         |
| ---------- | ---------------------------------------------------------------- |
| `Balanced` | Pro-rata X and Y (standard LP exit)                              |
| `X`        | X only — pro-rata withdraw + swap Y leg (WithdrawSwap in cpmm)   |
| `Y`        | Y only — pro-rata withdraw + swap X leg                          |

1. Rejects if pool is locked or `lp_amount == 0`.
2. Quotes via `PoolState::withdraw`; fails if output `< min_x` / `min_y`.
3. Transfers tokens vault → user; burns LP from user.

### `swap`

**Arguments:** `amount` (input size), `side: OperationSide` (`X` = pay X receive Y, `Y` = pay Y receive X), `min_out`.

1. Rejects if pool is locked or `amount == 0`.
2. Quotes via `PoolState::swap`; fails if output `< min_out`.
3. Transfers input user → vault, output vault → user (config PDA signs).

Swap fees stay in the vault (k increases); LP supply is unchanged.

### `update_config`

**Arguments:** `_seed`, `fee`, `authority`, `locked`.

**Who signs:** current `config.authority` (must be `Some` and equal to signer).

Updates `fee`, `authority`, and `locked`. Fee must be `< 10_000`. If `authority` is set to `None` on initialize, no signer can pass the authority check and the config becomes effectively immutable.

## Errors

Program errors (`AmmError`) include pool-level guards (`PoolLocked`, `Unauthorized`) and math errors mapped from `CpmmError` at the instruction boundary via `.map_err(AmmError::from)`. See [cpmm error mapping](../../crates/cpmm/README.md#errors).

## Client integration

Derive addresses for a pool:

1. `(config, config_bump) = find_program_address([b"config", seed.to_le_bytes()], program_id)`
2. `(mint_lp, lp_bump) = find_program_address([b"lp", config], program_id)`
3. `vault_x = get_associated_token_address(config, mint_x)`
4. `vault_y = get_associated_token_address(config, mint_y)`

Pass mints, config, LP mint, vaults, user ATAs, and SPL / ATA / system programs as required by each instruction’s `#[derive(Accounts)]` struct. Read `mint_x` and `mint_y` from on-chain `Config` after `initialize`.

**Slippage:** pass `min_lp` on deposit, `min_x` / `min_y` on withdraw, and `min_out` on swap. Precompute quotes off-chain with the same parameters the program uses:

```rust
let pool = PoolState::new(vault_x, vault_y, lp_supply, fee_bps);
let quote = pool.swap(amount, Side::Y, min_out)?;
```

Use `OperationSide` in instruction builders; use `Side` when calling the cpmm helper directly.

## Dependencies

- **anchor-lang** `1.0.x` (feature `init-if-needed` for user LP ATA on deposit/withdraw).
- **anchor-spl** `1.0.x` — SPL Token mints and ATAs.

Curve math: workspace crate [`crates/cpmm/`](../../crates/cpmm/README.md) (no external dependency).

## Build

From the workspace root: `anchor build` (see root [README.md](../../README.md)).

## Tests

### CPMM unit tests — `crates/cpmm/tests/cpmm.rs`

28 tests covering swap/deposit/withdraw formulas, slippage errors, fee bounds, and single-sided quotes. Run:

```bash
cargo test -p cpmm
```

`CpmmError` → `AmmError` mapping is tested in `programs/amm/src/error.rs`.

### Program integration tests — `tests/tests.rs`

LiteSVM in-process tests: load `amm.so`, create mints, fund a payer, send transactions.

| Test                               | What it checks                                                       |
| ---------------------------------- | -------------------------------------------------------------------- |
| `test_initialize`                  | Config fields, mints, fee, seed, stored bumps.                       |
| `test_deposit`                     | First balanced deposit fills vaults and mints LP.                    |
| `test_withdraw`                    | Partial LP burn; vault ratio unchanged when no swap occurred.         |
| `test_swap`                        | X→Y swap, k increases, LP unchanged.                                 |
| `test_swap_y_for_x`                | Y→X output matches cpmm quote exactly.                               |
| `test_second_deposit_mints_expected_lp` | Second deposit adds proportional LP and vault tokens.           |
| `test_swap_slippage_exceeded`      | Swap fails when `min_out` is too high.                               |
| `test_locked_pool_rejects_*`       | Deposit, withdraw, swap fail when pool is locked.                    |
| `test_unauthorized_update_config`  | Non-authority cannot update config.                                  |
| `test_update_config`               | Authority can update fee, transfer admin, and lock the pool.         |

Run:

```bash
cargo test -p amm --test tests
```
