use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, MintTo, Token, TokenAccount, Transfer};

use crate::constants::PLAYER_LIVE_STATE_SPACE;
use crate::error::GuessrError;
use crate::instructions::{ensure_player_authority, ensure_wallet_matches_status};
use crate::state::{DuelRoom, PlayerLiveState, PlayerStatus, RankedConfig};

pub fn update_player_state_handler(
    ctx: Context<UpdatePlayerState>,
    wallet_address: Pubkey,
    session_address: Pubkey,
    room_id: [u8; 32],
    round_index: u16,
    hp: u16,
    total_score: u64,
    earned_amount: u64,
    movement_hash: [u8; 32],
) -> Result<()> {
    let player_status = &mut ctx.accounts.player_status;
    let live_state = &mut ctx.accounts.player_live_state;
    let now = Clock::get()?.unix_timestamp;
    let authority = ctx.accounts.authority.key();

    ensure_wallet_matches_status(player_status, wallet_address)?;
    ensure_player_authority(player_status, authority)?;

    require!(player_status.is_online, GuessrError::PlayerOffline);
    require!(
        player_status.active_room == room_id,
        GuessrError::RoomMismatch
    );

    if session_address != Pubkey::default() {
        if authority == wallet_address {
            player_status.session_address = session_address;
        } else {
            require!(session_address == authority, GuessrError::Unauthorized);
        }
    }

    player_status.last_heartbeat_ts = now;

    live_state.player = player_status.player;
    live_state.wallet_address = player_status.player;
    live_state.session_address = authority;
    live_state.room_id = room_id;
    live_state.round_index = round_index;
    live_state.hp = hp;
    live_state.total_score = total_score;
    live_state.earned_amount = earned_amount;
    live_state.movement_hash = movement_hash;
    live_state.last_update_ts = now;
    live_state.bump = ctx.bumps.player_live_state;
    live_state.reserved = [0; 7];
    // TIME ?

    Ok(())
}

