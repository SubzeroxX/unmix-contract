use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{transfer, Mint, Token, TokenAccount, Transfer};

declare_id!("CbPxrLPpxo6Mp25XcDpbFwqkguFZetbuR6oWrNfscFSw");

const WITHDRAW_AUTHORITY: Pubkey = pubkey!("47Bxttko5pewcjuwd4oTzFeJmHY2yfs4j4YFXx2d68vV");

const USDC_MINT: Pubkey = pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
const USDT_MINT: Pubkey = pubkey!("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB");

#[program]
pub mod un_mix {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let mint_key = ctx.accounts.mint.key();

        // Only allow official USDC or USDT mints (legacy SPL Token expected)
        require!(
            mint_key == USDC_MINT || mint_key == USDT_MINT,
            ErrorCode::InvalidMint
        );

        let authority_key = ctx.accounts.authority.key();
        let expected_vault = anchor_spl::associated_token::get_associated_token_address(
            &authority_key,
            &mint_key,
        );
        require_keys_eq!(
            ctx.accounts.vault.key(),
            expected_vault,
            ErrorCode::InvalidVault
        );

        // Vault must match the mint being initialized
        require_keys_eq!(
            ctx.accounts.vault.mint,
            mint_key,
            ErrorCode::MintMismatch
        );

        let global = &mut ctx.accounts.global;

        // First call must set admin (only once)
        if global.admin == Pubkey::default() {
            global.admin = ctx.accounts.admin.key();
            global.bump = ctx.bumps.authority; // Store canonical bump
        } else {
            // Subsequent calls must be from the same admin
            require_keys_eq!(
                ctx.accounts.admin.key(),
                global.admin,
                ErrorCode::Unauthorized
            );
        }

        // Prevent re-setting the same mint
        if mint_key == USDC_MINT {
            require_keys_eq!(
                global.usdc_mint,
                Pubkey::default(),
                ErrorCode::MintAlreadySet
            );
            global.usdc_mint = mint_key;
        } else {
            require_keys_eq!(
                global.usdt_mint,
                Pubkey::default(),
                ErrorCode::MintAlreadySet
            );
            global.usdt_mint = mint_key;
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
            ctx.accounts.admin.key(),
            ctx.accounts.global.admin,
            ErrorCode::Unauthorized
        );
        require!(level >= 1 && level <= 6, ErrorCode::InvalidLevel);

        let idx = (level as usize) - 1;
        ctx.accounts.global.levels_enabled[idx] = enabled;

        emit!(LevelUpdatedEvent {
            admin: ctx.accounts.admin.key(),
            level,
            enabled,
        });

