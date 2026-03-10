use anchor_lang::prelude::*;

use crate::constants::{
    MATCH_MODE_DUEL, MATCH_MODE_RANKED_SOLO, PLAYER_LIVE_STATE_SPACE, PLAYER_PROFILE_SPACE,
    PLAYER_REWARD_STATS_SPACE,
};
use crate::error::GuessrError;
use crate::instructions::leaderboard::update_leaderboards;
use crate::state::{LeaderboardState, PlayerLiveState, PlayerProfile, PlayerRewardStats};

#[derive(Clone, Copy)]
pub enum ProfileOutcome {
    Win,
    Loss,
    Draw,
}

#[derive(Clone, Copy)]
pub enum ProfileMatchMode {
    Duel,
    RankedSolo,
}

pub fn apply_profile_match_result(
    profile: &mut PlayerProfile,
    player: Pubkey,
    mode: ProfileMatchMode,
    outcome: ProfileOutcome,
    xp_gained: u64,
    earning_delta: i64,
    final_score: u64,
    bump: u8,
    now_ts: i64,
) -> Result<()> {
    profile.player = player;
    profile.total_xp = profile
        .total_xp
        .checked_add(xp_gained)
        .ok_or(GuessrError::Overflow)?;
    profile.net_earnings = profile
        .net_earnings
        .checked_add(earning_delta)
        .ok_or(GuessrError::Overflow)?;
    profile.total_matches = profile
        .total_matches
        .checked_add(1)
        .ok_or(GuessrError::Overflow)?;
    profile.last_match_score = final_score;
    profile.last_update_ts = now_ts;
    profile.bump = bump;
    profile.reserved = [0; 7];

    match (mode, outcome) {
        (ProfileMatchMode::Duel, ProfileOutcome::Win) => {
            profile.duel_wins = profile
                .duel_wins
                .checked_add(1)
                .ok_or(GuessrError::Overflow)?;
        }
        (ProfileMatchMode::Duel, ProfileOutcome::Loss) => {
            profile.duel_losses = profile
                .duel_losses
                .checked_add(1)
                .ok_or(GuessrError::Overflow)?;
        }
        (ProfileMatchMode::Duel, ProfileOutcome::Draw) => {}
        (ProfileMatchMode::RankedSolo, ProfileOutcome::Win) => {
            profile.ranked_wins = profile
                .ranked_wins
                .checked_add(1)
                .ok_or(GuessrError::Overflow)?;
        }
        (ProfileMatchMode::RankedSolo, ProfileOutcome::Loss) => {
            profile.ranked_losses = profile
                .ranked_losses
                .checked_add(1)
                .ok_or(GuessrError::Overflow)?;
        }
        (ProfileMatchMode::RankedSolo, ProfileOutcome::Draw) => {}
    }

    Ok(())
}

pub fn commit_match_result_handler(
    ctx: Context<CommitMatchResult>,
    match_mode: u8,
    did_win: bool,
    xp_gained: u64,
    earning_delta: i64,
    final_score: u64,
) -> Result<()> {
    require!(
        match_mode == MATCH_MODE_DUEL || match_mode == MATCH_MODE_RANKED_SOLO,
        GuessrError::InvalidMatchMode
    );

    let mode = if match_mode == MATCH_MODE_DUEL {
        ProfileMatchMode::Duel
    } else {
        ProfileMatchMode::RankedSolo
    };
    let outcome = if did_win {
        ProfileOutcome::Win
    } else {
        ProfileOutcome::Loss
    };

    apply_profile_match_result(
        &mut ctx.accounts.player_profile,
        ctx.accounts.player.key(),
        mode,
        outcome,
        xp_gained,
        earning_delta,
        final_score,
        ctx.bumps.player_profile,
        Clock::get()?.unix_timestamp,
    )
}

