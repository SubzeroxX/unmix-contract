use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{transfer, Mint, Token, TokenAccount, Transfer},
};

declare_id!("CbPxrLPpxo6Mp25XcDpbFwqkguFZetbuR6oWrNfscFSw");

const WITHDRAW_AUTHORITY: Pubkey = pubkey!("47Bxttko5pewcjuwd4oTzFeJmHY2yfs4j4YFXx2d68vV");

// ──────────────────────────────────────────────
// Constants
// ──────────────────────────────────────────────

// All amounts in instructions, storage, and events are in **atoms** (10⁻⁶ of a token).
// Level caps are defined in whole/UI tokens and converted to atoms using UNIT.
const DECIMALS: u8 = 6;
const UNIT: u64 = 1_000_000;

// Official Solana mainnet mint addresses
const USDC_MINT: Pubkey = pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
const USDT_MINT: Pubkey = pubkey!("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB");

// Level total caps (in whole tokens)
const LEVEL_TOTAL_WHOLE_TOKENS: [u64; 6] = [
    5_400_000,
    88_000_000,
    173_000_000,
    116_250_000,
    45_800_000,
    37_200_000,
];

// Per-account maximums (in whole tokens)
const LEVEL_MAX_PER_ACCOUNT_WHOLE_TOKENS: [u64; 6] = [
    500,
    999,
    5_000,
    u64::MAX,
    u64::MAX,
    u64::MAX,
];

#[program]
pub mod un_mix {
    use super::*;

    /// Initializes global state or registers one of the two supported mints (USDC or USDT).
    /// Must be called **twice**: once for USDC and once for USDT.
    /// The first call sets the admin authority.
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let mint_key = ctx.accounts.mint.key();
        let global = &mut ctx.accounts.global;

        // Use struct field access (Anchor 0.30+ style)
        global.bump = ctx.bumps.authority;

        // Set admin only on first initialization
        if global.admin == Pubkey::default() {
            global.admin = *ctx.accounts.admin.key;
        } else {
            require_keys_eq!(
                *ctx.accounts.admin.key,
                global.admin,
                ErrorCode::Unauthorized
            );
        }

        // Register mint — allow only USDC or USDT, exactly once each
        if mint_key == USDC_MINT {
            require_eq!(
                global.usdc_mint,
                Pubkey::default(),
                ErrorCode::MintAlreadySet
            );
            global.usdc_mint = mint_key;
        } else if mint_key == USDT_MINT {
            require_eq!(
                global.usdt_mint,
                Pubkey::default(),
                ErrorCode::MintAlreadySet
            );
            global.usdt_mint = mint_key;
        } else {
            return err!(ErrorCode::InvalidMint);
        }

        emit!(InitializedEvent {
            admin: global.admin,
            mint: mint_key,
            vault: ctx.accounts.vault.key(),
        });

