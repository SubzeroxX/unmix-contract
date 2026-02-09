// test-helpers.ts
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { UnMix } from "../target/types/un_mix";
import {
  PublicKey,
  Keypair,
  LAMPORTS_PER_SOL,
  Connection,
  SystemProgram,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createMint,
  getOrCreateAssociatedTokenAccount,
  mintTo,
  getAccount,
  Account as TokenAccount,
} from "@solana/spl-token";

let _sharedAdmin: Keypair | undefined;

export function getSharedAdmin(): Keypair {
  if (!_sharedAdmin) {
    _sharedAdmin = Keypair.generate();
  }
  return _sharedAdmin;
}

export const DECIMALS = 6;
export const UNIT = 1_000_000n; // use BigInt to avoid precision issues

export const LEVEL_TOTAL_WHOLE_TOKENS = [
  5_400_000n,
  88_000_000n,
  173_000_000n,
  116_250_000n,
  45_800_000n,
  37_200_000n,
] as const;

export const LEVEL_MAX_PER_ACCOUNT_WHOLE_TOKENS = [
  500n,
  999n,
  5_000n,
  BigInt(Number.MAX_SAFE_INTEGER),
  BigInt(Number.MAX_SAFE_INTEGER),
  BigInt(Number.MAX_SAFE_INTEGER),
] as const;

/**
 * One-time program + mints + vaults + levels setup
 * Call this in before() hook of each test file
 */
let _initialized = false;
let _cachedSetup: ReturnType<typeof setupProgramAndMintsInternal> | undefined;

