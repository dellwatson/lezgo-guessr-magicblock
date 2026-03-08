use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, MintTo, Token, TokenAccount, Transfer};

use crate::constants::{
    ACTION_GUESS_SUBMIT, ACTION_HINT_OPEN, ACTION_MARK_MOVE, MAX_ACCURACY_BPS,
    PLAYER_LIVE_STATE_SPACE, PLAYER_PROFILE_SPACE, RANKED_ROOM_SPACE,
};
use crate::error::GuessrError;
use crate::instructions::profile::{
    apply_profile_match_result, ProfileMatchMode, ProfileOutcome,
};
use crate::state::{PlayerLiveState, PlayerProfile, PlayerStatus, RankedConfig, RankedRoom};

pub fn set_reward_mint_handler(ctx: Context<SetRewardMint>, reward_mint: Pubkey) -> Result<()> {
    let config = &mut ctx.accounts.ranked_config;
    config.reward_mint = reward_mint;
    config.treasury_token_account = ctx.accounts.treasury_token_account.key();
    Ok(())
}

pub fn open_ranked_room_handler(
    ctx: Context<OpenRankedRoom>,
    challenge_hash: [u8; 32],
) -> Result<()> {
    let ranked_room = &mut ctx.accounts.ranked_room;
    let player_status = &mut ctx.accounts.player_status;
    ranked_room.player = ctx.accounts.player.key();
    ranked_room.challenge_hash = challenge_hash;
    ranked_room.score = 0;
    ranked_room.total_earned = 0;
    ranked_room.total_lost = 0;
    ranked_room.action_count = 0;
    ranked_room.hints_opened = 0;
    ranked_room.last_round_index = 0;
    ranked_room.last_hp = 100;
    ranked_room.last_accuracy_bps = 0;
    ranked_room.last_distance_km = 0;
    ranked_room.last_action_kind = ACTION_HINT_OPEN;
    ranked_room.is_settled = false;
    ranked_room.last_movement_hash = [0; 32];
    ranked_room.last_action_ts = Clock::get()?.unix_timestamp;
    ranked_room.bump = ctx.bumps.ranked_room;
    ranked_room.reserved = [0; 7];

    require!(player_status.is_online, GuessrError::PlayerOffline);
    player_status.active_room = ranked_room.challenge_hash;
    player_status.last_heartbeat_ts = Clock::get()?.unix_timestamp;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn update_ranked_state_handler(
    ctx: Context<UpdateRankedState>,
    round_index: u16,
    hp_after: u16,
    distance_km: u32,
    accuracy_bps: u16,
    action_kind: u8,
    is_correct_country: bool,
    total_score: u64,
    movement_hash: [u8; 32],
) -> Result<()> {
    let ranked_room = &mut ctx.accounts.ranked_room;
    let player_status = &mut ctx.accounts.player_status;
    let live_state = &mut ctx.accounts.player_live_state;
    let config = &ctx.accounts.ranked_config;
    let now = Clock::get()?.unix_timestamp;

    require!(!ranked_room.is_settled, GuessrError::RoomAlreadySettled);
    require!(player_status.is_online, GuessrError::PlayerOffline);
    require!(
        player_status.active_room == ranked_room.challenge_hash,
        GuessrError::RoomMismatch
    );
    require!(
        action_kind == ACTION_HINT_OPEN
            || action_kind == ACTION_MARK_MOVE
            || action_kind == ACTION_GUESS_SUBMIT,
        GuessrError::InvalidActionKind
    );
    require!(
        accuracy_bps <= MAX_ACCURACY_BPS,
        GuessrError::InvalidAccuracy
    );
    require!(config.reward_multiplier > 0, GuessrError::InvalidMultiplier);
    require!(
        config.penalty_divisor > 0,
        GuessrError::InvalidPenaltyDivisor
    );

    let penalty_base = config
        .penalty_threshold
        .checked_div(config.penalty_divisor.max(1))
        .ok_or(GuessrError::Overflow)?
        .max(1);
    let distance_penalty = (distance_km as u64)
        .checked_div(config.penalty_divisor.max(1))
        .ok_or(GuessrError::Overflow)?;

    let (mut reward_amount, mut penalty_amount): (u64, u64) = match action_kind {
        ACTION_HINT_OPEN => {
            ranked_room.hints_opened = ranked_room
                .hints_opened
                .checked_add(1)
                .ok_or(GuessrError::Overflow)?;
            (0, penalty_base)
        }
        ACTION_MARK_MOVE => (
            (accuracy_bps as u64)
                .checked_div(config.reward_multiplier)
                .ok_or(GuessrError::Overflow)?,
            distance_penalty,
        ),
        ACTION_GUESS_SUBMIT => {
            let mut reward_amount = (accuracy_bps as u64)
                .checked_mul(2)
                .ok_or(GuessrError::Overflow)?
                .checked_div(config.reward_multiplier)
                .ok_or(GuessrError::Overflow)?;
            let mut penalty_amount = distance_penalty;
            if is_correct_country {
                reward_amount = reward_amount
                    .checked_add(penalty_base)
                    .ok_or(GuessrError::Overflow)?;
            } else {
                penalty_amount = penalty_amount
                    .checked_add(penalty_base)
                    .ok_or(GuessrError::Overflow)?;
                reward_amount = 0;
            }
            (reward_amount, penalty_amount)
        }
        _ => return err!(GuessrError::InvalidActionKind),
    };

    let hint_decay = (ranked_room.hints_opened as u64)
        .checked_mul(penalty_base)
        .ok_or(GuessrError::Overflow)?;
    reward_amount = reward_amount.saturating_sub(hint_decay);

    if action_kind != ACTION_HINT_OPEN && !is_correct_country {
        reward_amount = 0;
        penalty_amount = penalty_amount
            .checked_add(penalty_base)
            .ok_or(GuessrError::Overflow)?;
    }

    if reward_amount > 0 {
        let signer_seeds: &[&[u8]] = &[b"mint-authority", &[config.mint_authority_bump]];
        let cpi_accounts = MintTo {
            mint: ctx.accounts.reward_mint.to_account_info(),
            to: ctx.accounts.player_token_account.to_account_info(),
            authority: ctx.accounts.mint_authority.to_account_info(),
        };
        token::mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                cpi_accounts,
                &[signer_seeds],
            ),
            reward_amount,
        )?;
    }

    let mut actual_penalty = 0_u64;
    if penalty_amount > 0 {
        let player_balance = ctx.accounts.player_token_account.amount;
        let transfer_amount = penalty_amount.min(player_balance);
        if transfer_amount > 0 {
            let cpi_accounts = Transfer {
                from: ctx.accounts.player_token_account.to_account_info(),
                to: ctx.accounts.treasury_token_account.to_account_info(),
                authority: ctx.accounts.player.to_account_info(),
            };
            token::transfer(
                CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts),
                transfer_amount,
            )?;
            actual_penalty = transfer_amount;
        }
    }

    ranked_room.score = total_score;
    ranked_room.total_earned = ranked_room
        .total_earned
        .checked_add(reward_amount)
        .ok_or(GuessrError::Overflow)?;
    ranked_room.total_lost = ranked_room
        .total_lost
        .checked_add(actual_penalty)
        .ok_or(GuessrError::Overflow)?;
    ranked_room.action_count = ranked_room
        .action_count
        .checked_add(1)
        .ok_or(GuessrError::Overflow)?;
    ranked_room.last_round_index = round_index;
    ranked_room.last_hp = hp_after;
    ranked_room.last_accuracy_bps = accuracy_bps;
    ranked_room.last_distance_km = distance_km;
    ranked_room.last_action_kind = action_kind;
    ranked_room.last_movement_hash = movement_hash;
    ranked_room.last_action_ts = now;
    player_status.last_heartbeat_ts = now;

    live_state.player = ctx.accounts.player.key();
    live_state.wallet_address = ctx.accounts.player.key();
    live_state.session_address = Pubkey::default();
    live_state.room_id = ranked_room.challenge_hash;
    live_state.round_index = round_index;
    live_state.hp = hp_after;
    live_state.total_score = total_score;
    live_state.earned_amount = ranked_room
        .total_earned
        .saturating_sub(ranked_room.total_lost);
    live_state.movement_hash = movement_hash;
    live_state.last_update_ts = now;
    live_state.bump = ctx.bumps.player_live_state;
    live_state.reserved = [0; 7];

    Ok(())
}

