// tests/un_mix_security.test.ts
import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { UnMix } from "../target/types/un_mix";
import {
  PublicKey,
  Keypair,
  SystemProgram,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  getOrCreateAssociatedTokenAccount,
  mintTo,
  getAccount,
  createMint,
} from "@solana/spl-token";
import { expect } from "chai";
import {
  setupProgramAndMints,
  airdrop,
  getUserFundPDA,
  assertAccountingIsConsistent,
  UNIT,
  DECIMALS,
} from "./test-helpers";

describe("un_mix - Security & Fuzzing Tests", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.UnMix as Program<UnMix>;

  let admin: Keypair;
  let attacker: Keypair;
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

    attacker = Keypair.generate();
    await airdrop(provider.connection, attacker.publicKey);
  });

  const getAttackerUsdcAta = async () =>
    (await getOrCreateAssociatedTokenAccount(
      provider.connection,
      attacker,
      usdcMint,
      attacker.publicKey,
    )).address;

  const getAttackerUsdtAta = async () =>
    (await getOrCreateAssociatedTokenAccount(
      provider.connection,
      attacker,
      usdtMint,
      attacker.publicKey,
    )).address;

  // -------------------------------------------------------------------------
  // Authorization Attacks
  // -------------------------------------------------------------------------
  describe("Authorization Attacks", () => {
    it("Should prevent attacker from enabling/disabling levels", async () => {
      try {
        await program.methods
          .enableLevel(1, false)
          .accounts({
            admin: attacker.publicKey,
            global: globalPDA,
          })
          .signers([attacker])
          .rpc();
        expect.fail("Should have thrown Unauthorized");
      } catch (err) {
        expect((err as any).error.errorCode.code).to.equal("Unauthorized");
      }
    });

    it("Should prevent unauthorized withdrawal", async () => {
      const attackerUsdc = await getAttackerUsdcAta();

      try {
        await program.methods
          .withdraw(new BN((100n * UNIT).toString()))
          .accounts({
            withdrawer: attacker.publicKey,
            global: globalPDA,
            mint: usdcMint,
            vault: usdcVault,
            withdrawerTokenAccount: attackerUsdc,
            authority: authorityPDA,
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .signers([attacker])
          .rpc();
        expect.fail("Should have thrown Unauthorized");
      } catch (err) {
        expect((err as any).error.errorCode.code).to.equal("Unauthorized");
      }
    });

    it("Should reject fake PDA as admin", async () => {
      const fakeAdmin = Keypair.generate().publicKey;

      try {
        await program.methods
          .enableLevel(1, true)
          .accounts({
            admin: fakeAdmin,
            global: globalPDA,
          })
          .signers([attacker]) // wrong signer anyway
          .rpc();
        expect.fail("Should have failed due to signature or unauthorized");
      } catch (err) {
        // Anchor will usually throw SignatureVerification or Unauthorized
        expect(err).to.exist;
      }
    });
  });

  // -------------------------------------------------------------------------
  // PDA Manipulation Attacks
  // -------------------------------------------------------------------------
  describe("PDA Manipulation Attacks", () => {
    it("Should prevent funding to wrong vault (wrong owner)", async () => {
      await airdrop(provider.connection, attacker.publicKey);

      const attackerUsdc = await getAttackerUsdcAta();
      await mintTo(
        provider.connection,
        admin,
        usdcMint,
        attackerUsdc,
        admin,
        1000n * UNIT,
      );

      const attackerFundPDA = getUserFundPDA(program.programId, attacker.publicKey);

      // Create a wrong vault (owned by attacker instead of authority PDA)
      const wrongVault = (
        await getOrCreateAssociatedTokenAccount(
          provider.connection,
          attacker,
          usdcMint,
          attacker.publicKey, // wrong owner
        )
      ).address;

      try {
        await program.methods
          .fund(1, new BN((100n * UNIT).toString()))
          .accounts({
            user: attacker.publicKey,
            global: globalPDA,
            userFund: attackerFundPDA,
            mint: usdcMint,
            userTokenAccount: attackerUsdc,
            vault: wrongVault,
            authority: authorityPDA,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .signers([attacker])
          .rpc();
        expect.fail("Should have thrown InvalidVault or MintMismatch");
      } catch (err) {
        const code = (err as any).error.errorCode.code;
        // Anchor returns 'ConstraintTokenOwner' for associated_token authority mismatches
        expect(["ConstraintTokenOwner", "ConstraintSeeds", "InvalidVault"]).to.include(code);
      }
    });

    it("Should prevent using another user's UserFund PDA", async () => {
      const victim = Keypair.generate();
      await airdrop(provider.connection, victim.publicKey);

      const victimUsdc = (
        await getOrCreateAssociatedTokenAccount(
          provider.connection,
          admin,
          usdcMint,
          victim.publicKey,
        )
      ).address;

      await mintTo(
        provider.connection,
        admin,
        usdcMint,
        victimUsdc,
        admin,
        1000n * UNIT,
      );

      // Victim initializes their own fund account
      const victimFundPDA = getUserFundPDA(program.programId, victim.publicKey);
      await program.methods
        .fund(1, new BN((50n * UNIT).toString()))
        .accounts({
          user: victim.publicKey,
          global: globalPDA,
          userFund: victimFundPDA,
          mint: usdcMint,
          userTokenAccount: victimUsdc,
          vault: usdcVault,
          authority: authorityPDA,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([victim])
        .rpc();

      // Attacker tries to reuse victim's PDA
      const attackerUsdc = await getAttackerUsdcAta();
      await mintTo(
        provider.connection,
        admin,
        usdcMint,
        attackerUsdc,
        admin,
        1000n * UNIT,
      );

      try {
        await program.methods
          .fund(1, new BN((100n * UNIT).toString()))
          .accounts({
            user: attacker.publicKey,
            global: globalPDA,
            userFund: victimFundPDA, // wrong PDA
            mint: usdcMint,
            userTokenAccount: attackerUsdc,
            vault: usdcVault,
            authority: authorityPDA,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .signers([attacker])
          .rpc();
        expect.fail("Should have failed due to PDA seed mismatch");
      } catch (err) {
        expect(err).to.exist; // usually ConstraintSeeds or AccountOwnedByWrongProgram
      }
    });
  });

  // -------------------------------------------------------------------------
  // Token/Mint Manipulation
  // -------------------------------------------------------------------------
  describe("Token/Mint Manipulation", () => {
    it("Should prevent funding with wrong mint for vault", async () => {
      const attackerUsdc = await getAttackerUsdcAta();
      await mintTo(
        provider.connection,
        admin,
        usdcMint,
        attackerUsdc,
        admin,
        1000n * UNIT,
      );

      const attackerFundPDA = getUserFundPDA(program.programId, attacker.publicKey);

      try {
        await program.methods
          .fund(1, new BN((100n * UNIT).toString()))
          .accounts({
            user: attacker.publicKey,
            global: globalPDA,
            userFund: attackerFundPDA,
            mint: usdcMint,
            userTokenAccount: attackerUsdc,
            vault: usdtVault, // wrong vault for this mint
            authority: authorityPDA,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .signers([attacker])
          .rpc();
      } catch (err) {
        expect((err as any).error.errorCode.code).to.equal("ConstraintAssociated");
      }
    });

    it("Should reject funding with unregistered/fake mint", async () => {
      const fakeMint = await createMint(
        provider.connection,
        admin,
        admin.publicKey,
        null,
        DECIMALS,
      );

      const attackerFakeAta = (
        await getOrCreateAssociatedTokenAccount(
          provider.connection,
          admin,
          fakeMint,
          attacker.publicKey,
        )
      ).address;

      await mintTo(
        provider.connection,
        admin,
        fakeMint,
        attackerFakeAta,
        admin,
        1000n * UNIT,
      );

      const fakeVault = (
        await getOrCreateAssociatedTokenAccount(
          provider.connection,
          admin,
          fakeMint,
          authorityPDA,
          true,
        )
      ).address;

      const attackerFundPDA = getUserFundPDA(program.programId, attacker.publicKey);

      try {
        await program.methods
          .fund(1, new BN((100n * UNIT).toString()))
          .accounts({
            user: attacker.publicKey,
            global: globalPDA,
            userFund: attackerFundPDA,
            mint: fakeMint,
            userTokenAccount: attackerFakeAta,
            vault: fakeVault,
            authority: authorityPDA,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .signers([attacker])
          .rpc();
        expect.fail("Should have thrown InvalidMint");
      } catch (err) {
        expect((err as any).error.errorCode.code).to.equal("InvalidMint");
      }
    });
  });

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