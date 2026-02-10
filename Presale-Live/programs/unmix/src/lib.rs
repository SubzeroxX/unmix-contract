use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{transfer, Mint, Token, TokenAccount, Transfer}; // ← add this back

declare_id!("AkxithguMWHSuaufEQEyRgs91kpA2vaaWSWAU91nfJvu");

const WITHDRAW_AUTHORITY: Pubkey = pubkey!("BTfs792s9GzLTowWdtYiLskn5VsWZah1mVUuAUiwmLZx");
// main: 47Bxttko5pewcjuwd4oTzFeJmHY2yfs4j4YFXx2d68vV
// test: BTfs792s9GzLTowWdtYiLskn5VsWZah1mVUuAUiwmLZx
const USDC_MINT: Pubkey = pubkey!("4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU");
// main: EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v
// dev:4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU
const USDT_MINT: Pubkey = pubkey!("HzwqbKZw8HxMN6bF2yFZNrht3c2iXXzpKcFu7uBEDKtr");
// main: Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB
// dev (eurc): HzwqbKZw8HxMN6bF2yFZNrht3c2iXXzpKcFu7uBEDKtr

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum TokenType {
    USDC,
    USDT,
}

#[program]
pub mod un_mix {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        require_keys_eq!(
            ctx.accounts.usdc_mint.key(),
            USDC_MINT,
            ErrorCode::InvalidMint
        );
        require_keys_eq!(
            ctx.accounts.usdt_mint.key(),
            USDT_MINT,
            ErrorCode::InvalidMint
        );

        let authority_key = ctx.accounts.authority.key();

        require_keys_eq!(
            ctx.accounts.usdc_vault.key(),
            anchor_spl::associated_token::get_associated_token_address(
                &authority_key,
                &ctx.accounts.usdc_mint.key(),
            )
        );

        require_keys_eq!(
            ctx.accounts.usdt_vault.key(),
            anchor_spl::associated_token::get_associated_token_address(
                &authority_key,
                &ctx.accounts.usdt_mint.key(),
            )
        );

        let global = &mut ctx.accounts.global;
        global.admin = ctx.accounts.admin.key();
        global.usdc_mint = ctx.accounts.usdc_mint.key();
        global.usdt_mint = ctx.accounts.usdt_mint.key();
        global.total_fund_usd = 0;
        global.total_fund_usdc = 0;
        global.total_fund_usdt = 0;
        global.levels_enabled = [false; 6];
        global.total_per_level = [0; 6];
        global.total_per_level_usdc = [0; 6];
        global.total_per_level_usdt = [0; 6];