        Ok(())
    }

    pub fn fund(ctx: Context<Fund>, level: u8, amount: u64, mint: Pubkey) -> Result<()> {
        require!(amount > 0, ErrorCode::AmountTooSmall);
        require!(level >= 1 && level <= 6, ErrorCode::InvalidLevel);
        let idx = (level as usize) - 1;

        let global = &mut ctx.accounts.global;
        require!(global.levels_enabled[idx], ErrorCode::LevelNotEnabled);

        let is_usdc = mint == global.usdc_mint && mint == USDC_MINT;
        let is_usdt = mint == global.usdt_mint && mint == USDT_MINT;
        require!(is_usdc || is_usdt, ErrorCode::InvalidMint);

        let decimals = ctx.accounts.mint.decimals;
        require!(decimals <= 18, ErrorCode::InvalidDecimals);
        let unit = 10u64.pow(decimals as u32);
        let min_amount = unit;
        require_gte!(amount, min_amount, ErrorCode::AmountTooSmall);

        let max_per_account_tokens = LEVEL_MAX_PER_ACCOUNT[idx];
        let max_per_account = if max_per_account_tokens == u64::MAX {
            u64::MAX
        } else {
            max_per_account_tokens
                .checked_mul(unit)
                .ok_or(ErrorCode::ArithmeticError)?
        };

        let user_fund = &mut ctx.accounts.user_fund;
        user_fund.user = ctx.accounts.user.key();

        let current_user_contribution = if is_usdc {
            user_fund.per_level_usdc[idx]
        } else {
            user_fund.per_level_usdt[idx]
        };

        require!(
            current_user_contribution
                .checked_add(amount)
                .ok_or(ErrorCode::ArithmeticError)?
                <= max_per_account,
            ErrorCode::ExceedsMaxPerAccount
        );

        user_fund.total_funded = user_fund
            .total_funded
            .checked_add(amount)
            .ok_or(ErrorCode::ArithmeticError)?;

        if is_usdc {
            user_fund.per_level_usdc[idx] = user_fund.per_level_usdc[idx]
                .checked_add(amount)
                .ok_or(ErrorCode::ArithmeticError)?;
        } else {
            user_fund.per_level_usdt[idx] = user_fund.per_level_usdt[idx]
                .checked_add(amount)
                .ok_or(ErrorCode::ArithmeticError)?;
        }

        let current_level_total = global.total_per_level[idx];
        let max_level_amount = LEVEL_TOTAL_FUNDS[idx]
            .checked_mul(unit)
            .ok_or(ErrorCode::ArithmeticError)?;

        require!(
            current_level_total
                .checked_add(amount)
                .ok_or(ErrorCode::ArithmeticError)?
                <= max_level_amount,
            ErrorCode::ExceedsLevelTotal
        );

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

        global.total_per_level[idx] = current_level_total
            .checked_add(amount)
            .ok_or(ErrorCode::ArithmeticError)?;

        global.total_fund = global
            .total_fund
            .checked_add(amount)
            .ok_or(ErrorCode::ArithmeticError)?;

        if global.total_per_level[idx] >= max_level_amount {
            global.levels_enabled[idx] = false;
        }

        emit!(FundedEvent {
            user: ctx.accounts.user.key(),
            level,
            amount,
            mint,
        });

        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64, mint: Pubkey) -> Result<()> {
        require!(amount > 0, ErrorCode::AmountTooSmall);
        require_keys_eq!(
            ctx.accounts.withdrawer.key(),
            WITHDRAW_AUTHORITY,
            ErrorCode::Unauthorized
        );

        let global = &ctx.accounts.global;
        let is_usdc = mint == global.usdc_mint;
        let is_usdt = mint == global.usdt_mint;
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
            withdrawer: ctx.accounts.withdrawer.key(),
            amount,
            mint,
        });

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    pub mint: Account<'info, Mint>,

    #[account(
        init,
        payer = admin,
        space = Global::LEN,
        seeds = [b"global"],
        bump,
    )]
    pub global: Account<'info, Global>,

    /// CHECK: PDA authority used as vault owner
    #[account(seeds = [b"authority"], bump)]
    pub authority: UncheckedAccount<'info>,

    #[account(
        init_if_needed,
        payer = admin,
        associated_token::mint = mint,
        associated_token::authority = authority,
        constraint = vault.mint == mint.key() @ ErrorCode::MintMismatch
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
        space = UserFund::LEN,
        seeds = [b"user_fund", user.key().as_ref()],
        bump,
    )]
    pub user_fund: Account<'info, UserFund>,

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
        constraint = vault.mint == mint.key() @ ErrorCode::MintMismatch
    )]
    pub vault: Account<'info, TokenAccount>,

    /// CHECK: PDA
    #[account(seeds = [b"authority"], bump = global.bump)]
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
        constraint = vault.mint == mint.key() @ ErrorCode::MintMismatch
    )]
    pub vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = withdrawer,
    )]
    pub withdrawer_token_account: Account<'info, TokenAccount>,

    /// CHECK: PDA
    #[account(seeds = [b"authority"], bump = global.bump)]
    pub authority: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

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
    pub bump: u8,               // ← Added: canonical bump for authority PDA
}

impl Global {
    pub const LEN: usize = 8 + 32 * 3 + 8 * 3 + 6 + 8 * 6 + 1;
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
    pub const LEN: usize = 8 + 32 + 8 + 8 * 6 * 2;
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

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid level")]
    InvalidLevel,
    #[msg("Insufficient funds in vault")]
    InsufficientFunds,
    #[msg("Level is not enabled")]
    LevelNotEnabled,
    #[msg("Amount too small")]
    AmountTooSmall,
    #[msg("Exceeds max per account for this level")]
    ExceedsMaxPerAccount,
    #[msg("Exceeds total allowed for this level")]
    ExceedsLevelTotal,
    #[msg("Mint not supported")]
    InvalidMint,
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Arithmetic overflow/underflow")]
    ArithmeticError,
    #[msg("Invalid mint decimals (>18)")]
    InvalidDecimals,
    #[msg("Invalid vault account")]
    InvalidVault,
    #[msg("Mint mismatch")]
    MintMismatch,
    #[msg("Mint already initialized")]
    MintAlreadySet,
}
