use anchor_lang::prelude::*;

use crate::constants::{MATCH_MODE_DUEL, MATCH_MODE_RANKED_SOLO, PLAYER_PROFILE_SPACE};
use crate::error::GuessrError;
use crate::state::PlayerProfile;

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