        Ok(())
    }

    pub fn enable_level(ctx: Context<EnableLevel>, level: u8, enabled: bool) -> Result<()> {
        require_keys_eq!(
            ctx.accounts.admin.key(),
            ctx.accounts.global.admin,
            ErrorCode::Unauthorized
        );
        require!(level >= 1 && level <= 6, ErrorCode::InvalidLevel);
        let l = level as usize - 1;

        if enabled {
            let unit = 10u64.pow(ctx.accounts.usdc_mint.decimals as u32);
            let max_total = LEVEL_TOTAL_FUNDS[l]
                .checked_mul(unit)
                .ok_or(ErrorCode::ArithmeticError)?;
            require!(
                ctx.accounts.global.total_per_level[l] < max_total,
                ErrorCode::ExceedsLevelTotal
            );
        }

        let global = &mut ctx.accounts.global;
        global.levels_enabled[l] = enabled;

        emit!(LevelUpdatedEvent {
            admin: ctx.accounts.admin.key(),
            level,
            enabled,
        });

        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>, token_type: TokenType, amount: u64) -> Result<()> {
        require!(amount > 0, ErrorCode::AmountTooSmall);
        require_keys_eq!(
            ctx.accounts.withdrawer.key(),
            WITHDRAW_AUTHORITY,
            ErrorCode::Unauthorized
        );

        let (vault, withdrawer_token, mint) = match token_type {
            TokenType::USDC => (
                &ctx.accounts.usdc_vault,
                &ctx.accounts.withdrawer_token,
                &ctx.accounts.usdc_mint,
            ),
            TokenType::USDT => (
                &ctx.accounts.usdt_vault,
                &ctx.accounts.withdrawer_token,
                &ctx.accounts.usdt_mint,
            ),
        };

        require_keys_eq!(
            mint.key(),
            match token_type {
                TokenType::USDC => ctx.accounts.global.usdc_mint,
                TokenType::USDT => ctx.accounts.global.usdt_mint,
            },
            ErrorCode::MintMismatch
        );

        let vault_amount = vault.amount;
        require_gte!(vault_amount, amount, ErrorCode::InsufficientFunds);

        // Fixed seeds syntax
        let authority_seeds: &[&[u8]] = &[
            b"authority",           // &[u8; 9] coerces to &[u8]
            &[ctx.bumps.authority], // &[u8; 1] → &[u8]
        ];

        let signer_seeds: &[&[&[u8]]] = &[authority_seeds];

        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: vault.to_account_info(),
                    to: withdrawer_token.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info(),
                },
                signer_seeds,
            ),
            amount,
        )?;

        emit!(WithdrawnEvent {
            withdrawer: ctx.accounts.withdrawer.key(),
            token_type,
            amount,
        });

        Ok(())
    }

    // fund function remains unchanged — it was already correct in the previous version
    pub fn fund(ctx: Context<Fund>, token_type: TokenType, level: u8, amount: u64) -> Result<()> {
        require!(level >= 1 && level <= 6, ErrorCode::InvalidLevel);
        let l = level as usize - 1;

        let global = &mut ctx.accounts.global;
        require!(global.levels_enabled[l], ErrorCode::LevelNotEnabled);

        let (mint, user_token, vault) = match token_type {
            TokenType::USDC => (
                &ctx.accounts.usdc_mint,
                &ctx.accounts.user_token,
                &ctx.accounts.usdc_vault,
            ),
            TokenType::USDT => (
                &ctx.accounts.usdt_mint,
                &ctx.accounts.user_token,
                &ctx.accounts.usdt_vault,
            ),
        };

        require_keys_eq!(
            mint.key(),
            match token_type {
                TokenType::USDC => global.usdc_mint,
                TokenType::USDT => global.usdt_mint,
            },
            ErrorCode::MintMismatch
        );

        let decimals = mint.decimals;
        require!(decimals <= 18, ErrorCode::InvalidDecimals);
        let unit = 10u64.pow(decimals as u32);
        let min_amount = unit;
        require_gte!(amount, min_amount, ErrorCode::AmountTooSmall);

        let max_per_account_raw = if LEVEL_MAX_PER_ACCOUNT[l] == u64::MAX {
            u64::MAX
        } else {
            LEVEL_MAX_PER_ACCOUNT[l]
                .checked_mul(unit)
                .ok_or(ErrorCode::ArithmeticError)?
        };

        let max_total_raw = LEVEL_TOTAL_FUNDS[l]
            .checked_mul(unit)
            .ok_or(ErrorCode::ArithmeticError)?;

        let user_fund = &mut ctx.accounts.user_fund;
        user_fund.user = ctx.accounts.user.key();

        // READ PHASE
        let usdc_before = user_fund.usdc_per_level[l];
        let usdt_before = user_fund.usdt_per_level[l];
        let user_total_level_before = usdc_before
            .checked_add(usdt_before)
            .ok_or(ErrorCode::ArithmeticError)?;

        let global_usdc_this_level = global.total_per_level_usdc[l];
        let global_usdt_this_level = global.total_per_level_usdt[l];
        let global_total_level_before = global_usdc_this_level
            .checked_add(global_usdt_this_level)
            .ok_or(ErrorCode::ArithmeticError)?;

        // CHECK PHASE
        require!(
            user_total_level_before
                .checked_add(amount)
                .ok_or(ErrorCode::ArithmeticError)?
                <= max_per_account_raw,
            ErrorCode::ExceedsMaxPerAccount
        );

        require!(
            global_total_level_before
                .checked_add(amount)
                .ok_or(ErrorCode::ArithmeticError)?
                <= max_total_raw,
            ErrorCode::ExceedsLevelTotal
        );

        // TRANSFER
        transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: user_token.to_account_info(),
                    to: vault.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            amount,
        )?;

        // MUTATE PHASE
        match token_type {
            TokenType::USDC => {
                user_fund.usdc_per_level[l] = usdc_before
                    .checked_add(amount)
                    .ok_or(ErrorCode::ArithmeticError)?;
                global.total_per_level_usdc[l] = global_usdc_this_level
                    .checked_add(amount)
                    .ok_or(ErrorCode::ArithmeticError)?;
                global.total_fund_usdc = global
                    .total_fund_usdc
                    .checked_add(amount)
                    .ok_or(ErrorCode::ArithmeticError)?;
            }
            TokenType::USDT => {
                user_fund.usdt_per_level[l] = usdt_before
                    .checked_add(amount)
                    .ok_or(ErrorCode::ArithmeticError)?;
                global.total_per_level_usdt[l] = global_usdt_this_level
                    .checked_add(amount)
                    .ok_or(ErrorCode::ArithmeticError)?;
                global.total_fund_usdt = global
                    .total_fund_usdt
                    .checked_add(amount)
                    .ok_or(ErrorCode::ArithmeticError)?;
            }
        }

        global.total_fund_usd = global
            .total_fund_usd
            .checked_add(amount)
            .ok_or(ErrorCode::ArithmeticError)?;
        global.total_per_level[l] = global.total_per_level[l]
            .checked_add(amount)
            .ok_or(ErrorCode::ArithmeticError)?;

        user_fund.total_funded = user_fund
            .total_funded
            .checked_add(amount)
            .ok_or(ErrorCode::ArithmeticError)?;
        user_fund.per_level[l] = user_total_level_before
            .checked_add(amount)
            .ok_or(ErrorCode::ArithmeticError)?;

        if global_total_level_before + amount == max_total_raw {
            global.levels_enabled[l] = false;
        }

        emit!(FundedEvent {
            user: ctx.accounts.user.key(),
            token_type,
            level,
            amount,
        });

        Ok(())
    }
}

