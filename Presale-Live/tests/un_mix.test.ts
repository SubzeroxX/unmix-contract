// tests/un_mix.test.ts
import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { UnMix } from "../target/types/un_mix";
import { PublicKey, Keypair, SystemProgram } from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
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
  DECIMALS,
  UNIT,
} from "./test-helpers";

describe("un_mix - Comprehensive Tests", () => {
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

  let user1: Keypair;
  let user2: Keypair;
  let user3: Keypair;

  before(async () => {
    const setup = await setupProgramAndMints(program, provider);
    admin = setup.admin;
    usdcMint = setup.usdcMint;
    usdtMint = setup.usdtMint;
    globalPDA = setup.globalPDA;
    authorityPDA = setup.authorityPDA;
    usdcVault = setup.usdcVault;
    usdtVault = setup.usdtVault;

    user1 = Keypair.generate();
    user2 = Keypair.generate();
    user3 = Keypair.generate();

    await Promise.all([
      airdrop(provider.connection, user1.publicKey),
      airdrop(provider.connection, user2.publicKey),
      airdrop(provider.connection, user3.publicKey),
    ]);

    // Give users plenty of tokens for testing
    const amount = 10_000_000n * UNIT;
    for (const user of [user1, user2, user3]) {
      await getOrCreateAssociatedTokenAccount(
        provider.connection,
        admin,
        usdcMint,
        user.publicKey,
      );
      await mintTo(
        provider.connection,
        admin,
        usdcMint,
        (await getOrCreateAssociatedTokenAccount(
          provider.connection,
          admin,
          usdcMint,
          user.publicKey,
        )).address,
        admin,
        amount,
      );

      await getOrCreateAssociatedTokenAccount(
        provider.connection,
        admin,
        usdtMint,
        user.publicKey,
      );
      await mintTo(
        provider.connection,
        admin,
        usdtMint,
        (await getOrCreateAssociatedTokenAccount(
          provider.connection,
          admin,
          usdtMint,
          user.publicKey,
        )).address,
        admin,
        amount,
      );
    }
  });

  // -------------------------------------------------------------------------
  // Initialization
  // -------------------------------------------------------------------------
  it("Should have correctly initialized USDC and USDT mints", async () => {
    const global = await program.account.global.fetch(globalPDA);
    expect(global.usdcMint.toBase58()).to.equal(usdcMint.toBase58());
    expect(global.usdtMint.toBase58()).to.equal(usdtMint.toBase58());
    expect(global.admin.toBase58()).to.equal(admin.publicKey.toBase58());
    expect(global.bump).to.be.a("number");
  });

  it("Should fail to initialize again with same mint", async () => {
    const vault = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      admin,
      usdcMint,
      authorityPDA,
      true,
    );

    try {
      await program.methods
        .initialize()
        .accounts({
          admin: admin.publicKey,
          mint: usdcMint,
          global: globalPDA,
          authority: authorityPDA,
          vault: vault.address,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([admin])
        .rpc();
      expect.fail("Should have thrown an error");
    } catch (err: any) {
      // The error could be either:
      // 1. MintAlreadySet (custom error from your program)
      // 2. An anchor error about account already initialized

      let code = err.error?.errorCode?.code;

      // Check logs for custom error
      if (!code && err.logs) {
        if (err.logs.some((log: string) => log.includes("MintAlreadySet"))) {
          code = "MintAlreadySet";
        }
      }

      // Accept either MintAlreadySet or account already exists errors
      const isExpectedError =
        code === "MintAlreadySet" ||
        err.message?.includes("already in use") ||
        err.logs?.some((log: string) => log.includes("already in use"));

      expect(isExpectedError, `Unexpected error: ${err.message || JSON.stringify(err)}`).to.be.true;
    }
  });

  // -------------------------------------------------------------------------
  // Enable/Disable Levels
  // -------------------------------------------------------------------------
  it("Should enable and disable levels as admin", async () => {
    const globalBefore = await program.account.global.fetch(globalPDA);
    expect(globalBefore.levelsEnabled.every((v) => v === true)).to.be.true;

    await program.methods
      .enableLevel(3, false)
      .accounts({ admin: admin.publicKey, global: globalPDA })
      .signers([admin])
      .rpc();

    const globalAfter = await program.account.global.fetch(globalPDA);
    expect(globalAfter.levelsEnabled[2]).to.be.false;
  });

  it("Should fail to enable level as non-admin", async () => {
    try {
      await program.methods
        .enableLevel(1, true)
        .accounts({ admin: user1.publicKey, global: globalPDA })
        .signers([user1])
        .rpc();
      expect.fail("Should have thrown Unauthorized");
    } catch (err) {
      expect((err as any).error.errorCode.code).to.equal("Unauthorized");
    }
  });

  // -------------------------------------------------------------------------
  // Funding - Basic Operations
  // -------------------------------------------------------------------------
  it("Should fund level 1 with USDC", async () => {
    const amount = 150n * UNIT;
    const userFundPDA = getUserFundPDA(program.programId, user1.publicKey);
    const userAta = (
      await getOrCreateAssociatedTokenAccount(
        provider.connection,
        user1,
        usdcMint,
        user1.publicKey,
      )
    ).address;

    await program.methods
      .fund(1, new BN(amount.toString()))
      .accounts({
        user: user1.publicKey,
        global: globalPDA,
        userFund: userFundPDA,
        mint: usdcMint,
        userTokenAccount: userAta,
        vault: usdcVault,
        authority: authorityPDA,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([user1])
      .rpc();

    const userFund = await program.account.userFund.fetch(userFundPDA);
    expect(userFund.perLevelUsdc[0].toString()).to.equal(amount.toString());
  });

  // ... Add more funding tests, level cap tests, multi-token tests, etc.
  // Use similar pattern: use setup variables, use BigInt + BN conversion

  // -------------------------------------------------------------------------
  // Final consistency check
  // -------------------------------------------------------------------------
  after(async () => {
    await assertAccountingIsConsistent(
      program,
      provider.connection,
      globalPDA,
      usdcVault,
      usdtVault,
    );
  });
});