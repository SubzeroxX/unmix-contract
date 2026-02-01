use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Mint};
use anchor_spl::associated_token::AssociatedToken;

declare_id!("6K2jFphyzDFougxXgywW1YoUoFJXeydPe6KVztk9J5Uv");

#[program]
pub mod fundme_token {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let fund_data = &mut ctx.accounts.fund_data;
        fund_data.admin = ctx.accounts.admin.key();
        fund_data.total_raised = 0;
        fund_data.round_id = 1; // Start at Round 1
        fund_data.bump = ctx.bumps.fund_data;
        Ok(())
    }

    /// The "Power Reset": Increments the round_id. 
    /// This effectively resets EVERY donor to 0 for the new round.
    pub fn reset_round(ctx: Context<Reset>) -> Result<()> {
        let fund_data = &mut ctx.accounts.fund_data;
        require_keys_eq!(fund_data.admin, ctx.accounts.admin.key(), FundError::Unauthorized);
        
        // Increment round and clear total
        fund_data.round_id = fund_data.round_id.checked_add(1).unwrap();
        fund_data.total_raised = 0;
        
        msg!("New Round Started: {}. Everyone reset to 0!", fund_data.round_id);
        Ok(())
    }

    pub fn fund(ctx: Context<Fund>, amount: u64) -> Result<()> {
        require!(amount >= 1_000_000, FundError::TooFewFund);
        require!(amount <= 10_000_000, FundError::TooMuchFund);

        let record = &mut ctx.accounts.donation_record;
        record.donor = ctx.accounts.user.key();
        // Add to existing amount for THIS round
        record.amount = record.amount.checked_add(amount).unwrap();

        let fund_data = &mut ctx.accounts.fund_data;
        fund_data.total_raised = fund_data.total_raised.checked_add(amount).unwrap();

        let cpi_accounts = token::Transfer {
            from: ctx.accounts.user_token_account.to_account_info(),
            to: ctx.accounts.vault_token_account.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        let fund_data = &ctx.accounts.fund_data;
        require_keys_eq!(fund_data.admin, ctx.accounts.admin.key(), FundError::Unauthorized);

        let seeds = &[b"fund_data".as_ref(), &[fund_data.bump]];
        let signer = &[&seeds[..]];

        let cpi_accounts = token::Transfer {
            from: ctx.accounts.vault_token_account.to_account_info(),
            to: ctx.accounts.admin_token_account.to_account_info(),
            authority: fund_data.to_account_info(), 
        };
        let cpi_ctx = CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(), cpi_accounts, signer);
        token::transfer(cpi_ctx, amount)?;

        Ok(())
    }
}

// --- Contexts ---

#[derive(Accounts)]
pub struct Fund<'info> {
    #[account(
        init_if_needed, 
        payer = user, 
        space = 8 + DonationRecord::INIT_SPACE, 
        // SEED CHANGE: Includes round_id so users get a fresh account every round
        seeds = [b"donation", user.key().as_ref(), &fund_data.round_id.to_le_bytes()], 
        bump
    )]
    pub donation_record: Account<'info, DonationRecord>,

    #[account(mut, seeds = [b"fund_data"], bump = fund_data.bump)]
    pub fund_data: Account<'info, FundData>,

    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub vault_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

// (Initialize, Reset, and Withdraw remain similar but use the updated FundData)

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = admin, space = 8 + 32 + 8 + 8 + 1, seeds = [b"fund_data"], bump)]
    pub fund_data: Account<'info, FundData>,
    pub token_mint: Account<'info, Mint>,
    #[account(init, payer = admin, associated_token::mint = token_mint, associated_token::authority = fund_data)]
    pub vault_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

#[derive(Accounts)]
pub struct Reset<'info> {
    #[account(mut, seeds = [b"fund_data"], bump = fund_data.bump)]
    pub fund_data: Account<'info, FundData>,
    #[account(mut)]
    pub admin: Signer<'info>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(seeds = [b"fund_data"], bump = fund_data.bump)]
    pub fund_data: Account<'info, FundData>,
    #[account(mut)]
    pub vault_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub admin_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub admin: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

#[account]
pub struct FundData {
    pub admin: Pubkey,
    pub total_raised: u64,
    pub round_id: u64, // The "Season" counter
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct DonationRecord {
    pub donor: Pubkey,
    pub amount: u64,
}

#[error_code]
pub enum FundError {
    #[msg("Unauthorized: Admin only.")]
    Unauthorized,
    #[msg("Min donation is 1 USDC.")]
    TooFewFund,
    #[msg("Max donation is 10 USDC.")]
    TooMuchFund,
}