        Ok(())
    }

    pub fn enable_level(ctx: Context<EnableLevel>, level: u8, enabled: bool) -> Result<()> {
        require_keys_eq!(
            *ctx.accounts.admin.key,
            ctx.accounts.global.admin,
            ErrorCode::Unauthorized
        );
        require!(level >= 1 && level <= 6, ErrorCode::InvalidLevel);

        let idx = (level as usize) - 1;
        ctx.accounts.global.levels_enabled[idx] = enabled;

        emit!(LevelUpdatedEvent {
            admin: *ctx.accounts.admin.key,
            level,
            enabled,
        });

        Ok(())
    }

    pub fn fund(ctx: Context<Fund>, level: u8, amount: u64) -> Result<()> {
        require!(amount > 0, ErrorCode::AmountTooSmall);
        require!(level >= 1 && level <= 6, ErrorCode::InvalidLevel);

        let idx = (level as usize) - 1;
        let global = &mut ctx.accounts.global;
        require!(global.levels_enabled[idx], ErrorCode::LevelNotEnabled);

        let mint_key = ctx.accounts.mint.key();
        let is_usdc = mint_key == global.usdc_mint;
        let is_usdt = mint_key == global.usdt_mint;
        require!(is_usdc || is_usdt, ErrorCode::InvalidMint);

        require_eq!(
            ctx.accounts.mint.decimals,
            DECIMALS,
            ErrorCode::InvalidDecimals
        );

        let max_per_account_atoms = if LEVEL_MAX_PER_ACCOUNT_WHOLE_TOKENS[idx] == u64::MAX {
            u64::MAX
        } else {
            LEVEL_MAX_PER_ACCOUNT_WHOLE_TOKENS[idx]
                .checked_mul(UNIT)
                .ok_or(ErrorCode::ArithmeticError)?
        };

        let user_fund = &mut ctx.accounts.user_fund;
        user_fund.user = *ctx.accounts.user.key;

        let current_atoms = if is_usdc {
            user_fund.per_level_usdc[idx]
        } else {
            user_fund.per_level_usdt[idx]
        };

        let new_atoms = current_atoms
            .checked_add(amount)
            .ok_or(ErrorCode::ArithmeticError)?;

        require!(
            new_atoms <= max_per_account_atoms,
            ErrorCode::ExceedsMaxPerAccount
        );

        user_fund.total_funded = user_fund
            .total_funded
            .checked_add(amount)
            .ok_or(ErrorCode::ArithmeticError)?;

        if is_usdc {
            user_fund.per_level_usdc[idx] = new_atoms;
        } else {
            user_fund.per_level_usdt[idx] = new_atoms;
        }

        let max_level_atoms = LEVEL_TOTAL_WHOLE_TOKENS[idx]
            .checked_mul(UNIT)
            .ok_or(ErrorCode::ArithmeticError)?;

        let new_total = global.total_per_level[idx]
            .checked_add(amount)
            .ok_or(ErrorCode::ArithmeticError)?;

        require!(new_total <= max_level_atoms, ErrorCode::ExceedsLevelTotal);

        transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.user_token_account.to_account_info(),
                    to: ctx.accounts.vault.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            amount,
        )?;

        if is_usdc {
            global.usdc_vault_total = global
                .usdc_vault_total
                .checked_add(amount)
                .ok_or(ErrorCode::ArithmeticError)?;
        } else {
            global.usdt_vault_total = global
                .usdt_vault_total
                .checked_add(amount)
                .ok_or(ErrorCode::ArithmeticError)?;
        }

        global.total_per_level[idx] = new_total;
        global.total_fund = global
            .total_fund
            .checked_add(amount)
            .ok_or(ErrorCode::ArithmeticError)?;

        if new_total >= max_level_atoms {
            global.levels_enabled[idx] = false;
        }

        emit!(FundedEvent {
            user: *ctx.accounts.user.key,
            level,
            amount,
            mint: mint_key,
        });

        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        require!(amount > 0, ErrorCode::AmountTooSmall);

        require_keys_eq!(
            *ctx.accounts.withdrawer.key,
            WITHDRAW_AUTHORITY,
            ErrorCode::Unauthorized
        );

        let global = &ctx.accounts.global;
        let mint_key = ctx.accounts.mint.key();
        let is_usdc = mint_key == global.usdc_mint;
        let is_usdt = mint_key == global.usdt_mint;
        require!(is_usdc || is_usdt, ErrorCode::InvalidMint);

        require_gte!(
            ctx.accounts.vault.amount,
            amount,
            ErrorCode::InsufficientFunds
        );

        let seeds = &[b"authority".as_ref(), &[global.bump]];
        let signer_seeds = &[&seeds[..]];

        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.vault.to_account_info(),
                    to: ctx.accounts.withdrawer_token_account.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info(),
                },
                signer_seeds,
            ),
            amount,
        )?;

        emit!(WithdrawnEvent {
            withdrawer: *ctx.accounts.withdrawer.key,
            amount,
            mint: mint_key,
        });

        Ok(())
    }
}

// ──────────────────────────────────────────────
// Accounts
// ──────────────────────────────────────────────

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        constraint = mint.key() == USDC_MINT || mint.key() == USDT_MINT @ ErrorCode::InvalidMint,
        constraint = mint.decimals == DECIMALS @ ErrorCode::InvalidDecimals
    )]
    pub mint: Account<'info, Mint>,

    #[account(
        init_if_needed,
        payer = admin,
        space = 8 + Global::INIT_SPACE,
        seeds = [b"global"],
        bump,
    )]
    pub global: Account<'info, Global>,

    /// CHECK: PDA — vault authority
    #[account(
        seeds = [b"authority"],
        bump
    )]
    pub authority: UncheckedAccount<'info>,

    #[account(
        init_if_needed,
        payer = admin,
        associated_token::mint = mint,
        associated_token::authority = authority
    )]
    pub vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct EnableLevel<'info> {
    pub admin: Signer<'info>,

    #[account(mut, seeds = [b"global"], bump)]
    pub global: Account<'info, Global>,
}

