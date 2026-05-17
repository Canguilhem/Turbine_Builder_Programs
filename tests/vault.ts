import * as anchor from "@anchor-lang/core";
import { Program } from "@anchor-lang/core";
import { Vault } from "../target/types/vault";
import { BN } from "bn.js";
import { expect } from "chai";
import { confrimTx, fundWallets } from "./utils";

describe("vault tests", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  let user, user_pk, vaultStatePda, vaultPda, stateBump, vaultBump;

  const program = anchor.workspace.vault as Program<Vault>;
  before(async () => {
    // new user
    // Works well on localnet - TODO use fixed keypairs on devnet
    user = anchor.web3.Keypair.generate();
    user_pk = user.publicKey;

    // fund account via request airdrop
    await fundWallets(provider, [user_pk]);

    // derive PDAs
    [vaultStatePda, stateBump] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("state"), user_pk.toBuffer()],
      program.programId,
    );

    [vaultPda, vaultBump] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("vault"), vaultStatePda.toBuffer()],
      program.programId,
    );

    console.log("setup: ", {
      rpc: provider.connection.rpcEndpoint,
      vaultStatePda: {
        pda: vaultStatePda.toString(),
        bump: stateBump,
      },
      vaultPda: {
        pda: vaultPda.toString(),
        bump: vaultBump,
      },
    });
  });

  it("Is initialized!", async () => {
    const signature = await program.methods
      .initialize()
      .accountsStrict({
        user: user_pk,
        vaultState: vaultStatePda,
        vault: vaultPda,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([user])
      .rpc();

    await confrimTx(provider.connection, signature, "vault init");

    const vaultState = await program.account.vaultState.fetch(vaultStatePda);
    expect(vaultState.vaultBump).to.equal(vaultBump);
    expect(vaultState.stateBump).to.equal(stateBump);
  });

  it("deposit 1 Sol into the vault", async () => {
    const depositAmount = 1 * anchor.web3.LAMPORTS_PER_SOL;
    const initVaultBalance = await provider.connection.getBalance(vaultPda);
    const initUserBalance = await provider.connection.getBalance(user_pk);

    // deposit
    const signature = await program.methods
      .deposit(new BN(depositAmount))
      .accountsStrict({
        user: user_pk,
        vaultState: vaultStatePda,
        vault: vaultPda,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([user])
      .rpc();

    await confrimTx(provider.connection, signature, "vault deposit");

    const finalVaultBalance = await provider.connection.getBalance(vaultPda);
    const finalUserBalance = await provider.connection.getBalance(user_pk);

    expect(finalVaultBalance).to.equal(initVaultBalance + depositAmount);
    expect(finalUserBalance).to.be.lessThanOrEqual(
      initUserBalance - depositAmount,
    );
  });
  it("withdraw .5 Sol from the vault", async () => {
    const withdrawAmount = 0.5 * anchor.web3.LAMPORTS_PER_SOL;
    const initVaultBalance = await provider.connection.getBalance(vaultPda);
    const initUserBalance = await provider.connection.getBalance(user_pk);

    // deposit
    const signature = await program.methods
      .withdraw(new BN(withdrawAmount))
      .accountsStrict({
        user: user_pk,
        vaultState: vaultStatePda,
        vault: vaultPda,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([user])
      .rpc();

    await confrimTx(provider.connection, signature, "vault withdraw");

    const finalVaultBalance = await provider.connection.getBalance(vaultPda);
    const finalUserBalance = await provider.connection.getBalance(user_pk);

    expect(finalVaultBalance).to.equal(initVaultBalance - withdrawAmount);
    expect(finalUserBalance).to.be.greaterThan(initUserBalance);
  });
  it("closes the vault & withdraw funds", async () => {
    const initVaultBalance = await provider.connection.getBalance(vaultPda);
    const initUserBalance = await provider.connection.getBalance(user_pk);

    // deposit
    const signature = await program.methods
      .close()
      .accountsStrict({
        user: user_pk,
        vaultState: vaultStatePda,
        vault: vaultPda,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([user])
      .rpc();

    await confrimTx(provider.connection, signature, "vault close");

    const finalVaultBalance = await provider.connection.getBalance(vaultPda);
    const finalUserBalance = await provider.connection.getBalance(user_pk);
    const vaultStateInfo = await provider.connection.getAccountInfo(
      vaultStatePda,
    );

    expect(finalVaultBalance).to.equal(0);
    expect(vaultStateInfo).to.be.null;
    expect(finalUserBalance).to.be.greaterThan(initUserBalance);
  });
});