// Initialize (authority PDA stays low-level but documented)
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    pub usdc_mint: Account<'info, Mint>,
    pub usdt_mint: Account<'info, Mint>,
    #[account(
        init,
        payer = admin,
        space = Global::LEN,
        seeds = [b"global"],
        bump,
    )]
    pub global: Account<'info, Global>,

    // PDA used only for signing CPI transfers — Anchor verifies derivation
    /// CHECK: this is a program-derived address used **only** as authority/signer in CPI token transfers;
    /// no data is read/written, seeds + bump are enforced by constraint
    #[account(seeds = [b"authority"], bump)]
    pub authority: AccountInfo<'info>,  // ← changed from UncheckedAccount

    #[account(
        init_if_needed,
        payer = admin,
        associated_token::mint = usdc_mint,
        associated_token::authority = authority,
    )]
    pub usdc_vault: Account<'info, TokenAccount>,
    #[account(
        init_if_needed,
        payer = admin,
        associated_token::mint = usdt_mint,
        associated_token::authority = authority,
    )]
    pub usdt_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct EnableLevel<'info> {
    pub admin: Signer<'info>,
    #[account(
        mut,
        seeds = [b"global"],
        bump,
        constraint = global.usdc_mint == usdc_mint.key() @ ErrorCode::MintMismatch,
        constraint = global.usdt_mint == usdt_mint.key() @ ErrorCode::MintMismatch
    )]
    pub global: Account<'info, Global>,
    pub usdc_mint: Account<'info, Mint>,
    pub usdt_mint: Account<'info, Mint>,
}

// ──────────────────────────────────────────────
// Withdraw — make withdrawer_token checked
#[derive(Accounts)]
pub struct Withdraw<'info> {
    pub withdrawer: Signer<'info>,
    #[account(seeds = [b"global"], bump)]
    pub global: Account<'info, Global>,
    pub usdc_mint: Account<'info, Mint>,
    pub usdt_mint: Account<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = usdc_mint,
        associated_token::authority = authority,
    )]
    pub usdc_vault: Account<'info, TokenAccount>,
    #[account(
        mut,
        associated_token::mint = usdt_mint,
        associated_token::authority = authority,
    )]
    pub usdt_vault: Account<'info, TokenAccount>,

    // Safer: let Anchor check it's a token account owned by Token program
    #[account(
        mut,
        // optional: you can add more if you want tighter checks
        // token::mint = usdc_mint / usdt_mint  ← but we select dynamically so can't here
    )]
    pub withdrawer_token: Account<'info, TokenAccount>,  // ← changed!

    // PDA — same justification as above
    /// CHECK: PDA used only as CPI signer; derivation enforced by seeds + bump
    #[account(seeds = [b"authority"], bump)]
    pub authority: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}

