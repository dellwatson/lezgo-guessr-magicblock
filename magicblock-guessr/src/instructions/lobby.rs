use anchor_lang::prelude::*;

use crate::constants::PLAYER_STATUS_SPACE;
use crate::error::GuessrError;
use crate::state::{LobbyState, PlayerStatus};

pub fn join_lobby_handler(ctx: Context<JoinLobby>) -> Result<()> {
    let lobby = &mut ctx.accounts.lobby_state;
    let player_status = &mut ctx.accounts.player_status;
    let now = Clock::get()?.unix_timestamp;

    let was_active = player_status.is_online
        && now
            .checked_sub(player_status.last_heartbeat_ts)
            .ok_or(GuessrError::Overflow)?
            <= lobby.heartbeat_ttl_sec;

    if player_status.is_online && !was_active {
        lobby.online_players = lobby
            .online_players
            .checked_sub(1)
            .ok_or(GuessrError::Underflow)?;
    }

    if !was_active {
        lobby.online_players = lobby
            .online_players
            .checked_add(1)
            .ok_or(GuessrError::Overflow)?;
    }

    player_status.player = ctx.accounts.player.key();
    player_status.active_room = [0u8; 32];
    player_status.last_heartbeat_ts = now;
    player_status.is_online = true;
    player_status.bump = ctx.bumps.player_status;
    player_status.reserved = [0; 6];

    Ok(())
}

pub fn heartbeat_handler(ctx: Context<Heartbeat>) -> Result<()> {
    let lobby = &mut ctx.accounts.lobby_state;
    let player_status = &mut ctx.accounts.player_status;

    require!(player_status.is_online, GuessrError::PlayerOffline);
    let now = Clock::get()?.unix_timestamp;
    let elapsed = now
        .checked_sub(player_status.last_heartbeat_ts)
        .ok_or(GuessrError::Overflow)?;

    if elapsed > lobby.heartbeat_ttl_sec {
        lobby.online_players = lobby
            .online_players
            .checked_sub(1)
            .ok_or(GuessrError::Underflow)?;
        player_status.is_online = false;
        player_status.active_room = [0u8; 32];
        return err!(GuessrError::HeartbeatExpired);
    }

    player_status.last_heartbeat_ts = now;

    Ok(())
}

pub fn leave_lobby_handler(ctx: Context<LeaveLobby>) -> Result<()> {
    let lobby = &mut ctx.accounts.lobby_state;
    let player_status = &mut ctx.accounts.player_status;

    if player_status.is_online {
        lobby.online_players = lobby
            .online_players
            .checked_sub(1)
            .ok_or(GuessrError::Underflow)?;
    }

    player_status.is_online = false;
    player_status.active_room = [0u8; 32];

    Ok(())
}

pub fn prune_stale_player_handler(ctx: Context<PruneStalePlayer>) -> Result<()> {
    let lobby = &mut ctx.accounts.lobby_state;
    let player_status = &mut ctx.accounts.player_status;
    let now = Clock::get()?.unix_timestamp;

    if !player_status.is_online {
        return Ok(());
    }

    let elapsed = now
        .checked_sub(player_status.last_heartbeat_ts)
        .ok_or(GuessrError::Overflow)?;

    if elapsed > lobby.heartbeat_ttl_sec {
        lobby.online_players = lobby
            .online_players
            .checked_sub(1)
            .ok_or(GuessrError::Underflow)?;
        player_status.is_online = false;
        player_status.active_room = [0u8; 32];
    }

    Ok(())
}

#[derive(Accounts)]
pub struct JoinLobby<'info> {
    #[account(mut)]
    pub player: Signer<'info>,
    #[account(
        mut,
        seeds = [b"lobby-state"],
        bump = lobby_state.bump
    )]
    pub lobby_state: Account<'info, LobbyState>,
    #[account(
        init_if_needed,
        payer = player,
        space = PLAYER_STATUS_SPACE,
        seeds = [b"player-status", player.key().as_ref()],
        bump
    )]
    pub player_status: Account<'info, PlayerStatus>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Heartbeat<'info> {
    #[account(mut)]
    pub player: Signer<'info>,
    #[account(
        mut,
        seeds = [b"lobby-state"],
        bump = lobby_state.bump
    )]
    pub lobby_state: Account<'info, LobbyState>,
    #[account(
        mut,
        seeds = [b"player-status", player.key().as_ref()],
        bump = player_status.bump,
        constraint = player_status.player == player.key() @ GuessrError::Unauthorized
    )]
    pub player_status: Account<'info, PlayerStatus>,
}

#[derive(Accounts)]
pub struct PruneStalePlayer<'info> {
    pub caller: Signer<'info>,
    #[account(
        mut,
        seeds = [b"lobby-state"],
        bump = lobby_state.bump
    )]
    pub lobby_state: Account<'info, LobbyState>,
    #[account(mut)]
    pub player_status: Account<'info, PlayerStatus>,
}

#[derive(Accounts)]
pub struct LeaveLobby<'info> {
    #[account(mut)]
    pub player: Signer<'info>,
    #[account(
        mut,
        seeds = [b"lobby-state"],
        bump = lobby_state.bump
    )]
    pub lobby_state: Account<'info, LobbyState>,
    #[account(
        mut,
        seeds = [b"player-status", player.key().as_ref()],
        bump = player_status.bump,
        constraint = player_status.player == player.key() @ GuessrError::Unauthorized
    )]
    pub player_status: Account<'info, PlayerStatus>,
}