pub fn settle_ranked_room_handler(ctx: Context<SettleRankedRoom>, score: u64) -> Result<()> {
    let ranked_room = &mut ctx.accounts.ranked_room;
    let profile = &mut ctx.accounts.player_profile;
    let player = ctx.accounts.player.key();
    let now = Clock::get()?.unix_timestamp;

    require!(!ranked_room.is_settled, GuessrError::RoomAlreadySettled);

    ranked_room.score = score;
    ranked_room.is_settled = true;
    ranked_room.last_action_ts = now;

    let earned = ranked_room.total_earned;
    let lost = ranked_room.total_lost;

    let earning_delta = if earned >= lost {
        let delta: i64 = earned
            .checked_sub(lost)
            .ok_or(GuessrError::Underflow)?
            .try_into()
            .map_err(|_| GuessrError::Overflow)?;
        delta
    } else {
        let delta: i64 = lost
            .checked_sub(earned)
            .ok_or(GuessrError::Underflow)?
            .try_into()
            .map_err(|_| GuessrError::Overflow)?;
        delta.checked_neg().ok_or(GuessrError::Overflow)?
    };

    let xp_gained = 20_u64
        .checked_add(score.checked_div(20).ok_or(GuessrError::Overflow)?)
        .ok_or(GuessrError::Overflow)?
        .checked_add((ranked_room.action_count as u64).checked_div(2).ok_or(GuessrError::Overflow)?)
        .ok_or(GuessrError::Overflow)?;

    let outcome = if score > 0 && earning_delta >= 0 {
        ProfileOutcome::Win
    } else {
        ProfileOutcome::Loss
    };

    apply_profile_match_result(
        profile,
        player,
        ProfileMatchMode::RankedSolo,
        outcome,
        xp_gained,
        earning_delta,
        score,
        ctx.bumps.player_profile,
        now,
    )?;

    Ok(())
}