// ──────────────────────────────────────────────
// Fund — same pattern for user_token
#[derive(Accounts)]
pub struct Fund<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut, seeds = [b"global"], bump)]
    pub global: Account<'info, Global>,
    #[account(
        init_if_needed,
        payer = user,
        space = UserFund::LEN,
        seeds = [b"user_fund", user.key().as_ref()],
        bump,
    )]
    pub user_fund: Account<'info, UserFund>,
    pub usdc_mint: Account<'info, Mint>,
    pub usdt_mint: Account<'info, Mint>,

    // Safer: checked token account (Anchor verifies owner + data layout)
    #[account(mut)]
    pub user_token: Account<'info, TokenAccount>,  // ← changed!

    #[account(
        mut,
        associated_token::mint = usdc_mint,
        associated_token::authority = authority,
    )]
    pub usdc_vault: Account<'info, TokenAccount>,
    #[account(
        mut,
        associated_token::mint = usdt_mint,
        associated_token::authority = authority,
    )]
    pub usdt_vault: Account<'info, TokenAccount>,

    /// CHECK: PDA referenced in vault constraints + used for signing in withdraw;
    /// derivation enforced, no data access
    #[account(seeds = [b"authority"], bump)]
    pub authority: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[account]
#[derive(Default)]
pub struct Global {
    pub admin: Pubkey,
    pub usdc_mint: Pubkey,
    pub usdt_mint: Pubkey,
    pub total_fund_usd: u64,
    pub total_fund_usdc: u64,
    pub total_fund_usdt: u64,
    pub levels_enabled: [bool; 6],
    pub total_per_level: [u64; 6], // legacy / approximate sum
    pub total_per_level_usdc: [u64; 6],
    pub total_per_level_usdt: [u64; 6],
}

impl Global {
    pub const LEN: usize = 8 + 32 * 3 + 8 * 3 + 6 * 1 + 6 * 8 * 3;
}

#[account]
#[derive(Default)]
pub struct UserFund {
    pub user: Pubkey,
    pub total_funded: u64,
    pub per_level: [u64; 6], // legacy / approximate sum
    pub usdc_per_level: [u64; 6],
    pub usdt_per_level: [u64; 6],
}

impl UserFund {
    pub const LEN: usize = 8 + 32 + 8 + 6 * 8 * 3;
}

const LEVEL_TOTAL_FUNDS: [u64; 6] = [
    5_400_000,
    88_000_000,
    173_000_000,
    116_250_000,
    45_800_000,
    37_200_000,
];

const LEVEL_MAX_PER_ACCOUNT: [u64; 6] = [500, 999, 5000, u64::MAX, u64::MAX, u64::MAX];

#[event]
pub struct FundedEvent {
    pub user: Pubkey,
    pub token_type: TokenType,
    pub level: u8,
    pub amount: u64,
}

#[event]
pub struct WithdrawnEvent {
    pub withdrawer: Pubkey,
    pub token_type: TokenType,
    pub amount: u64,
}

#[event]
pub struct LevelUpdatedEvent {
    pub admin: Pubkey,
    pub level: u8,
    pub enabled: bool,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid level.")]
    InvalidLevel,
    #[msg("Insufficient funds")]
    InsufficientFunds,
    #[msg("Level not enabled.")]
    LevelNotEnabled,
    #[msg("Amount too small.")]
    AmountTooSmall,
    #[msg("Exceeds max per account for level.")]
    ExceedsMaxPerAccount,
    #[msg("Exceeds total for level.")]
    ExceedsLevelTotal,
    #[msg("Mint mismatch.")]
    MintMismatch,
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Arithmetic error (overflow/underflow)")]
    ArithmeticError,
    #[msg("Invalid mint decimals (must be <= 18)")]
    InvalidDecimals,
    #[msg("Invalid account")]
    InvalidAccount,
    #[msg("Invalid mint")]
    InvalidMint,
}