pub fn commit_match_result_pool_handler(
    ctx: Context<CommitMatchResultPool>,
    wallet_address: Pubkey,
    match_mode: u8,
    did_win: bool,
    xp_gained: u64,
    earning_delta: i64,
    final_score: u64,
) -> Result<()> {
    require!(
        match_mode == MATCH_MODE_DUEL || match_mode == MATCH_MODE_RANKED_SOLO,
        GuessrError::InvalidMatchMode
    );

    let mode = if match_mode == MATCH_MODE_DUEL {
        ProfileMatchMode::Duel
    } else {
        ProfileMatchMode::RankedSolo
    };
    let outcome = if did_win {
        ProfileOutcome::Win
    } else {
        ProfileOutcome::Loss
    };
    let now = Clock::get()?.unix_timestamp;

    apply_profile_match_result(
        &mut ctx.accounts.player_profile,
        wallet_address,
        mode,
        outcome,
        xp_gained,
        earning_delta,
        final_score,
        ctx.bumps.player_profile,
        now,
    )?;

    let rewards = &mut ctx.accounts.player_rewards;
    if rewards.player == Pubkey::default() {
        rewards.touch(wallet_address, ctx.bumps.player_rewards, now);
    }
    if earning_delta > 0 {
        rewards.total_earned = rewards
            .total_earned
            .checked_add(earning_delta as u64)
            .ok_or(GuessrError::Overflow)?;
    }
    rewards.last_update_ts = now;

    update_leaderboards(&mut ctx.accounts.leaderboard, &ctx.accounts.player_profile, rewards, now);

    Ok(())
}

pub fn init_player_accounts_handler(
    ctx: Context<InitPlayerAccounts>,
    wallet_address: Pubkey,
) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;

    let profile = &mut ctx.accounts.player_profile;
    if profile.player == Pubkey::default() {
        profile.player = wallet_address;
        profile.last_update_ts = now;
        profile.bump = ctx.bumps.player_profile;
        profile.reserved = [0; 7];
    }

    let rewards = &mut ctx.accounts.player_rewards;
    if rewards.player == Pubkey::default() {
        rewards.touch(wallet_address, ctx.bumps.player_rewards, now);
    }

    let live_state = &mut ctx.accounts.player_live_state;
    if live_state.player == Pubkey::default() {
        live_state.player = wallet_address;
        live_state.wallet_address = wallet_address;
        live_state.session_address = ctx.accounts.authority.key();
        live_state.room_id = [0u8; 32];
        live_state.round_index = 0;
        live_state.hp = 100;
        live_state.total_score = 0;
        live_state.earned_amount = 0;
        live_state.movement_hash = [0u8; 32];
        live_state.last_update_ts = now;
        live_state.bump = ctx.bumps.player_live_state;
        live_state.reserved = [0; 7];
    }

    Ok(())
}

#[derive(Accounts)]
pub struct CommitMatchResult<'info> {
    #[account(mut)]
    pub player: Signer<'info>,
    #[account(
        init_if_needed,
        payer = player,
        space = PLAYER_PROFILE_SPACE,
        seeds = [b"player-profile", player.key().as_ref()],
        bump
    )]
    pub player_profile: Account<'info, PlayerProfile>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(wallet_address: Pubkey)]
pub struct CommitMatchResultPool<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        init_if_needed,
        payer = authority,
        space = PLAYER_PROFILE_SPACE,
        seeds = [b"player-profile", wallet_address.as_ref()],
        bump
    )]
    pub player_profile: Account<'info, PlayerProfile>,
    #[account(
        init_if_needed,
        payer = authority,
        space = PLAYER_REWARD_STATS_SPACE,
        seeds = [b"player-rewards", wallet_address.as_ref()],
        bump
    )]
    pub player_rewards: Account<'info, PlayerRewardStats>,
    #[account(
        mut,
        seeds = [b"leaderboard"],
        bump = leaderboard.bump,
    )]
    pub leaderboard: Box<Account<'info, LeaderboardState>>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(wallet_address: Pubkey)]
pub struct InitPlayerAccounts<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        init_if_needed,
        payer = authority,
        space = PLAYER_PROFILE_SPACE,
        seeds = [b"player-profile", wallet_address.as_ref()],
        bump
    )]
    pub player_profile: Account<'info, PlayerProfile>,
    #[account(
        init_if_needed,
        payer = authority,
        space = PLAYER_REWARD_STATS_SPACE,
        seeds = [b"player-rewards", wallet_address.as_ref()],
        bump
    )]
    pub player_rewards: Account<'info, PlayerRewardStats>,
    #[account(
        init_if_needed,
        payer = authority,
        space = PLAYER_LIVE_STATE_SPACE,
        seeds = [b"player-live-state", wallet_address.as_ref()],
        bump
    )]
    pub player_live_state: Account<'info, PlayerLiveState>,
    pub system_program: Program<'info, System>,
}
