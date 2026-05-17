# Turbine Vault (Anchor workspace)

Solana programs and TypeScript tests collected in one Anchor workspace. Configuration lives in [`Anchor.toml`](Anchor.toml) (local program IDs, provider, test script).

## Programs

In-depth architecture, instructions, client notes, and test details live next to each crate:

| Program    | Documentation                                                                                                                                                          |
| ---------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **vault**  | [programs/vault/README.md](programs/vault/README.md) — per-user SOL vault (PDAs, `initialize` / `deposit` / `withdraw` / `close`), Rust LiteSVM tests, TS Mocha suite. |
| **escrow** | Source: [`programs/escrow/`](programs/escrow/) — add a `README.md` there when you document flows and accounts.                                                         |

## Quick commands

From this directory:

- **Init:** `anchor init`
- **Add new programs:** `anchor new`
- **Build:** `anchor build`
- **TypeScript tests:** `anchor test` (uses the `test` script in `Anchor.toml`)
- **Rust tests (vault + LiteSVM):** `cargo test -p vault` (after a successful `anchor build` so `target/deploy/vault.so` exists)

## Other docs

- [runbooks/README.md](runbooks/README.md)
