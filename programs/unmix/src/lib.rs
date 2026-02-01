use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount, transfer, Transfer};
use anchor_spl::associated_token::AssociatedToken;

declare_id!("CrBMthLXkbuE31sxgfZvayRbvpMDQm47okm9mdaqkTkW");

#[program]
pub mod un_mix {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let authority_key = ctx.accounts.authority.key();

        // Verify vault address
        require_keys_eq!(
            ctx.accounts.vault.key(),
            anchor_spl::associated_token::get_associated_token_address(
                &authority_key,
                &ctx.accounts.usdc_mint.key()
            )
        );

        // Create vault if it doesn't exist
        if ctx.accounts.vault.to_account_info().data_is_empty() {
            anchor_spl::associated_token::create(
                CpiContext::new(
                    ctx.accounts.associated_token_program.to_account_info(),
                    anchor_spl::associated_token::Create {
                        payer: ctx.accounts.admin.to_account_info(),
                        associated_token: ctx.accounts.vault.to_account_info(),
                        authority: ctx.accounts.authority.to_account_info(),
                        mint: ctx.accounts.usdc_mint.to_account_info(),
                        system_program: ctx.accounts.system_program.to_account_info(),
                        token_program: ctx.accounts.token_program.to_account_info(),
                    },
                ),
            )?;
        }

        let global = &mut ctx.accounts.global;
        global.admin = ctx.accounts.admin.key();
        global.usdc_mint = ctx.accounts.usdc_mint.key();
        global.total_fund = 0;
        global.levels_enabled = [false; 6];
        global.total_per_level = [0; 6];

        Ok(())
    }

    pub fn enable_level(ctx: Context<EnableLevel>, level: u8, enabled: bool) -> Result<()> {
        require_keys_eq!(ctx.accounts.admin.key(), ctx.accounts.global.admin, ErrorCode::Unauthorized);
        require!(level >= 1 && level <= 6, ErrorCode::InvalidLevel);
        let global = &mut ctx.accounts.global;
        global.levels_enabled[level as usize - 1] = enabled;
        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        require_keys_eq!(ctx.accounts.admin.key(), ctx.accounts.global.admin, ErrorCode::Unauthorized);
        if amount == 0 {
            return Ok(());
        }

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
                    to: ctx.accounts.admin_usdc.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info(),
                },
                seeds,
            ),
            amount,
        )?;

        Ok(())
    }

    pub fn fund(ctx: Context<Fund>, level: u8, amount: u64) -> Result<()> {
        require!(level >= 1 && level <= 6, ErrorCode::InvalidLevel);
        let l = level as usize - 1;

        let global = &mut ctx.accounts.global;
        require!(global.levels_enabled[l], ErrorCode::LevelNotEnabled);

        let decimals = ctx.accounts.usdc_mint.decimals;
        let unit = 10u64.pow(decimals as u32);
        let min_amount = 1 * unit;
        require_gte!(amount, min_amount, ErrorCode::AmountTooSmall);

        let max_per_account = LEVEL_MAX_PER_ACCOUNT[l];
        let max_per_lamports = if max_per_account == u64::MAX {
            u64::MAX
        } else {
            max_per_account * unit
        };

        let user_fund = &mut ctx.accounts.user_fund;
        // Set user if new (safe to set always since seeded)
        user_fund.user = ctx.accounts.user.key();

        let user_per_level = user_fund.per_level[l];
        require!(user_per_level + amount <= max_per_lamports, ErrorCode::ExceedsMaxPerAccount);

        let total_this_level = global.total_per_level[l];
        let max_total_level = LEVEL_TOTAL_FUNDS[l] * unit;
        require!(total_this_level + amount <= max_total_level, ErrorCode::ExceedsLevelTotal);

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

        // Update stats
        global.total_fund += amount;
        global.total_per_level[l] += amount;
        user_fund.total_funded += amount;
        user_fund.per_level[l] += amount;

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
    )]
    pub global: Account<'info, Global>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    pub admin: Signer<'info>,
    #[account(
        mut,
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
        associated_token::authority = admin,
    )]
    pub admin_usdc: Account<'info, TokenAccount>,
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
    pub total_per_level: [u64; 6],
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

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid level.")]
    InvalidLevel,
    #[msg("Insuffitient funds")]
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
    Unauthorized
}