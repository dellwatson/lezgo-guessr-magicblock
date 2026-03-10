use anchor_lang::prelude::*;

use crate::constants::ROOM_STATUS_CLEARED;
use crate::error::GuessrError;
use crate::instructions::{ensure_player_authority, ensure_wallet_matches_status};
use crate::state::{PlayerStatus, RoomPool, RoomPoolEntry};

// match room
pub fn enter_room_handler(
    ctx: Context<EnterRoom>,
    wallet_address: Pubkey,
    room_id: [u8; 32],
    match_mode: u8,
    slot_filled: u8,
    slot_total: u8,
    status: u8,
    player_a: Pubkey,
    player_b: Pubkey,
) -> Result<()> {
    let player_status = &mut ctx.accounts.player_status;
    let room_pool = &mut ctx.accounts.room_pool;
    ensure_wallet_matches_status(player_status, wallet_address)?;
    ensure_player_authority(player_status, ctx.accounts.authority.key())?;
    let now = Clock::get()?.unix_timestamp;

    require!(player_status.is_online, GuessrError::PlayerOffline);

    player_status.active_room = room_id;
    player_status.last_heartbeat_ts = now;

    room_pool.push_entry(RoomPoolEntry {
        room_id,
        wallet: player_status.player,
        session: ctx.accounts.authority.key(),
        status,
        slot_filled,
        slot_total,
        match_mode,
        players: [player_a, player_b],
        last_update_ts: now,
    });

    Ok(())
}

pub fn clear_room_handler(ctx: Context<ClearRoom>, wallet_address: Pubkey) -> Result<()> {
    let player_status = &mut ctx.accounts.player_status;
    let room_pool = &mut ctx.accounts.room_pool;
    ensure_wallet_matches_status(player_status, wallet_address)?;
    ensure_player_authority(player_status, ctx.accounts.authority.key())?;
    let now = Clock::get()?.unix_timestamp;
    let room_id = player_status.active_room;
    player_status.active_room = [0u8; 32];

    room_pool.push_entry(RoomPoolEntry {
        room_id,
        wallet: player_status.player,
        session: ctx.accounts.authority.key(),
        status: ROOM_STATUS_CLEARED,
        slot_filled: 0,
        slot_total: 0,
        match_mode: 0,
        players: [player_status.player, Pubkey::default()],
        last_update_ts: now,
    });

    Ok(())
}

#[derive(Accounts)]
#[instruction(
    wallet_address: Pubkey,
    room_id: [u8; 32],
    match_mode: u8,
    slot_filled: u8,
    slot_total: u8,
    status: u8,
    player_a: Pubkey,
    player_b: Pubkey
)]
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
        mut,
        seeds = [b"room-pool"],
        bump = room_pool.bump
    )]
    pub room_pool: Account<'info, RoomPool>,
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
    #[account(
        mut,
        seeds = [b"room-pool"],
        bump = room_pool.bump
    )]
    pub room_pool: Account<'info, RoomPool>,
}
