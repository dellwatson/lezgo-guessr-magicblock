use anchor_lang::prelude::*;

use crate::constants::{PLAYER_PROFILE_SPACE, PLAYER_REWARD_STATS_SPACE};
use crate::error::GuessrError;
use crate::instructions::leaderboard::update_leaderboards;
use crate::instructions::profile::{apply_profile_match_result, ProfileMatchMode, ProfileOutcome};
use crate::instructions::{ensure_player_authority, ensure_wallet_matches_status};
use crate::state::{DuelRoom, LeaderboardState, PlayerProfile, PlayerRewardStats, PlayerStatus};

pub fn settle_duel_room_handler(
    ctx: Context<SettleDuelRoom>,
    wallet_address: Pubkey,
    room_id: [u8; 32], // room_id is a string and unique too

    winner: Pubkey,
    is_draw: bool, // cant be draw, if draw winner is zero?
    host_score: u64,
    challenger_score: u64, // there's no challenger score, score is on pub
                           // XP ?
) -> Result<()> {
    let player_key = ctx.accounts.player_status.player;
    let player_status = &mut ctx.accounts.player_status;
    let duel_room = &mut ctx.accounts.duel_room;
    let host_profile = &mut ctx.accounts.host_profile;
    let challenger_profile = &mut ctx.accounts.challenger_profile;
    let now = Clock::get()?.unix_timestamp;

    ensure_wallet_matches_status(player_status, wallet_address)?;
    ensure_player_authority(player_status, ctx.accounts.authority.key())?;
    require!(duel_room.room_id == room_id, GuessrError::RoomMismatch);
    require!(!duel_room.is_settled, GuessrError::DuelAlreadySettled);
    require!(
        duel_room.host == player_key || duel_room.challenger == player_key,
        GuessrError::PlayerNotInRoom
    );
    require!(
        duel_room.challenger != Pubkey::default(),
        GuessrError::DuelRoomIncomplete
    );

    if !is_draw {
        require!(
            winner == duel_room.host || winner == duel_room.challenger,
            GuessrError::InvalidWinner
        );
        duel_room.winner = winner;
    } else {
        duel_room.winner = Pubkey::default();
    }

    duel_room.host_score = host_score;
    duel_room.challenger_score = challenger_score;
    duel_room.is_settled = true;
    duel_room.last_update_ts = now;

    let host_earning_delta = if duel_room.host_earned >= duel_room.host_lost {
        let delta: i64 = duel_room
            .host_earned
            .checked_sub(duel_room.host_lost)
            .ok_or(GuessrError::Underflow)?
            .try_into()
            .map_err(|_| GuessrError::Overflow)?;
        delta
    } else {
        let delta: i64 = duel_room
            .host_lost
            .checked_sub(duel_room.host_earned)
            .ok_or(GuessrError::Underflow)?
            .try_into()
            .map_err(|_| GuessrError::Overflow)?;
        delta.checked_neg().ok_or(GuessrError::Overflow)?
    };
    let challenger_earning_delta = if duel_room.challenger_earned >= duel_room.challenger_lost {
        let delta: i64 = duel_room
            .challenger_earned
            .checked_sub(duel_room.challenger_lost)
            .ok_or(GuessrError::Underflow)?
            .try_into()
            .map_err(|_| GuessrError::Overflow)?;
        delta
    } else {
        let delta: i64 = duel_room
            .challenger_lost
            .checked_sub(duel_room.challenger_earned)
            .ok_or(GuessrError::Underflow)?
            .try_into()
            .map_err(|_| GuessrError::Overflow)?;
        delta.checked_neg().ok_or(GuessrError::Overflow)?
    };

    let host_xp = 15_u64
        .checked_add(host_score.checked_div(20).ok_or(GuessrError::Overflow)?)
        .ok_or(GuessrError::Overflow)?
        .checked_add(if !is_draw && winner == duel_room.host {
            25
        } else {
            8
        })
        .ok_or(GuessrError::Overflow)?;
    let challenger_xp = 15_u64
        .checked_add(
            challenger_score
                .checked_div(20)
                .ok_or(GuessrError::Overflow)?,
        )
        .ok_or(GuessrError::Overflow)?
        .checked_add(if !is_draw && winner == duel_room.challenger {
            25
        } else {
            8
        })
        .ok_or(GuessrError::Overflow)?;

    let host_outcome = if is_draw {
        ProfileOutcome::Draw
    } else if winner == duel_room.host {
        ProfileOutcome::Win
    } else {
        ProfileOutcome::Loss
    };
    let challenger_outcome = if is_draw {
        ProfileOutcome::Draw
    } else if winner == duel_room.challenger {
        ProfileOutcome::Win
    } else {
        ProfileOutcome::Loss
    };

    apply_profile_match_result(
        host_profile,
        duel_room.host,
        ProfileMatchMode::Duel,
        host_outcome,
        host_xp,
        host_earning_delta,
        host_score,
        ctx.bumps.host_profile,
        now,
    )?;
    apply_profile_match_result(
        challenger_profile,
        duel_room.challenger,
        ProfileMatchMode::Duel,
        challenger_outcome,
        challenger_xp,
        challenger_earning_delta,
        challenger_score,
        ctx.bumps.challenger_profile,
        now,
    )?;

    let host_rewards = &mut ctx.accounts.host_rewards;
    if host_rewards.player == Pubkey::default() {
        host_rewards.touch(duel_room.host, ctx.bumps.host_rewards, now);
    }
    host_rewards.total_earned = host_rewards
        .total_earned
        .checked_add(duel_room.host_earned)
        .ok_or(GuessrError::Overflow)?;
    host_rewards.last_update_ts = now;

    let challenger_rewards = &mut ctx.accounts.challenger_rewards;
    if challenger_rewards.player == Pubkey::default() {
        challenger_rewards.touch(duel_room.challenger, ctx.bumps.challenger_rewards, now);
    }
    challenger_rewards.total_earned = challenger_rewards
        .total_earned
        .checked_add(duel_room.challenger_earned)
        .ok_or(GuessrError::Overflow)?;
    challenger_rewards.last_update_ts = now;

    update_leaderboards(&mut ctx.accounts.leaderboard, host_profile, host_rewards, now);
    update_leaderboards(
        &mut ctx.accounts.leaderboard,
        challenger_profile,
        challenger_rewards,
        now,
    );

    if player_status.active_room == room_id {
        player_status.active_room = [0u8; 32];
    }

    Ok(())
}

