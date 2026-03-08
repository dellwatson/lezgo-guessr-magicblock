use anchor_lang::prelude::*;

use crate::constants::DUEL_ROOM_SPACE;
use crate::error::GuessrError;
use crate::state::{DuelRoom, PlayerStatus};

// match room
pub fn enter_room_handler(ctx: Context<EnterRoom>, room_id: [u8; 32]) -> Result<()> {
    let player_status = &mut ctx.accounts.player_status;
    let duel_room = &mut ctx.accounts.duel_room;
    let player = ctx.accounts.player.key();
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

pub fn clear_room_handler(ctx: Context<ClearRoom>) -> Result<()> {
    let player_status = &mut ctx.accounts.player_status;
    player_status.active_room = [0u8; 32];
    Ok(())
}

#[derive(Accounts)]
#[instruction(room_id: [u8; 32])]
pub struct EnterRoom<'info> {
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
        space = DUEL_ROOM_SPACE,
        seeds = [b"duel-room", room_id.as_ref()],
        bump
    )]
    pub duel_room: Account<'info, DuelRoom>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ClearRoom<'info> {
    #[account(mut)]
    pub player: Signer<'info>,
    #[account(
        mut,
        seeds = [b"player-status", player.key().as_ref()],
        bump = player_status.bump,
        constraint = player_status.player == player.key() @ GuessrError::Unauthorized
    )]
    pub player_status: Account<'info, PlayerStatus>,
}
