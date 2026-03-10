use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount};

use crate::constants::REWARD_CLAIM_SPACE;
use crate::error::GuessrError;
use crate::state::{RankedConfig, RewardClaim};

pub fn create_reward_claim_handler(
    ctx: Context<CreateRewardClaim>,
    match_id: [u8; 32],
    mode: u8,
    region: u8,
    amount: u64,
) -> Result<()> {
    let claim = &mut ctx.accounts.reward_claim;
    let now = Clock::get()?.unix_timestamp;

    require!(amount > 0, GuessrError::InvalidAmount);

    claim.player = ctx.accounts.player.key();
    claim.match_id = match_id;
    claim.mode = mode;
    claim.region = region;
    claim.amount = amount;
    claim.claimed = false;
    claim.created_ts = now;
    claim.bump = ctx.bumps.reward_claim;
    claim.reserved = [0; 6];

    Ok(())
}

pub fn claim_reward_handler(ctx: Context<ClaimReward>) -> Result<()> {
    let claim = &mut ctx.accounts.reward_claim;

    require!(!claim.claimed, GuessrError::AlreadyClaimed);
    require!(claim.amount > 0, GuessrError::InvalidAmount);
    require!(
        claim.player == ctx.accounts.player.key(),
        GuessrError::Unauthorized
    );

    let transfer_amount = claim.amount;

    let cpi_accounts = token::Transfer {
        from: ctx.accounts.treasury_token_account.to_account_info(),
        to: ctx.accounts.player_token_account.to_account_info(),
        authority: ctx.accounts.treasury_authority.to_account_info(),
    };

    token::transfer(
        CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts),
        transfer_amount,
    )?;

    claim.claimed = true;

    Ok(())
}

#[derive(Accounts)]
#[instruction(match_id: [u8; 32], mode: u8)]
pub struct CreateRewardClaim<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut)]
    pub player: Signer<'info>,
    #[account(
        init_if_needed,
        payer = payer,
        space = REWARD_CLAIM_SPACE,
        seeds = [b"reward-claim", player.key().as_ref(), match_id.as_ref(), &[mode]],
        bump,
    )]
    pub reward_claim: Account<'info, RewardClaim>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ClaimReward<'info> {
    #[account(mut)]
    pub player: Signer<'info>,
    #[account(
        mut,
        seeds = [b"reward-claim", player.key().as_ref(), reward_claim.match_id.as_ref(), &[reward_claim.mode]],
        bump = reward_claim.bump,
        constraint = reward_claim.player == player.key() @ GuessrError::Unauthorized,
    )]
    pub reward_claim: Account<'info, RewardClaim>,
    #[account(
        mut,
        seeds = [b"ranked-config"],
        bump = ranked_config.bump,
    )]
    pub ranked_config: Account<'info, RankedConfig>,
    #[account(
        mut,
        address = ranked_config.treasury_token_account,
    )]
    pub treasury_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = treasury_authority.key() == ranked_config.authority @ GuessrError::Unauthorized,
    )]
    pub treasury_authority: Signer<'info>,
    #[account(
        mut,
        constraint = player_token_account.owner == player.key() @ GuessrError::InvalidTokenOwner,
    )]
    pub player_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}
