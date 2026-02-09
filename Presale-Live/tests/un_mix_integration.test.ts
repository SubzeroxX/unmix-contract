// tests/un_mix_integration.test.ts
import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { UnMix } from "../target/types/un_mix";
import { PublicKey, Keypair, SystemProgram } from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  getOrCreateAssociatedTokenAccount,
  mintTo,
  getAccount,
} from "@solana/spl-token";
import { expect } from "chai";
import {
  setupProgramAndMints,
  airdrop,
  getUserFundPDA,
  assertAccountingIsConsistent,
  UNIT,
} from "./test-helpers";

describe("un_mix - Integration & Stress Tests", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.UnMix as Program<UnMix>;

  let admin: Keypair;
  let usdcMint: PublicKey;
  let usdtMint: PublicKey;
  let globalPDA: PublicKey;
  let authorityPDA: PublicKey;
  let usdcVault: PublicKey;
  let usdtVault: PublicKey;

  before(async () => {
    const setup = await setupProgramAndMints(program, provider);
    admin = setup.admin;
    usdcMint = setup.usdcMint;
    usdtMint = setup.usdtMint;
    globalPDA = setup.globalPDA;
    authorityPDA = setup.authorityPDA;
    usdcVault = setup.usdcVault;
    usdtVault = setup.usdtVault;
  });

  const getUserUsdcAta = async (user: PublicKey) =>
    (
      await getOrCreateAssociatedTokenAccount(
        provider.connection,
        admin,
        usdcMint,
        user,
      )
    ).address;

  const getUserUsdtAta = async (user: PublicKey) =>
    (
      await getOrCreateAssociatedTokenAccount(
        provider.connection,
        admin,
        usdtMint,
        user,
      )
    ).address;

  // -------------------------------------------------------------------------
  // Multi-User Concurrent Funding
  // -------------------------------------------------------------------------
  it("Should handle 50 users funding concurrently", async function () {
    this.timeout(60000); // 60 seconds timeout for concurrency stress

    const numUsers = 50;
    const users = Array.from({ length: numUsers }, () => Keypair.generate());

    await Promise.all(users.map((u) => airdrop(provider.connection, u.publicKey, 5)));

    // Mint tokens to all users
    await Promise.all(
      users.map(async (user) => {
        const ata = await getUserUsdcAta(user.publicKey);
        await mintTo(
          provider.connection,
          admin,
          usdcMint,
          ata,
          admin,
          1000n * UNIT,
        );
      }),
    );

    // Concurrent funding
    const fundPromises = users.map(async (user, idx) => {
      const userFundPDA = getUserFundPDA(program.programId, user.publicKey);
      const userAta = await getUserUsdcAta(user.publicKey);

      const level = (idx % 6) + 1;
      const amount = 50n * UNIT;

      return program.methods
        .fund(level, new BN(amount.toString()))
        .accounts({
          user: user.publicKey,
          global: globalPDA,
          userFund: userFundPDA,
          mint: usdcMint,
          userTokenAccount: userAta,
          vault: usdcVault,
          authority: authorityPDA,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([user])
        .rpc({ skipPreflight: true }); // faster, less strict
    });

    const results = await Promise.allSettled(fundPromises);
    const succeeded = results.filter((r) => r.status === "fulfilled").length;

    expect(succeeded).to.be.greaterThan(numUsers * 0.8); // allow some tx failures

    await assertAccountingIsConsistent(
      program,
      provider.connection,
      globalPDA,
      usdcVault,
      usdtVault,
    );
  });

  // -------------------------------------------------------------------------
  // Cross-Level Funding Patterns
  // -------------------------------------------------------------------------
  it("Should handle user funding multiple levels with both tokens", async () => {
    const multiUser = Keypair.generate();
    await airdrop(provider.connection, multiUser.publicKey);

    const usdcAta = await getUserUsdcAta(multiUser.publicKey);
    const usdtAta = await getUserUsdtAta(multiUser.publicKey);

    await mintTo(
      provider.connection,
      admin,
      usdcMint,
      usdcAta,
      admin,
      10_000n * UNIT,
    );
    await mintTo(
      provider.connection,
      admin,
      usdtMint,
      usdtAta,
      admin,
      10_000n * UNIT,
    );

    const userFundPDA = getUserFundPDA(program.programId, multiUser.publicKey);

    for (let level = 1; level <= 6; level++) {
      const isUsdc = level % 2 === 1;
      const mint = isUsdc ? usdcMint : usdtMint;
      const ata = isUsdc ? usdcAta : usdtAta;
      const vault = isUsdc ? usdcVault : usdtVault;

      await program.methods
        .fund(level, new BN((200n * UNIT).toString()))
        .accounts({
          user: multiUser.publicKey,
          global: globalPDA,
          userFund: userFundPDA,
          mint,
          userTokenAccount: ata,
          vault,
          authority: authorityPDA,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([multiUser])
        .rpc();
    }

    const userFund = await program.account.userFund.fetch(userFundPDA);

    // Check alternating tokens per level
    for (let i = 0; i < 6; i++) {
      const expectedUsdc = (i + 1) % 2 === 1 ? 200n * UNIT : 0n;
      const expectedUsdt = (i + 1) % 2 === 0 ? 200n * UNIT : 0n;

      expect(BigInt(userFund.perLevelUsdc[i].toString())).to.equal(expectedUsdc);
      expect(BigInt(userFund.perLevelUsdt[i].toString())).to.equal(expectedUsdt);
    }
  });

  // -------------------------------------------------------------------------
  // Level Cap Stress Tests
  // -------------------------------------------------------------------------
  it("Should handle approaching level cap with many small transactions", async function () {
    this.timeout(90000);

    const capUser = Keypair.generate();
    await airdrop(provider.connection, capUser.publicKey);

    const usdcAta = await getUserUsdcAta(capUser.publicKey);
    await mintTo(
      provider.connection,
      admin,
      usdcMint,
      usdcAta,
      admin,
      10_000_000n * UNIT // Increase this significantly
    );

    const userFundPDA = getUserFundPDA(program.programId, capUser.publicKey);

    let successCount = 0;
    const targetTxs = 200; // more than needed to hit cap

    for (let i = 0; i < targetTxs; i++) {
      try {
        await program.methods
          .fund(4, new BN((35_000n * UNIT).toString()))
          .accounts({
            user: capUser.publicKey,
            global: globalPDA,
            userFund: userFundPDA,
            mint: usdcMint,
            userTokenAccount: usdcAta,
            vault: usdcVault,
            authority: authorityPDA,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .signers([capUser])
          .rpc();

        successCount++;
      } catch (err) {
        const code = (err as any)?.error?.errorCode?.code;
        if (code === "ExceedsLevelTotal" || code === "LevelNotEnabled") {
          console.log(`Cap reached after ${successCount} successful txs`);
          break;
        }
        throw err;
      }
    }

    expect(successCount).to.be.greaterThan(100); // at least some succeed

    await assertAccountingIsConsistent(
      program,
      provider.connection,
      globalPDA,
      usdcVault,
      usdtVault,
    );
  });

  // -------------------------------------------------------------------------
  // Mixed Token Operations
  // -------------------------------------------------------------------------
  it("Should handle rapid switching between USDC and USDT", async () => {
    const switchUser = Keypair.generate();
    await airdrop(provider.connection, switchUser.publicKey);

    const usdcAta = await getUserUsdcAta(switchUser.publicKey);
    const usdtAta = await getUserUsdtAta(switchUser.publicKey);

    await mintTo(
      provider.connection,
      admin,
      usdcMint,
      usdcAta,
      admin,
      5000n * UNIT,
    );
    await mintTo(
      provider.connection,
      admin,
      usdtMint,
      usdtAta,
      admin,
      5000n * UNIT,
    );

    const userFundPDA = getUserFundPDA(program.programId, switchUser.publicKey);

    for (let i = 0; i < 40; i++) {
      const isUsdc = i % 2 === 0;
      const mint = isUsdc ? usdcMint : usdtMint;
      const ata = isUsdc ? usdcAta : usdtAta;
      const vault = isUsdc ? usdcVault : usdtVault;

      await program.methods
        .fund(4, new BN((25n * UNIT).toString())) // level 4 = no per-account limit
        .accounts({
          user: switchUser.publicKey,
          global: globalPDA,
          userFund: userFundPDA,
          mint,
          userTokenAccount: ata,
          vault,
          authority: authorityPDA,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([switchUser])
        .rpc();
    }

    const userFund = await program.account.userFund.fetch(userFundPDA);
    expect(BigInt(userFund.perLevelUsdc[3].toString())).to.equal(500n * UNIT);
    expect(BigInt(userFund.perLevelUsdt[3].toString())).to.equal(500n * UNIT);
  });

  // -------------------------------------------------------------------------
  // Final State Verification
  // -------------------------------------------------------------------------
  after(async () => {
    await assertAccountingIsConsistent(
      program,
      provider.connection,
      globalPDA,
      usdcVault,
      usdtVault,
    );

    const global = await program.account.global.fetch(globalPDA);
    console.log("\n=== Integration Test Final Summary ===");
    console.log(`Total Funded: ${Number(global.totalFund) / Number(UNIT)} tokens`);
    console.log(`USDC Vault:   ${Number(global.usdcVaultTotal) / Number(UNIT)}`);
    console.log(`USDT Vault:   ${Number(global.usdtVaultTotal) / Number(UNIT)}`);
  });
});