#[derive(Accounts)]
pub struct Fund<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mut, seeds = [b"global"], bump)]
    pub global: Account<'info, Global>,

    #[account(
        init_if_needed,
        payer = user,
        space = 8 + UserFund::INIT_SPACE,
        seeds = [b"user_fund", user.key().as_ref()],
        bump,
    )]
    pub user_fund: Account<'info, UserFund>,

    #[account(
        constraint = mint.decimals == DECIMALS @ ErrorCode::InvalidDecimals
    )]
    pub mint: Account<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = user,
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = authority,
        constraint = vault.mint == mint.key() @ ErrorCode::MintMismatch,
        constraint = vault.owner == authority.key() @ ErrorCode::InvalidVault
    )]
    pub vault: Account<'info, TokenAccount>,

    /// CHECK: PDA
    #[account(
        seeds = [b"authority"],
        bump = global.bump
    )]
    pub authority: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    pub withdrawer: Signer<'info>,

    #[account(seeds = [b"global"], bump)]
    pub global: Account<'info, Global>,

    pub mint: Account<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = authority,
    )]
    pub vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = withdrawer,
    )]
    pub withdrawer_token_account: Account<'info, TokenAccount>,

    /// CHECK: PDA
    #[account(
        seeds = [b"authority"],
        bump = global.bump
    )]
    pub authority: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

// ──────────────────────────────────────────────
// State
// ──────────────────────────────────────────────

#[account]
#[derive(Default)]
pub struct Global {
    pub admin: Pubkey,
    pub usdc_mint: Pubkey,
    pub usdt_mint: Pubkey,
    pub total_fund: u64,
    pub usdc_vault_total: u64,
    pub usdt_vault_total: u64,
    pub levels_enabled: [bool; 6],
    pub total_per_level: [u64; 6],
    pub bump: u8,
}

impl Global {
    pub const INIT_SPACE: usize = 32 * 3          // admin + usdc_mint + usdt_mint
        + 8 * 3                               // total_fund + usdc_vault_total + usdt_vault_total
        + 1 * 6                               // levels_enabled
        + 8 * 6                               // total_per_level
        + 1;                                  // bump
}

#[account]
#[derive(Default)]
pub struct UserFund {
    pub user: Pubkey,
    pub total_funded: u64,
    pub per_level_usdc: [u64; 6],
    pub per_level_usdt: [u64; 6],
}

impl UserFund {
    pub const INIT_SPACE: usize = 32              // user
        + 8                                       // total_funded
        + 8 * 6 * 2;                              // per_level_usdc + per_level_usdt
}

// ──────────────────────────────────────────────
// Events
// ──────────────────────────────────────────────

#[event]
pub struct InitializedEvent {
    pub admin: Pubkey,
    pub mint: Pubkey,
    pub vault: Pubkey,
}

#[event]
pub struct FundedEvent {
    pub user: Pubkey,
    pub level: u8,
    pub amount: u64,
    pub mint: Pubkey,
}

#[event]
pub struct WithdrawnEvent {
    pub withdrawer: Pubkey,
    pub amount: u64,
    pub mint: Pubkey,
}

#[event]
pub struct LevelUpdatedEvent {
    pub admin: Pubkey,
    pub level: u8,
    pub enabled: bool,
}

// ──────────────────────────────────────────────
// Errors
// ──────────────────────────────────────────────

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid level (must be 1-6)")]
    InvalidLevel,
    #[msg("Insufficient funds in vault")]
    InsufficientFunds,
    #[msg("This level is not enabled")]
    LevelNotEnabled,
    #[msg("Amount must be greater than zero")]
    AmountTooSmall,
    #[msg("Exceeds maximum allowed per account for this level")]
    ExceedsMaxPerAccount,
    #[msg("Exceeds total allowed amount for this level")]
    ExceedsLevelTotal,
    #[msg("Unsupported mint (only USDC or USDT allowed)")]
    InvalidMint,
    #[msg("Unauthorized caller")]
    Unauthorized,
    #[msg("Arithmetic overflow/underflow")]
    ArithmeticError,
    #[msg("Token must have exactly 6 decimals")]
    InvalidDecimals,
    #[msg("Invalid vault account")]
    InvalidVault,
    #[msg("Mint mismatch with vault")]
    MintMismatch,
    #[msg("This mint has already been initialized")]
    MintAlreadySet,
}