pub fn close_ranked_room_handler(ctx: Context<CloseRankedRoom>) -> Result<()> {
    require!(
        ctx.accounts.ranked_room.is_settled,
        GuessrError::RoomNotSettled
    );
    if ctx.accounts.player_status.active_room == ctx.accounts.ranked_room.challenge_hash {
        ctx.accounts.player_status.active_room = [0u8; 32];
    }
    Ok(())
}

#[derive(Accounts)]
#[instruction(reward_mint: Pubkey)]
pub struct SetRewardMint<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [b"ranked-config"],
        bump = ranked_config.bump,
        constraint = ranked_config.authority == authority.key() @ GuessrError::Unauthorized,
    )]
    pub ranked_config: Account<'info, RankedConfig>,
    #[account(
        constraint = treasury_token_account.mint == reward_mint @ GuessrError::InvalidTreasuryMint,
    )]
    pub treasury_token_account: Account<'info, TokenAccount>,
}

#[derive(Accounts)]
#[instruction(challenge_hash: [u8; 32])]
pub struct OpenRankedRoom<'info> {
    #[account(mut)]
    pub player: Signer<'info>,
    #[account(
        init,
        payer = player,
        space = RANKED_ROOM_SPACE,
        seeds = [b"ranked-room", player.key().as_ref(), challenge_hash.as_ref()],
        bump
    )]
    pub ranked_room: Account<'info, RankedRoom>,
    #[account(
        mut,
        seeds = [b"player-status", player.key().as_ref()],
        bump = player_status.bump,
        constraint = player_status.player == player.key() @ GuessrError::Unauthorized
    )]
    pub player_status: Account<'info, PlayerStatus>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateRankedState<'info> {
    #[account(mut)]
    pub player: Signer<'info>,
    #[account(
        mut,
        seeds = [b"player-status", player.key().as_ref()],
        bump = player_status.bump,
        constraint = player_status.player == player.key() @ GuessrError::Unauthorized
    )]
    pub player_status: Account<'info, PlayerStatus>,
    #[account(
        init_if_needed,
        payer = player,
        space = PLAYER_LIVE_STATE_SPACE,
        seeds = [b"player-live-state", player.key().as_ref()],
        bump
    )]
    pub player_live_state: Account<'info, PlayerLiveState>,
    #[account(
        mut,
        seeds = [b"ranked-room", player.key().as_ref(), ranked_room.challenge_hash.as_ref()],
        bump = ranked_room.bump,
        constraint = ranked_room.player == player.key() @ GuessrError::Unauthorized,
    )]
    pub ranked_room: Account<'info, RankedRoom>,
    #[account(
        seeds = [b"ranked-config"],
        bump = ranked_config.bump,
    )]
    pub ranked_config: Account<'info, RankedConfig>,
    #[account(mut, address = ranked_config.reward_mint)]
    pub reward_mint: Account<'info, Mint>,
    /// CHECK: PDA signer for mint authority.
    #[account(
        seeds = [b"mint-authority"],
        bump = ranked_config.mint_authority_bump,
    )]
    pub mint_authority: UncheckedAccount<'info>,
    #[account(
        mut,
        constraint = player_token_account.owner == player.key() @ GuessrError::InvalidTokenOwner,
        constraint = player_token_account.mint == reward_mint.key() @ GuessrError::InvalidTokenMint,
    )]
    pub player_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        address = ranked_config.treasury_token_account,
        constraint = treasury_token_account.mint == reward_mint.key() @ GuessrError::InvalidTreasuryMint,
    )]
    pub treasury_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SettleRankedRoom<'info> {
    #[account(mut)]
    pub player: Signer<'info>,
    #[account(
        mut,
        seeds = [b"ranked-room", player.key().as_ref(), ranked_room.challenge_hash.as_ref()],
        bump = ranked_room.bump,
        constraint = ranked_room.player == player.key() @ GuessrError::Unauthorized,
    )]
    pub ranked_room: Account<'info, RankedRoom>,
    #[account(
        seeds = [b"ranked-config"],
        bump = ranked_config.bump,
    )]
    pub ranked_config: Account<'info, RankedConfig>,
    #[account(mut, address = ranked_config.reward_mint)]
    pub reward_mint: Account<'info, Mint>,
    /// CHECK: PDA signer for mint authority.
    #[account(
        seeds = [b"mint-authority"],
        bump = ranked_config.mint_authority_bump,
    )]
    pub mint_authority: UncheckedAccount<'info>,
    #[account(
        mut,
        constraint = player_token_account.owner == player.key() @ GuessrError::InvalidTokenOwner,
        constraint = player_token_account.mint == reward_mint.key() @ GuessrError::InvalidTokenMint,
    )]
    pub player_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        address = ranked_config.treasury_token_account,
        constraint = treasury_token_account.mint == reward_mint.key() @ GuessrError::InvalidTreasuryMint,
    )]
    pub treasury_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
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
pub struct CloseRankedRoom<'info> {
    #[account(mut)]
    pub player: Signer<'info>,
    #[account(
        mut,
        seeds = [b"player-status", player.key().as_ref()],
        bump = player_status.bump,
        constraint = player_status.player == player.key() @ GuessrError::Unauthorized
    )]
    pub player_status: Account<'info, PlayerStatus>,
    #[account(
        mut,
        close = player,
        seeds = [b"ranked-room", player.key().as_ref(), ranked_room.challenge_hash.as_ref()],
        bump = ranked_room.bump,
        constraint = ranked_room.player == player.key() @ GuessrError::Unauthorized,
    )]
    pub ranked_room: Account<'info, RankedRoom>,
}