pub fn update_duel_state_handler(
    ctx: Context<UpdateDuelState>,
    wallet_address: Pubkey,
    session_address: Pubkey,
    room_id: [u8; 32], // room_id is a string and unique too
    round_index: u16,
    hp: u16,
    total_score: u64,
    earning_delta: i64, // duel might not use this yet for now.
    movement_hash: [u8; 32], // is movement has -> coordinate ?
                        // need to know the owner of the state.
) -> Result<()> {
    let player_status = &mut ctx.accounts.player_status;
    let live_state = &mut ctx.accounts.player_live_state;
    let duel_room = &mut ctx.accounts.duel_room;
    let config = &ctx.accounts.ranked_config;
    let authority = ctx.accounts.authority.key();
    let player_key = player_status.player;
    let now = Clock::get()?.unix_timestamp;

    ensure_wallet_matches_status(player_status, wallet_address)?;
    ensure_player_authority(player_status, authority)?;

    require!(player_status.is_online, GuessrError::PlayerOffline);
    require!(
        player_status.active_room == room_id,
        GuessrError::RoomMismatch
    );
    require!(duel_room.room_id == room_id, GuessrError::RoomMismatch);
    require!(!duel_room.is_settled, GuessrError::DuelAlreadySettled);

    if session_address != Pubkey::default() {
        if authority == wallet_address {
            player_status.session_address = session_address;
        } else {
            require!(session_address == authority, GuessrError::Unauthorized);
        }
    }

    if duel_room.host != player_key && duel_room.challenger != player_key {
        if duel_room.challenger == Pubkey::default() {
            duel_room.challenger = player_key;
            duel_room.player_count = duel_room
                .player_count
                .checked_add(1)
                .ok_or(GuessrError::Overflow)?;
        } else {
            return err!(GuessrError::PlayerNotInRoom);
        }
    }

    let reward_amount = if earning_delta > 0 {
        earning_delta as u64
    } else {
        0_u64
    };
    let penalty_amount = if earning_delta < 0 {
        earning_delta
            .checked_neg()
            .ok_or(GuessrError::Overflow)?
            .try_into()
            .map_err(|_| GuessrError::Overflow)?
    } else {
        0_u64
    };

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
        // Session-key updates do not control the wallet-owned SPL account authority.
        // Penalty transfer only executes when the wallet itself is the signer.
        if authority == wallet_address {
            let player_balance = ctx.accounts.player_token_account.amount;
            let transfer_amount = penalty_amount.min(player_balance);
            if transfer_amount > 0 {
                let cpi_accounts = Transfer {
                    from: ctx.accounts.player_token_account.to_account_info(),
                    to: ctx.accounts.treasury_token_account.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info(),
                };
                token::transfer(
                    CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts),
                    transfer_amount,
                )?;
                actual_penalty = transfer_amount;
            }
        }
    }

    if duel_room.host == player_key {
        duel_room.host_score = total_score;
        duel_room.host_earned = duel_room
            .host_earned
            .checked_add(reward_amount)
            .ok_or(GuessrError::Overflow)?;
        duel_room.host_lost = duel_room
            .host_lost
            .checked_add(actual_penalty)
            .ok_or(GuessrError::Overflow)?;
    } else if duel_room.challenger == player_key {
        duel_room.challenger_score = total_score;
        duel_room.challenger_earned = duel_room
            .challenger_earned
            .checked_add(reward_amount)
            .ok_or(GuessrError::Overflow)?;
        duel_room.challenger_lost = duel_room
            .challenger_lost
            .checked_add(actual_penalty)
            .ok_or(GuessrError::Overflow)?;
    } else {
        return err!(GuessrError::PlayerNotInRoom);
    }

    duel_room.last_update_ts = now;
    player_status.last_heartbeat_ts = now;

    live_state.player = player_key;
    live_state.wallet_address = player_key;
    live_state.session_address = authority;
    live_state.room_id = room_id;
    live_state.round_index = round_index;
    live_state.hp = hp;
    live_state.total_score = total_score;
    live_state.earned_amount = reward_amount;
    live_state.movement_hash = movement_hash;
    live_state.last_update_ts = now;
    live_state.bump = ctx.bumps.player_live_state;
    live_state.reserved = [0; 7];

    Ok(())
}

#[derive(Accounts)]
#[instruction(wallet_address: Pubkey)]
pub struct UpdatePlayerState<'info> {
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
        init_if_needed,
        payer = authority,
        space = PLAYER_LIVE_STATE_SPACE,
        seeds = [b"player-live-state", wallet_address.as_ref()],
        bump
    )]
    pub player_live_state: Account<'info, PlayerLiveState>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(
    wallet_address: Pubkey,
    _session_address: Pubkey,
    room_id: [u8; 32]
)]
pub struct UpdateDuelState<'info> {
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
        init_if_needed,
        payer = authority,
        space = PLAYER_LIVE_STATE_SPACE,
        seeds = [b"player-live-state", wallet_address.as_ref()],
        bump
    )]
    pub player_live_state: Account<'info, PlayerLiveState>,
    #[account(
        mut,
        seeds = [b"duel-room", room_id.as_ref()],
        bump = duel_room.bump
    )]
    pub duel_room: Account<'info, DuelRoom>,
    #[account(
        seeds = [b"ranked-config"],
        bump = ranked_config.bump
    )]
    pub ranked_config: Account<'info, RankedConfig>,
    #[account(mut, address = ranked_config.reward_mint)]
    pub reward_mint: Account<'info, Mint>,
    /// CHECK: PDA signer for mint authority.
    #[account(
        seeds = [b"mint-authority"],
        bump = ranked_config.mint_authority_bump
    )]
    pub mint_authority: UncheckedAccount<'info>,
    #[account(
        mut,
        constraint = player_token_account.owner == wallet_address @ GuessrError::InvalidTokenOwner,
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
