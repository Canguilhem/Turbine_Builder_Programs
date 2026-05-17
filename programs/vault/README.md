# Vault (Anchor)

Per-user SOL vault on Solana: each wallet gets its own program-derived accounts for state and for holding lamports. Only the user who initialized the vault can deposit, withdraw, or close it.

**Program ID:** `3YqWHvBo8ytiLYQrMGvoNL7MKvNNvECnZJHRj7zdoP82`

## Architecture

Two PDAs are derived per **user** (the signer who runs `initialize`):

| Account                    | Role                                                                              | Seeds                     |
| -------------------------- | --------------------------------------------------------------------------------- | ------------------------- |
| **State** (`VaultState`)   | Stores bump seeds for both PDAs; closed on `close`.                               | `[b"state", user]`        |
| **Vault** (system account) | Holds SOL (deposits). Has no custom data; signed via PDA seeds on withdraw/close. | `[b"vault", vault_state]` |

`VaultState` is a small on-chain account: `vault_bump` and `state_bump` so later instructions can validate PDAs without recomputing bumps at the client for the state account (the vault bump is still required for CPI signers).

## Instructions

### `initialize`

- Creates the **state** PDA (payer = user).
- Funds the **vault** PDA with the minimum rent-exempt balance for an empty system account (lamports transferred from user → vault via `system_program::transfer`).
- Writes both bumps into `VaultState`.

### `deposit`

- User must match the state PDA seeds (`[b"state", user]`).
- Transfers `amount` lamports from the user to the vault PDA.

### `withdraw`

- Same account constraints as deposit.
- CPI: vault PDA signs with seeds `[b"vault", vault_state, vault_bump]` and sends `amount` lamports to the user.

### `close`

- Closes `vault_state` and sends its rent to the user (Anchor `close = user` on the state account).
- CPI: drains **all** lamports from the vault PDA to the user (`vault.lamports()`), with the vault PDA as signer.
- After a successful close, both the state account and the vault’s held balance should be fully recovered by the user (subject to normal rent and transaction fees).

## Client integration

Derive addresses deterministically:

1. `(vault_state, state_bump) = find_program_address([b"state", user], program_id)`
2. `(vault, vault_bump) = find_program_address([b"vault", vault_state], program_id)`

Pass `user`, `vault_state`, `vault`, and `system_program` as required by each instruction’s `#[derive(Accounts)]` struct.

## Build

From the **workspace root** (parent of this folder), run `anchor build`. That compiles the program and writes `target/deploy/vault.so`, which the Rust LiteSVM tests load directly.

## Tests

### Rust (LiteSVM) — `programs/vault/tests/vault.rs`

In-process integration tests: load `vault.so`, fund a payer, send transactions through LiteSVM, assert PDAs and lamport changes.

| Test | What it checks |
| ---- | ---------------- |
| `test_vault_initialize` | After `initialize`, `VaultState` exists and `vault_bump` / `state_bump` match `find_program_address`. |
| `test_deposit_widthraw_close` | Deposit increases the vault by the deposited amount; withdraw decreases it; `close` removes both accounts and the user gains lamports (rent + drained vault). |

**Run:** from the workspace root (after `anchor build`): `cargo test -p vault`.  
**Or:** `cd programs/vault && cargo test`.

### TypeScript (Anchor + Mocha) — `tests/vault.ts`

End-to-end tests against whatever cluster **`AnchorProvider.env()`** uses. This repo’s `Anchor.toml` sets `[provider] cluster = "localnet"`, and `[scripts] test` runs Mocha via Yarn on `tests/**/*.ts`. The client uses **`@anchor-lang/core`** (see root `package.json`). Transactions are confirmed with `confrimTx` in `tests/utils.ts` (`commitment: "confirmed"`).

**Setup (shared `before` hook):** generates a fresh `user` keypair, funds it with `fundWallets` in `tests/utils.ts` (airdrop on non-devnet endpoints, then balance logging), derives `vaultStatePda` and `vaultPda` with the same seeds as on-chain. Each instruction uses `accountsStrict` and signs with `user`.

The four specs assume **one vault per file run**: they execute in order on the same accounts (`initialize` → deposit → withdraw → `close`). Reordering or running specs in isolation would require adjusting hooks or using separate `describe` blocks.

| Spec | What it checks |
| ---- | ---------------- |
| `Is initialized!` | `initialize` succeeds; on-chain `vaultState` matches expected bumps. |
| `deposit 1 Sol into the vault` | Vault balance increases by 1 SOL; user balance drops by at least that much (fees). |
| `withdraw .5 Sol from the vault` | Vault decreases by 0.5 SOL; user balance increases. |
| `closes the vault & withdraw funds` | `close` succeeds; vault lamports read as `0`; user balance increases. |

**Run:** from the workspace root: `anchor test` (builds/deploys to localnet as configured, then runs the `yarn` test script from `Anchor.toml`).  
To run only the TS file with the same Mocha invocation:  
`yarn run ts-mocha -p ./tsconfig.json -t 1000000 "tests/**/*.ts"`.

The commented-out SPL mint/ATA code in `vault.ts` is leftover setup; the vault program under test is SOL-only.
