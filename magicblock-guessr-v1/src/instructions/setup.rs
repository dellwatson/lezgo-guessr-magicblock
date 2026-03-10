use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;

use crate::constants::{LEADERBOARD_SPACE, LOBBY_STATE_SPACE, RANKED_CONFIG_SPACE, ROOM_POOL_SPACE};
use crate::error::GuessrError;
use crate::state::{LeaderboardState, LobbyState, RankedConfig, RoomPool, RoomPoolEntry};

pub fn initialize_system_handler(
    ctx: Context<InitializeSystem>,
    heartbeat_ttl_sec: i64,
    reward_mint: Pubkey,
    reward_multiplier: u64,
    penalty_divisor: u64,
    penalty_threshold: u64,
) -> Result<()> {
    require!(heartbeat_ttl_sec > 0, GuessrError::InvalidHeartbeatTtl);
    require!(reward_multiplier > 0, GuessrError::InvalidMultiplier);
    require!(penalty_divisor > 0, GuessrError::InvalidPenaltyDivisor);
    require!(
        reward_mint == ctx.accounts.reward_mint.key(),
        GuessrError::InvalidRewardMint
    );

    let lobby = &mut ctx.accounts.lobby_state;
    lobby.authority = ctx.accounts.authority.key();
    lobby.heartbeat_ttl_sec = heartbeat_ttl_sec;
    lobby.online_players = 0;
    lobby.bump = ctx.bumps.lobby_state;
    lobby.reserved = [0; 19];

    let config = &mut ctx.accounts.ranked_config;
    config.authority = ctx.accounts.authority.key();
    config.reward_mint = reward_mint;
    config.treasury_token_account = ctx.accounts.treasury_token_account.key();
    config.reward_multiplier = reward_multiplier;
    config.penalty_divisor = penalty_divisor;
    config.penalty_threshold = penalty_threshold;
    config.mint_authority_bump = ctx.bumps.mint_authority;
    config.bump = ctx.bumps.ranked_config;
    config.reserved = [0; 13];

    let room_pool = &mut ctx.accounts.room_pool;
    room_pool.write_index = 0;
    room_pool.entry_count = 0;
    room_pool.entries = [RoomPoolEntry::default(); crate::constants::ROOM_POOL_MAX_ENTRIES];
    room_pool.bump = ctx.bumps.room_pool;
    room_pool.reserved = [0; 7];

    Ok(())
}

pub fn initialize_leaderboard_handler(ctx: Context<InitializeLeaderboard>) -> Result<()> {
    ctx.accounts
        .leaderboard
        .reset(ctx.bumps.leaderboard, Clock::get()?.unix_timestamp);
    Ok(())
}

#[derive(Accounts)]
pub struct InitializeSystem<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        init,
        payer = authority,
        space = LOBBY_STATE_SPACE,
        seeds = [b"lobby-state"],
        bump
    )]
    pub lobby_state: Account<'info, LobbyState>,
    #[account(
        init,
        payer = authority,
        space = RANKED_CONFIG_SPACE,
        seeds = [b"ranked-config"],
        bump
    )]
    pub ranked_config: Account<'info, RankedConfig>,
    #[account(
        init,
        payer = authority,
        space = ROOM_POOL_SPACE,
        seeds = [b"room-pool"],
        bump
    )]
    pub room_pool: Account<'info, RoomPool>,
    pub reward_mint: UncheckedAccount<'info>,
    #[account(
        mut,
        constraint = treasury_token_account.mint == reward_mint.key() @ GuessrError::InvalidTreasuryMint,
    )]
    pub treasury_token_account: Account<'info, TokenAccount>,
    /// CHECK: PDA signer for token mint authority.
    #[account(seeds = [b"mint-authority"], bump)]
    pub mint_authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializeLeaderboard<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        seeds = [b"lobby-state"],
        bump = lobby_state.bump,
        constraint = lobby_state.authority == authority.key() @ GuessrError::Unauthorized,
    )]
    pub lobby_state: Account<'info, LobbyState>,
    #[account(
        init,
        payer = authority,
        space = LEADERBOARD_SPACE,
        seeds = [b"leaderboard"],
        bump
    )]
    pub leaderboard: Box<Account<'info, LeaderboardState>>,
    pub system_program: Program<'info, System>,
}