async function setupProgramAndMintsInternal(
  program: Program<UnMix>,
  provider: anchor.AnchorProvider,
): Promise<{
  admin: Keypair;
  usdcMint: PublicKey;
  usdtMint: PublicKey;
  usdcVault: PublicKey;
  usdtVault: PublicKey;
  globalPDA: PublicKey;
  authorityPDA: PublicKey;
}> {
  const admin = getSharedAdmin();
  await airdrop(provider.connection, admin.publicKey, 50);

  const usdcMint = await createMint(
    provider.connection,
    admin,
    admin.publicKey,
    null,
    DECIMALS,
  );

  const usdtMint = await createMint(
    provider.connection,
    admin,
    admin.publicKey,
    null,
    DECIMALS,
  );

  const { global: globalPDA, authority: authorityPDA } = getPDAs(program.programId);

  // Check if already initialized
  let globalAccount;
  try {
    globalAccount = await program.account.global.fetch(globalPDA);
    _initialized = true;
    console.log("Global already initialized — skipping re-init");
  } catch (err) {
    // Not initialized yet
  }

  if (!_initialized) {
    // USDC
    const usdcVaultAcc = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      admin,
      usdcMint,
      authorityPDA,
      true,
    );
    const usdcVault = usdcVaultAcc.address;

    await program.methods
      .initialize()
      .accounts({
        admin: admin.publicKey,
        mint: usdcMint,
        global: globalPDA,
        authority: authorityPDA,
        vault: usdcVault,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([admin])
      .rpc();

    // USDT
    const usdtVaultAcc = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      admin,
      usdtMint,
      authorityPDA,
      true,
    );
    const usdtVault = usdtVaultAcc.address;

    await program.methods
      .initialize()
      .accounts({
        admin: admin.publicKey,
        mint: usdtMint,
        global: globalPDA,
        authority: authorityPDA,
        vault: usdtVault,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([admin])
      .rpc();

    // Enable levels
    for (let i = 1; i <= 6; i++) {
      await program.methods
        .enableLevel(i, true)
        .accounts({ admin: admin.publicKey, global: globalPDA })
        .signers([admin])
        .rpc();
    }

    _initialized = true;
  }

  // Fetch current state (even if already init)
  const globalAccountNow = await program.account.global.fetch(globalPDA);

  return {
    admin,
    usdcMint: globalAccountNow.usdcMint,
    usdtMint: globalAccountNow.usdtMint,
    usdcVault: (await getOrCreateAssociatedTokenAccount(
      provider.connection,
      admin,
      globalAccountNow.usdcMint,
      authorityPDA,
      true,
    )).address,
    usdtVault: (await getOrCreateAssociatedTokenAccount(
      provider.connection,
      admin,
      globalAccountNow.usdtMint,
      authorityPDA,
      true,
    )).address,
    globalPDA,
    authorityPDA,
  };
}

export async function setupProgramAndMints(
  program: Program<UnMix>,
  provider: anchor.AnchorProvider,
) {
  if (_cachedSetup) {
    console.log("Reusing cached program setup");
    return _cachedSetup;
  }

  _cachedSetup = await setupProgramAndMintsInternal(program, provider);
  return _cachedSetup;
}

/**
 * Simple airdrop helper
 */
export async function airdrop(
  connection: Connection,
  to: PublicKey,
  amountSol = 10,
): Promise<void> {
  const tx = await connection.requestAirdrop(to, amountSol * LAMPORTS_PER_SOL);
  await connection.confirmTransaction(tx);
}

/**
 * Quick way to give a user tokens for testing
 */
export async function fundUserTokenAccount(
  connection: Connection,
  payer: Keypair,
  mint: PublicKey,
  user: PublicKey,
  amountAtoms: bigint,
): Promise<PublicKey> {
  const ata = await getOrCreateAssociatedTokenAccount(
    connection,
    payer,
    mint,
    user,
  );

  await mintTo(
    connection,
    payer,
    mint,
    ata.address,
    payer,
    amountAtoms,
  );

  return ata.address;
}

/**
 * Verify vault balance matches program's accounting
 */
export async function assertVaultBalanceMatches(
  connection: Connection,
  vault: PublicKey,
  expected: bigint,
  label = "Vault",
): Promise<void> {
  const acc = await getAccount(connection, vault);
  const actual = BigInt(acc.amount.toString());
  if (actual !== expected) {
    throw new Error(
      `${label} balance mismatch: expected ${expected}, got ${actual}`,
    );
  }
}

/**
 * Check important global <-> vault <-> level totals invariants
 */
export async function assertAccountingIsConsistent(
  program: Program<UnMix>,
  connection: Connection,
  globalPDA: PublicKey,
  usdcVault: PublicKey,
  usdtVault: PublicKey,
): Promise<void> {
  let global;
  try {
    global = await program.account.global.fetch(globalPDA);
  } catch (err) {
    console.warn("Global account not found or failed to fetch:", err);
    return; // skip check if global doesn't exist yet
  }

  let sumLevels = 0n;
  for (const amt of global.totalPerLevel) {
    sumLevels += BigInt(amt.toString());
  }

  const sumVaults =
    BigInt(global.usdcVaultTotal.toString()) +
    BigInt(global.usdtVaultTotal.toString());

  if (sumLevels !== BigInt(global.totalFund.toString())) {
    throw new Error(
      `Level totals sum (${sumLevels}) != totalFund (${global.totalFund})`,
    );
  }

  if (sumVaults !== BigInt(global.totalFund.toString())) {
    throw new Error(
      `Vault totals sum (${sumVaults}) != totalFund (${global.totalFund})`,
    );
  }

  const usdcAcc = await getAccount(connection, usdcVault);
  const usdtAcc = await getAccount(connection, usdtVault);

  if (BigInt(usdcAcc.amount.toString()) !== BigInt(global.usdcVaultTotal.toString())) {
    throw new Error(`USDC vault mismatch`);
  }

  if (BigInt(usdtAcc.amount.toString()) !== BigInt(global.usdtVaultTotal.toString())) {
    throw new Error(`USDT vault mismatch`);
  }
}

/**
 * Derive user fund PDA
 */
export function getUserFundPDA(
  programId: PublicKey,
  user: PublicKey,
): PublicKey {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("user_fund"), user.toBuffer()],
    programId,
  )[0];
}

export function getPDAs(programId: PublicKey) {
  const [global] = PublicKey.findProgramAddressSync([Buffer.from("global")], programId);
  const [authority] = PublicKey.findProgramAddressSync([Buffer.from("authority")], programId);
  return { global, authority };
}