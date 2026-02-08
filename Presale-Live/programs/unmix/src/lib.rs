use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{transfer, Mint, Token, TokenAccount, Transfer};

declare_id!("AjpFJjuwbYwPjxtChUxdgVa8mmrSQ2zakkk1EmZJ4YkL");

const WITHDRAW_AUTHORITY: Pubkey = pubkey!("47Bxttko5pewcjuwd4oTzFeJmHY2yfs4j4YFXx2d68vV"); // Replace with the actual hardcoded address for exclusive withdraw authority
const USDC_MINT: Pubkey = pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
// devnet: 4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU
// mainnet: EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v
#[program]
pub mod un_mix {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        require_keys_eq!(
            ctx.accounts.usdc_mint.key(),
            USDC_MINT,
            ErrorCode::InvalidMint
        );

        let authority_key = ctx.accounts.authority.key();
        
        // Verify vault address
        require_keys_eq!(
            ctx.accounts.vault.key(),
            anchor_spl::associated_token::get_associated_token_address(
                &authority_key,
                &ctx.accounts.usdc_mint.key()
            )
        );

        let global = &mut ctx.accounts.global;
        global.admin = ctx.accounts.admin.key();
        global.usdc_mint = ctx.accounts.usdc_mint.key();
        global.total_fund = 0;
        global.levels_enabled = [false; 6];
        global.total_per_level = [0; 6];
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
        global.levels_enabled[level as usize - 1] = enabled;

        emit!(LevelUpdatedEvent {
            admin: ctx.accounts.admin.key(),
            level,
            enabled,
        });
        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        require!(amount > 0, ErrorCode::AmountTooSmall);
        require_keys_eq!(
            ctx.accounts.usdc_mint.key(),
            ctx.accounts.global.usdc_mint,
            ErrorCode::MintMismatch
        );
        require_keys_eq!(
            ctx.accounts.withdrawer.key(),
            WITHDRAW_AUTHORITY,
            ErrorCode::Unauthorized
        );

        let vault_amount = ctx.accounts.vault.amount;
        require_gte!(vault_amount, amount, ErrorCode::InsufficientFunds);

