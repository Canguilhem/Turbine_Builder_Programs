import * as anchor from "@anchor-lang/core";
// import {
//   Commitment,
//   LAMPORTS_PER_SOL,
//   PublicKey,
//   SystemProgram,
// } from "@solana/web3.js";
export const commitment = "confirmed";

export const fundWallets = async (
  provider: anchor.AnchorProvider,
  wallets: anchor.web3.PublicKey[],
) => {
  if (!provider.connection.rpcEndpoint.includes("devnet")) {
    await Promise.all(
      wallets.map(async (walletPk) => {
        provider.connection.requestAirdrop(
          walletPk,
          10 * anchor.web3.LAMPORTS_PER_SOL,
        );
      }),
    );
  }
  await checkSolBalance(provider, wallets, 9);
};

export const checkSolBalance = async (
  provider: anchor.AnchorProvider,
  pubKeys: anchor.web3.PublicKey[],
  decimals = 9,
) => {
  for (const walletPk of pubKeys) {
    const balance = await provider.connection.getBalance(walletPk);
    console.log(
      `${walletPk.toBase58().slice(0, 6)} balance: ${formatTokens(
        balance.toString(),
        decimals,
      )}`,
    );
  }
};

export const confrimTx = async (
  connection: anchor.web3.Connection,
  signature: string,
  operationLabel: string,
) => {
  const latestBlockHash = await connection.getLatestBlockhash();

  await connection.confirmTransaction(
    {
      signature,
      ...latestBlockHash,
    },
    commitment,
  );
  console.log(`${operationLabel} signature: ${signature}`);
};

const formatTokens = (amount: string, decimals = 6) =>
  (Number(amount) / 10 ** decimals).toLocaleString();
