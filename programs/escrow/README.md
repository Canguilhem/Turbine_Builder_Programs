# Escrow (Anchor)

Two-sided SPL token escrow: the **maker** locks **mint A** in a vault controlled by an escrow PDA and sets how much **mint B** they want from a **taker**. The taker completes the swap (`take`); the maker can abort and reclaim **mint A** (`refund`).

**Program ID:** `HSQjtk3WCLWicX9Mku925tHpq19B8x8bj39WZXCMrUNR`

## On-chain state

`Escrow` (account discriminator `1` in this crate) stores:

| Field | Meaning |
| ----- | -------- |
| `seed` | Part of the escrow PDA; lets one maker open multiple escrows. |
| `maker` | Maker pubkeys; constrained with `has_one` on later instructions. |
| `mint_a` | Token mint locked in the vault / returned on refund. |
| `mint_b` | Token mint the taker pays to the maker. |
| `receive_amount` | Amount (smallest units) of **B** the taker must pay and of **A** released to the taker on `take` (see instructions). |
| `bump` | Escrow PDA bump seed. |

Constant seed prefix: `ESCROW_SEED = b"escrow"` (`constants.rs`).

## PDAs and vault

**Escrow state PDA**

```text
seeds = [b"escrow", maker, seed.to_le_bytes()]
```

**Token vault** — not a PDA by seed; it is the **associated token account** for `(mint_a, escrow)` (created in `make` with `associated_token::authority = escrow`).

Clients should derive the escrow address with `PublicKey.findProgramAddressSync` (or equivalent) using the same seeds and this program id, then derive the vault ATA from `mint_a` + escrow authority.

## Instructions

### `make` (discriminator `0`)

**Arguments:** `seed`, `receive`, `deposit`.

1. Initializes the **escrow** account and the **vault** (ATA for `mint_a`, authority = escrow).
2. Transfers **`deposit`** units of **mint A** from `maker_ata_a` into the vault via `transfer_checked`.

The **maker** pays rent for new accounts. `receive` is stored as `receive_amount` (used on `take` / sizing on `take`’s A leg).

### `take` (discriminator `1`)

**Who signs:** **taker** (and account list includes **maker** as a system account for rent destinations).

1. **`deposit` (internal):** Transfers **`receive_amount`** of **mint B** from `taker_ata_b` → `maker_ata_b` (`maker`’s ATA for mint B, created if needed).
2. **`withdraw_and_close`:** Transfers **`receive_amount`** of **mint A** from **vault** → `taker_ata_a`, then **closes** the vault (vault rent goes to **maker** per `CloseAccount`).

The **escrow** account is annotated with `close = maker`, so on success Anchor closes it and returns its rent to the **maker**. Mints must match `has_one` on `escrow`; `maker` must match `escrow.maker`.

### `refund` (discriminator `2`)

**Who signs:** **maker**.

1. Transfers the **full** vault balance (`vault.amount`) of **mint A** back to `maker_ata_a`.
2. Closes the vault (rent to **maker**).
3. Closes **escrow** (`close = maker`).

Use this if no taker completes the deal; only the maker can refund.

## Dependencies

- **anchor-lang** `1.0.x` (feature `init-if-needed` for `take`’s `init_if_needed` ATAs).
- **anchor-spl** `1.0.x` — `associated_token`, Token-2022-friendly `token_interface`, SPL program CPIs.

## Client integration checklist

- Pass the correct **token program** interface account (Token vs Token-2022) consistently on every mint/ATA that uses `mint::token_program = token_program`.
- **`make`:** maker signer; pass both mints, maker’s ATA for A, new escrow + vault accounts, ATA + system programs.
- **`take`:** taker signer; supply maker, both mints, taker ATAs, maker’s ATA for B (optional init), escrow + vault with seeds/bump matching on-chain state.
- **`refund`:** maker signer; mint A, maker ATA A, vault, escrow with matching seeds.

IDL / TypeScript clients should use the instruction discriminators above if you assemble transactions manually.

## Build

From the workspace root: `anchor build` (see root [README.md](../../README.md)).

## Tests

There are no program-local or workspace TS tests for **escrow** in this repo yet; add `programs/escrow/tests/*.rs` (e.g. LiteSVM/Mollusk) or `tests/escrow.ts` and wire them the same way as **vault** when you are ready.