#[derive(Accounts)]
#[instruction(wallet_address: Pubkey, room_id: [u8; 32])]
pub struct SettleDuelRoom<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [b"player-status", wallet_address.as_ref()],
        bump = player_status.bump,
        constraint = player_status.player == wallet_address @ GuessrError::Unauthorized
    )]
    pub player_status: Account<'info, PlayerStatus>,
    #[account(
        mut,
        seeds = [b"duel-room", room_id.as_ref()],
        bump = duel_room.bump
    )]
    pub duel_room: Account<'info, DuelRoom>,
    #[account(
        init_if_needed,
        payer = authority,
        space = PLAYER_PROFILE_SPACE,
        seeds = [b"player-profile", duel_room.host.as_ref()],
        bump
    )]
    pub host_profile: Account<'info, PlayerProfile>,
    #[account(
        init_if_needed,
        payer = authority,
        space = PLAYER_PROFILE_SPACE,
        seeds = [b"player-profile", duel_room.challenger.as_ref()],
        bump
    )]
    pub challenger_profile: Account<'info, PlayerProfile>,
    #[account(
        init_if_needed,
        payer = authority,
        space = PLAYER_REWARD_STATS_SPACE,
        seeds = [b"player-rewards", duel_room.host.as_ref()],
        bump
    )]
    pub host_rewards: Account<'info, PlayerRewardStats>,
    #[account(
        init_if_needed,
        payer = authority,
        space = PLAYER_REWARD_STATS_SPACE,
        seeds = [b"player-rewards", duel_room.challenger.as_ref()],
        bump
    )]
    pub challenger_rewards: Account<'info, PlayerRewardStats>,
    #[account(
        mut,
        seeds = [b"leaderboard"],
        bump = leaderboard.bump,
    )]
    pub leaderboard: Box<Account<'info, LeaderboardState>>,
    pub system_program: Program<'info, System>,
}
