import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { FundmeToken } from "../target/types/fundme_token";
import {
  createMint,
  getOrCreateAssociatedTokenAccount,
  mintTo,
  getAssociatedTokenAddressSync
} from "@solana/spl-token";
import { expect } from "chai";

describe("fundme-token-rounds", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.FundmeToken as Program<FundmeToken>;
  const wallet = provider.wallet as anchor.Wallet;

  let mint: anchor.web3.PublicKey;
  let userAta: anchor.web3.PublicKey;
  let vaultAta: anchor.web3.PublicKey;

  // Global PDA
  const [fundDataPDA] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("fund_data")], program.programId
  );

  // Helper to find the donor PDA for the current round
  const getDonationPDA = (userPubkey: anchor.web3.PublicKey, roundId: number) => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("donation"),
        userPubkey.toBuffer(),
        new anchor.BN(roundId).toArrayLike(Buffer, "le", 8)
      ],
      program.programId
    )[0];
  };

  before(async () => {
    mint = await createMint(provider.connection, wallet.payer, wallet.publicKey, null, 6);
    const userAtaInfo = await getOrCreateAssociatedTokenAccount(
      provider.connection, wallet.payer, mint, wallet.publicKey
    );
    userAta = userAtaInfo.address;
    await mintTo(provider.connection, wallet.payer, mint, userAta, wallet.publicKey, 100_000_000);
    vaultAta = getAssociatedTokenAddressSync(mint, fundDataPDA, true);
  });

  it("Initializes Round 1", async () => {
    await program.methods.initialize().accounts({
      fundData: fundDataPDA,
      tokenMint: mint,
      vaultTokenAccount: vaultAta,
      admin: wallet.publicKey,
    }).rpc();

    const state = await program.account.fundData.fetch(fundDataPDA);
    expect(state.roundId.toNumber()).to.equal(1);
  });

  it("Donates in Round 1", async () => {
    const amount = new anchor.BN(5_000_000); // 5 USDC
    const donationPDA = getDonationPDA(wallet.publicKey, 1);

    await program.methods.fund(amount).accounts({
      user: wallet.publicKey,
      fundData: fundDataPDA,
      userTokenAccount: userAta,
      vaultTokenAccount: vaultAta,
      tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      systemProgram: anchor.web3.SystemProgram.programId,
    }).rpc();

    const record = await program.account.donationRecord.fetch(donationPDA);
    expect(record.amount.toNumber()).to.equal(5_000_000);
  });

  it("Admin triggers Reset (Starts Round 2)", async () => {
    await program.methods.resetRound().accounts({
      fundData: fundDataPDA,
      admin: wallet.publicKey,
    }).rpc();

    const state = await program.account.fundData.fetch(fundDataPDA);
    expect(state.roundId.toNumber()).to.equal(2);
    expect(state.totalRaised.toNumber()).to.equal(0);
  });

  it("User has 0 donations in Round 2 (Fresh Start)", async () => {
    const donationPDA_R2 = getDonationPDA(wallet.publicKey, 2);

    // Attempting to fetch Round 2 record should fail because it hasn't been created yet
    const accountInfo = await provider.connection.getAccountInfo(donationPDA_R2);
    expect(accountInfo).to.be.null;

    // Now donate in Round 2
    const amount = new anchor.BN(2_000_000); // 2 USDC
    await program.methods.fund(amount).accounts({
      user: wallet.publicKey,
      fundData: fundDataPDA,
      userTokenAccount: userAta,
      vaultTokenAccount: vaultAta,
      tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      systemProgram: anchor.web3.SystemProgram.programId,
    }).rpc();

    const record = await program.account.donationRecord.fetch(donationPDA_R2);
    expect(record.amount.toNumber()).to.equal(2_000_000);
  });

  it("Old Round 1 data still exists on-chain", async () => {
    const donationPDA_R1 = getDonationPDA(wallet.publicKey, 1);
    const record = await program.account.donationRecord.fetch(donationPDA_R1);
    // Round 1 data is preserved!
    expect(record.amount.toNumber()).to.equal(5_000_000);
  });
});