import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { UnMix } from "../target/types/un_mix";
import { PublicKey, SystemProgram, SYSVAR_RENT_PUBKEY } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID, getAssociatedTokenAddressSync, createMint, createAssociatedTokenAccount, mintTo } from "@solana/spl-token";
import { expect } from "chai";

describe("un_mix", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.UnMix as Program<UnMix>;

  const admin = provider.wallet;
  const user = anchor.web3.Keypair.generate();
  const user2 = anchor.web3.Keypair.generate();
  const user3 = anchor.web3.Keypair.generate();

  let usdcMint: PublicKey;
  let vaultAuthority: PublicKey;
  let vault: PublicKey;
  let global: PublicKey;
  let userFund: PublicKey;
  let user2Fund: PublicKey;
  let user3Fund: PublicKey;
  let adminUsdc: PublicKey;
  let userUsdc: PublicKey;
  let user2Usdc: PublicKey;
  let user3Usdc: PublicKey;

  const decimals = 6;
  const unit = 10 ** decimals;

  const levelTotals = [
    5_400_000 * unit,
    88_000_000 * unit,
    173_000_000 * unit,
    116_250_000 * unit,
    45_800_000 * unit,
    37_200_000 * unit,
  ];

  const levelMaxPerAccount = [
    500 * unit,
    999 * unit,
    5000 * unit,
    Number.MAX_SAFE_INTEGER,
    Number.MAX_SAFE_INTEGER,
    Number.MAX_SAFE_INTEGER,
  ];

  before(async () => {
    // Create USDC mint
    usdcMint = await createMint(
      provider.connection,
      admin.payer,
      admin.publicKey,
      null,
      decimals
    );

    // Global account PDA
    [global] = PublicKey.findProgramAddressSync(
      [Buffer.from("global")],
      program.programId
    );

    // Vault authority PDA
    [vaultAuthority] = PublicKey.findProgramAddressSync(
      [Buffer.from("authority")],
      program.programId
    );

    // Vault ATA (associated token account for authority)
    vault = getAssociatedTokenAddressSync(usdcMint, vaultAuthority, true);

    // Admin USDC ATA
    adminUsdc = getAssociatedTokenAddressSync(usdcMint, admin.publicKey, true);

    // Create admin USDC account
    const adminUsdcInfo = await provider.connection.getAccountInfo(adminUsdc);
    if (!adminUsdcInfo) {
      await createAssociatedTokenAccount(
        provider.connection,
        admin.payer,
        usdcMint,
        admin.publicKey
      );
    }

    // Mint USDC to admin
    await mintTo(
      provider.connection,
      admin.payer,
      usdcMint,
      adminUsdc,
      admin.publicKey,
      5_000_000 * unit
    );

    // Airdrop SOL to user
    await provider.connection.requestAirdrop(user.publicKey, anchor.web3.LAMPORTS_PER_SOL);
    await provider.connection.requestAirdrop(user2.publicKey, anchor.web3.LAMPORTS_PER_SOL);
    await new Promise((resolve) => setTimeout(resolve, 1000));

    // User's USDC ATA
    userUsdc = getAssociatedTokenAddressSync(usdcMint, user.publicKey, true);

    // Create user's USDC account
    await createAssociatedTokenAccount(
      provider.connection,
      user,
      usdcMint,
      user.publicKey
    );

    // Mint USDC to user
    await mintTo(
      provider.connection,
      admin.payer,
      usdcMint,
      userUsdc,
      admin.publicKey,
      100_000 * unit
    );

    // User 2 setup
    user2Usdc = getAssociatedTokenAddressSync(usdcMint, user2.publicKey, true);
    await createAssociatedTokenAccount(
      provider.connection,
      user2,
      usdcMint,
      user2.publicKey
    );

    await mintTo(
      provider.connection,
      admin.payer,
      usdcMint,
      user2Usdc,
      admin.publicKey,
      100_000 * unit
    );

    // User 3 setup
    user3Usdc = getAssociatedTokenAddressSync(usdcMint, user3.publicKey, true);
    await provider.connection.requestAirdrop(user3.publicKey, anchor.web3.LAMPORTS_PER_SOL);
    await new Promise((resolve) => setTimeout(resolve, 500));
    await createAssociatedTokenAccount(
      provider.connection,
      user3,
      usdcMint,
      user3.publicKey
    );

    await mintTo(
      provider.connection,
      admin.payer,
      usdcMint,
      user3Usdc,
      admin.publicKey,
      100_000 * unit
    );

    // User's fund account PDA
    [userFund] = PublicKey.findProgramAddressSync(
      [Buffer.from("user_fund"), user.publicKey.toBuffer()],
      program.programId
    );

    // User2's fund account PDA
    [user2Fund] = PublicKey.findProgramAddressSync(
      [Buffer.from("user_fund"), user2.publicKey.toBuffer()],
      program.programId
    );

    // User3's fund account PDA
    [user3Fund] = PublicKey.findProgramAddressSync(
      [Buffer.from("user_fund"), user3.publicKey.toBuffer()],
      program.programId
    );
  });

  it("Initializes the program", async () => {
    await program.methods
      .initialize()
      .accounts({
        admin: admin.publicKey,
        usdcMint,
        global,
        authority: vaultAuthority,
        vault,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const globalAccount = await program.account.global.fetch(global);
    expect(globalAccount.admin).to.deep.equal(admin.publicKey);
    expect(globalAccount.usdcMint).to.deep.equal(usdcMint);
    expect(globalAccount.totalFund.toNumber()).to.equal(0);
    expect(globalAccount.levelsEnabled.every((e) => !e)).to.be.true;
    expect(globalAccount.totalPerLevel.every((l) => l.toNumber() === 0)).to.be.true;
  });

  it("Fails to initialize again", async () => {
    try {
      await program.methods
        .initialize()
        .accounts({
          admin: admin.publicKey,
          usdcMint,
          global,
          authority: vaultAuthority,
          vault,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
      expect.fail("Should have failed");
    } catch (err: any) {
      expect(err).to.exist;
      // ConstraintInit is error code 2000
      const hasConstraintInit = 
        err.error?.errorCode?.code === "ConstraintInit" ||
        err.error?.errorCode?.number === 2000 ||
        err.message?.includes("ConstraintInit") ||
        err.message?.includes("already in use");
      expect(hasConstraintInit).to.be.true;
    }
  });

  it("Enables level 1 as admin", async () => {
    await program.methods
      .enableLevel(1, true)
      .accounts({
        admin: admin.publicKey,
        global,
      })
      .rpc();

    const globalAccount = await program.account.global.fetch(global);
    expect(globalAccount.levelsEnabled[0]).to.be.true;
  });

  it("Fails to enable level as non-admin", async () => {
    try {
      await program.methods
        .enableLevel(2, true)
        .accounts({
          admin: user.publicKey,
          global,
        })
        .signers([user])
        .rpc();
      expect.fail("Should have failed");
    } catch (err: any) {
      expect(err.error.errorCode.code).to.equal("Unauthorized");
    }
  });

  it("Fails with invalid level", async () => {
    try {
      await program.methods
        .enableLevel(7, true)
        .accounts({
          admin: admin.publicKey,
          global,
        })
        .rpc();
      expect.fail("Should have failed");
    } catch (err: any) {
      expect(err.error.errorCode.code).to.equal("InvalidLevel");
    }
  });

  it("Funds level 1 with minimum amount", async () => {
    await program.methods
      .fund(1, new anchor.BN(1 * unit))
      .accounts({
        user: user.publicKey,
        global,
        userFund,
        usdcMint,
        userUsdc,
        vault,
        authority: vaultAuthority,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([user])
      .rpc();

    const globalAccount = await program.account.global.fetch(global);
    expect(globalAccount.totalFund.toNumber()).to.equal(1 * unit);
    expect(globalAccount.totalPerLevel[0].toNumber()).to.equal(1 * unit);

    const userFundAccount = await program.account.userFund.fetch(userFund);
    expect(userFundAccount.user).to.deep.equal(user.publicKey);
    expect(userFundAccount.totalFunded.toNumber()).to.equal(1 * unit);
    expect(userFundAccount.perLevel[0].toNumber()).to.equal(1 * unit);

    const vaultBalance = await provider.connection.getTokenAccountBalance(vault);
    expect(Number(vaultBalance.value.amount)).to.equal(1 * unit);
  });

  it("Fails to fund disabled level", async () => {
    try {
      await program.methods
        .fund(2, new anchor.BN(1 * unit))
        .accounts({
          user: user.publicKey,
          global,
          userFund,
          usdcMint,
          userUsdc,
          vault,
          authority: vaultAuthority,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([user])
        .rpc();
      expect.fail("Should have failed");
    } catch (err: any) {
      expect(err.error.errorCode.code).to.equal("LevelNotEnabled");
    }
  });

  it("Fails to fund below minimum amount", async () => {
    try {
      await program.methods
        .fund(1, new anchor.BN(1))
        .accounts({
          user: user.publicKey,
          global,
          userFund,
          usdcMint,
          userUsdc,
          vault,
          authority: vaultAuthority,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([user])
        .rpc();
      expect.fail("Should have failed");
    } catch (err: any) {
      expect(err.error.errorCode.code).to.equal("AmountTooSmall");
    }
  });

  it("Funds multiple times accumulating per level", async () => {
    const amount = 10 * unit;

    await program.methods
      .fund(1, new anchor.BN(amount))
      .accounts({
        user: user.publicKey,
        global,
        userFund,
        usdcMint,
        userUsdc,
        vault,
        authority: vaultAuthority,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([user])
      .rpc();

    const userFundAccount = await program.account.userFund.fetch(userFund);
    expect(userFundAccount.perLevel[0].toNumber()).to.equal((1 + 10) * unit);
    expect(userFundAccount.totalFunded.toNumber()).to.equal((1 + 10) * unit);
  });

  it("Fails to exceed max per account for level 1", async () => {
    try {
      // Level 1 max is 500 USDC, user already funded 11 USDC
      const amount = (500 - 11 + 1) * unit;
      await program.methods
        .fund(1, new anchor.BN(amount))
        .accounts({
          user: user.publicKey,
          global,
          userFund,
          usdcMint,
          userUsdc,
          vault,
          authority: vaultAuthority,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([user])
        .rpc();
      expect.fail("Should have failed");
    } catch (err: any) {
      expect(err.error.errorCode.code).to.equal("ExceedsMaxPerAccount");
    }
  });

  it("Enables level 2 and funds up to level total", async () => {
    // Enable level 2
    await program.methods
      .enableLevel(2, true)
      .accounts({
        admin: admin.publicKey,
        global,
      })
      .rpc();

    // Fund within the max per account for level 2 (999 USDC)
    const fundAmount = 999 * unit;

    await program.methods
      .fund(2, new anchor.BN(fundAmount))
      .accounts({
        user: user2.publicKey,
        global,
        userFund: user2Fund,
        usdcMint,
        userUsdc: user2Usdc,
        vault,
        authority: vaultAuthority,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([user2])
      .rpc();

    const globalAccount = await program.account.global.fetch(global);
    expect(globalAccount.totalPerLevel[1].toNumber()).to.equal(fundAmount);
  });

  it("Fails to exceed total level funding", async () => {
    try {
      // Enable level 5 (not used yet)
      await program.methods
        .enableLevel(5, true)
        .accounts({
          admin: admin.publicKey,
          global,
        })
        .rpc();

      // Level 5 (index 4) has unlimited per-account and level total of 45_800_000
      const levelTotal = 45_800_000 * unit;
      const exceedAmount = levelTotal + 1 * unit;

      await program.methods
        .fund(5, new anchor.BN(exceedAmount))
        .accounts({
          user: user3.publicKey,
          global,
          userFund: user3Fund,
          usdcMint,
          userUsdc: user3Usdc,
          vault,
          authority: vaultAuthority,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([user3])
        .rpc();
      expect.fail("Should have failed");
    } catch (err: any) {
      expect(err.error.errorCode.code).to.equal("ExceedsLevelTotal");
    }
  });

  it("Withdraws funds as admin", async () => {
    const withdrawAmount = 5 * unit;

    const vaultBalanceBefore = await provider.connection.getTokenAccountBalance(vault);
    const adminBalanceBefore = await provider.connection.getTokenAccountBalance(adminUsdc);

    await program.methods
      .withdraw(new anchor.BN(withdrawAmount))
      .accounts({
        admin: admin.publicKey,
        global,
        usdcMint,
        vault,
        adminUsdc,
        authority: vaultAuthority,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    const vaultBalanceAfter = await provider.connection.getTokenAccountBalance(vault);
    const adminBalanceAfter = await provider.connection.getTokenAccountBalance(adminUsdc);

    expect(Number(vaultBalanceAfter.value.amount)).to.equal(
      Number(vaultBalanceBefore.value.amount) - withdrawAmount
    );
    expect(Number(adminBalanceAfter.value.amount)).to.equal(
      Number(adminBalanceBefore.value.amount) + withdrawAmount
    );
  });

  it("Fails to withdraw as non-admin", async () => {
    try {
      await program.methods
        .withdraw(new anchor.BN(1 * unit))
        .accounts({
          admin: user.publicKey,
          global,
          usdcMint,
          vault,
          adminUsdc: userUsdc,
          authority: vaultAuthority,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([user])
        .rpc();
      expect.fail("Should have failed");
    } catch (err: any) {
      expect(err.error.errorCode.code).to.equal("Unauthorized");
    }
  });

  it("Fails to withdraw more than vault has", async () => {
    try {
      const vaultBalance = await provider.connection.getTokenAccountBalance(vault);
      const amount = Number(vaultBalance.value.amount) + 1;

      await program.methods
        .withdraw(new anchor.BN(amount))
        .accounts({
          admin: admin.publicKey,
          global,
          usdcMint,
          vault,
          adminUsdc,
          authority: vaultAuthority,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .rpc();
      expect.fail("Should have failed");
    } catch (err: any) {
      expect(err.error.errorCode.code).to.equal("InsufficientFunds");
    }
  });

  it("Allows withdraw of 0 amount", async () => {
    const vaultBalanceBefore = await provider.connection.getTokenAccountBalance(vault);

    await program.methods
      .withdraw(new anchor.BN(0))
      .accounts({
        admin: admin.publicKey,
        global,
        usdcMint,
        vault,
        adminUsdc,
        authority: vaultAuthority,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    const vaultBalanceAfter = await provider.connection.getTokenAccountBalance(vault);
    expect(Number(vaultBalanceAfter.value.amount)).to.equal(Number(vaultBalanceBefore.value.amount));
  });

  it("Disables level 1", async () => {
    await program.methods
      .enableLevel(1, false)
      .accounts({
        admin: admin.publicKey,
        global,
      })
      .rpc();

    const globalAccount = await program.account.global.fetch(global);
    expect(globalAccount.levelsEnabled[0]).to.be.false;
  });

  it("Funds different levels simultaneously", async () => {
    // Enable level 3
    await program.methods
      .enableLevel(3, true)
      .accounts({
        admin: admin.publicKey,
        global,
      })
      .rpc();

    // Enable level 4
    await program.methods
      .enableLevel(4, true)
      .accounts({
        admin: admin.publicKey,
        global,
      })
      .rpc();

    // Fund user 2 to level 3
    const level3Amount = 1000 * unit;
    await program.methods
      .fund(3, new anchor.BN(level3Amount))
      .accounts({
        user: user2.publicKey,
        global,
        userFund: user2Fund,
        usdcMint,
        userUsdc: user2Usdc,
        vault,
        authority: vaultAuthority,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([user2])
      .rpc();

    const globalAccount = await program.account.global.fetch(global);
    expect(globalAccount.totalPerLevel[2].toNumber()).to.equal(level3Amount);
    expect(globalAccount.totalPerLevel[3].toNumber()).to.equal(0);

    const user2FundAccount = await program.account.userFund.fetch(user2Fund);
    expect(user2FundAccount.perLevel[2].toNumber()).to.equal(level3Amount);
    expect(user2FundAccount.perLevel[3].toNumber()).to.equal(0);
  });
});
