use anchor_lang::prelude::*;

use crate::constants::DUEL_ROOM_SPACE;
use crate::error::GuessrError;
use crate::instructions::{ensure_player_authority, ensure_wallet_matches_status};
use crate::state::{DuelRoom, PlayerStatus};

// match room
pub fn enter_room_handler(
    ctx: Context<EnterRoom>,
    wallet_address: Pubkey,
    room_id: [u8; 32],
) -> Result<()> {
    let player_status = &mut ctx.accounts.player_status;
    let duel_room = &mut ctx.accounts.duel_room;
    ensure_wallet_matches_status(player_status, wallet_address)?;
    ensure_player_authority(player_status, ctx.accounts.authority.key())?;
    let player = player_status.player;
    let now = Clock::get()?.unix_timestamp;

    require!(player_status.is_online, GuessrError::PlayerOffline);

    if duel_room.room_id == [0u8; 32] {
        duel_room.room_id = room_id;
        duel_room.host = player;
        duel_room.challenger = Pubkey::default();
        duel_room.player_count = 1;
        duel_room.is_settled = false;
        duel_room.host_score = 0;
        duel_room.challenger_score = 0;
        duel_room.host_earned = 0;
        duel_room.host_lost = 0;
        duel_room.challenger_earned = 0;
        duel_room.challenger_lost = 0;
        duel_room.winner = Pubkey::default();
        duel_room.last_update_ts = now;
        duel_room.bump = ctx.bumps.duel_room;
        duel_room.reserved = [0; 14];
    } else {
        require!(duel_room.room_id == room_id, GuessrError::RoomMismatch);
        require!(!duel_room.is_settled, GuessrError::DuelAlreadySettled);

        if duel_room.host != player && duel_room.challenger != player {
            if duel_room.challenger == Pubkey::default() {
                duel_room.challenger = player;
                duel_room.player_count = duel_room
                    .player_count
                    .checked_add(1)
                    .ok_or(GuessrError::Overflow)?;
            } else {
                return err!(GuessrError::RoomFull);
            }
        }

        duel_room.last_update_ts = now;
    }

    player_status.active_room = room_id;
    player_status.last_heartbeat_ts = now;

    Ok(())
}

pub fn clear_room_handler(ctx: Context<ClearRoom>, wallet_address: Pubkey) -> Result<()> {
    let player_status = &mut ctx.accounts.player_status;
    ensure_wallet_matches_status(player_status, wallet_address)?;
    ensure_player_authority(player_status, ctx.accounts.authority.key())?;
    player_status.active_room = [0u8; 32];
    Ok(())
}

#[derive(Accounts)]
#[instruction(wallet_address: Pubkey, room_id: [u8; 32])]
pub struct EnterRoom<'info> {
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
        space = DUEL_ROOM_SPACE,
        seeds = [b"duel-room", room_id.as_ref()],
        bump
    )]
    pub duel_room: Account<'info, DuelRoom>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(wallet_address: Pubkey)]
pub struct ClearRoom<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [b"player-status", wallet_address.as_ref()],
        bump = player_status.bump,
        constraint = player_status.player == wallet_address @ GuessrError::Unauthorized
    )]
    pub player_status: Account<'info, PlayerStatus>,
}