        let authority_seed = b"authority";
        let binding = [authority_seed.as_ref(), &[ctx.bumps.authority]];
        let seeds = &[&binding[..]];

        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.vault.to_account_info(),
                    to: ctx.accounts.withdrawer_usdc.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info(),
                },
                seeds,
            ),
            amount,
        )?;

        emit!(WithdrawnEvent {
            withdrawer: ctx.accounts.withdrawer.key(),
            amount,
        });

        Ok(())
    }

    pub fn fund(ctx: Context<Fund>, level: u8, amount: u64) -> Result<()> {
        require_keys_eq!(
            ctx.accounts.usdc_mint.key(),
            ctx.accounts.global.usdc_mint,
            ErrorCode::MintMismatch
        );
        require!(level >= 1 && level <= 6, ErrorCode::InvalidLevel);
        let l = level as usize - 1;

        let global = &mut ctx.accounts.global;
        require!(global.levels_enabled[l], ErrorCode::LevelNotEnabled);

        let decimals = ctx.accounts.usdc_mint.decimals;
        require!(decimals <= 18, ErrorCode::InvalidDecimals);
        let unit = 10u64.pow(decimals as u32);
        let min_amount = 1u64.checked_mul(unit).ok_or(ErrorCode::ArithmeticError)?;
        require_gte!(amount, min_amount, ErrorCode::AmountTooSmall);

        let max_per_account = LEVEL_MAX_PER_ACCOUNT[l];
        let max_per_lamports = if max_per_account == u64::MAX {
            u64::MAX
        } else {
            max_per_account.checked_mul(unit).ok_or(ErrorCode::ArithmeticError)?
        };

        let user_fund = &mut ctx.accounts.user_fund;
        // Set user if new (safe to set always since seeded)
        user_fund.user = ctx.accounts.user.key();

        let user_per_level = user_fund.per_level[l];
        require!(
            user_per_level.checked_add(amount).ok_or(ErrorCode::ArithmeticError)? <= max_per_lamports,
            ErrorCode::ExceedsMaxPerAccount
        );

        let total_this_level = global.total_per_level[l];
        let max_total_level = LEVEL_TOTAL_FUNDS[l].checked_mul(unit).ok_or(ErrorCode::ArithmeticError)?;
        require!(
            total_this_level.checked_add(amount).ok_or(ErrorCode::ArithmeticError)? <= max_total_level,
            ErrorCode::ExceedsLevelTotal
        );

        // Transfer tokens
        transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.user_usdc.to_account_info(),
                    to: ctx.accounts.vault.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            amount,
        )?;

        // Close level if reached its max
        if total_this_level.checked_add(amount).unwrap() == max_total_level {
            global.levels_enabled[l] = false;
        }

        // Update stats
        global.total_fund = global
            .total_fund
            .checked_add(amount)
            .ok_or(ErrorCode::ArithmeticError)?;
        global.total_per_level[l] = global.total_per_level[l]
            .checked_add(amount)
            .ok_or(ErrorCode::ArithmeticError)?;
        user_fund.total_funded = user_fund
            .total_funded
            .checked_add(amount)
            .ok_or(ErrorCode::ArithmeticError)?;
        user_fund.per_level[l] = user_fund.per_level[l]
            .checked_add(amount)
            .ok_or(ErrorCode::ArithmeticError)?;

        emit!(FundedEvent {
            user: ctx.accounts.user.key(),
            level,
            amount,
        });

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    pub usdc_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = admin,
        space = Global::LEN,
        seeds = [b"global"],
        bump,
    )]
    pub global: Account<'info, Global>,

    /// CHECK: PDA authority for the vault
    #[account(
        seeds = [b"authority"],
        bump,
    )]
    pub authority: UncheckedAccount<'info>,

    // Use Anchor's built-in ATA initialization
    #[account(
        init_if_needed,
        payer = admin,
        associated_token::mint = usdc_mint,
        associated_token::authority = authority,
    )]
    pub vault: Account<'info, TokenAccount>,

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
        constraint = global.usdc_mint == usdc_mint.key() @ ErrorCode::MintMismatch
    )]
    pub global: Account<'info, Global>,
    pub usdc_mint: Account<'info, Mint>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    pub withdrawer: Signer<'info>,
    #[account(
        seeds = [b"global"],
        bump,
    )]
    pub global: Account<'info, Global>,
    pub usdc_mint: Account<'info, Mint>,
    #[account(
        mut,
        associated_token::mint = usdc_mint,
        associated_token::authority = authority,
    )]
    pub vault: Account<'info, TokenAccount>,
    #[account(
        mut,
        associated_token::mint = usdc_mint,
        associated_token::authority = withdrawer,
    )]
    pub withdrawer_usdc: Account<'info, TokenAccount>,
    /// CHECK: PDA authority derived from known seeds, safe to use for token transfers
    #[account(
        seeds = [b"authority"],
        bump,
    )]
    pub authority: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Fund<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut,
        seeds = [b"global"],
        bump,
    )]
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
    #[account(
        mut,
        associated_token::mint = usdc_mint,
        associated_token::authority = user,
    )]
    pub user_usdc: Account<'info, TokenAccount>,
    #[account(
        mut,
        associated_token::mint = usdc_mint,
        associated_token::authority = authority,
    )]
    pub vault: Account<'info, TokenAccount>,
    /// CHECK: PDA authority derived from known seeds, safe to use for token transfers
    #[account(
        seeds = [b"authority"],
        bump,
    )]
    pub authority: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[account]
#[derive(Default)]
pub struct Global {
    pub admin: Pubkey,
    pub usdc_mint: Pubkey,
    pub total_fund: u64,
    pub levels_enabled: [bool; 6],
    pub total_per_level: [u64; 6]
}

impl Global {
    pub const LEN: usize = 8 + 32 + 32 + 8 + 6 * 1 + 6 * 8;
}

#[account]
#[derive(Default)]
pub struct UserFund {
    pub user: Pubkey,
    pub total_funded: u64,
    pub per_level: [u64; 6],
}

impl UserFund {
    pub const LEN: usize = 8 + 32 + 8 + 6 * 8;
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
    pub level: u8,
    pub amount: u64,
}

#[event]
pub struct WithdrawnEvent {
    pub withdrawer: Pubkey,
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