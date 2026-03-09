use anchor_lang::prelude::*;
use ephemeral_rollups_sdk::anchor::{commit, delegate};
use ephemeral_rollups_sdk::cpi::DelegateConfig;
use ephemeral_rollups_sdk::ephem::commit_accounts;
use std::collections::HashSet;

use crate::constants::{MATCH_MODE_DUEL, MATCH_MODE_RANKED_SOLO};
use crate::error::GuessrError;
use crate::state::{LeaderboardState, LobbyState, RankedConfig};

pub const DELEGATE_TARGET_LOBBY_STATE: u8 = 0;
pub const DELEGATE_TARGET_RANKED_CONFIG: u8 = 1;
pub const DELEGATE_TARGET_PLAYER_STATUS: u8 = 2;
pub const DELEGATE_TARGET_PLAYER_LIVE_STATE: u8 = 3;
pub const DELEGATE_TARGET_PLAYER_PROFILE: u8 = 4;
pub const DELEGATE_TARGET_DUEL_ROOM: u8 = 5;
pub const DELEGATE_TARGET_RANKED_ROOM: u8 = 6;
pub const DELEGATE_TARGET_REWARD_CLAIM: u8 = 7;
pub const DELEGATE_TARGET_LEADERBOARD: u8 = 8;
pub const DELEGATE_TARGET_PLAYER_REWARDS: u8 = 9;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, Default)]
pub struct DelegateGuessrStateArgs {
    /// Delegation target selector (use DELEGATE_TARGET_* constants).
    pub target: u8,
    /// Player wallet for player-scoped PDAs.
    pub player: Pubkey,
    /// room_id / challenge_hash / match_id bytes for room and claim PDAs.
    pub room_or_match_id: [u8; 32],
    /// Match mode, used only for reward-claim PDA seeds.
    pub mode: u8,
}

pub fn delegate_guessr_state_handler(
    ctx: Context<DelegateGuessrState>,
    args: DelegateGuessrStateArgs,
) -> Result<()> {
    match args.target {
        DELEGATE_TARGET_LOBBY_STATE => ctx.accounts.delegate_pda(
            &ctx.accounts.payer,
            &[b"lobby-state"],
            DelegateConfig::default(),
        )?,
        DELEGATE_TARGET_RANKED_CONFIG => ctx.accounts.delegate_pda(
            &ctx.accounts.payer,
            &[b"ranked-config"],
            DelegateConfig::default(),
        )?,
        DELEGATE_TARGET_PLAYER_STATUS => ctx.accounts.delegate_pda(
            &ctx.accounts.payer,
            &[b"player-status", args.player.as_ref()],
            DelegateConfig::default(),
        )?,
        DELEGATE_TARGET_PLAYER_LIVE_STATE => ctx.accounts.delegate_pda(
            &ctx.accounts.payer,
            &[b"player-live-state", args.player.as_ref()],
            DelegateConfig::default(),
        )?,
        DELEGATE_TARGET_PLAYER_PROFILE => ctx.accounts.delegate_pda(
            &ctx.accounts.payer,
            &[b"player-profile", args.player.as_ref()],
            DelegateConfig::default(),
        )?,
        DELEGATE_TARGET_PLAYER_REWARDS => ctx.accounts.delegate_pda(
            &ctx.accounts.payer,
            &[b"player-rewards", args.player.as_ref()],
            DelegateConfig::default(),
        )?,
        DELEGATE_TARGET_DUEL_ROOM => ctx.accounts.delegate_pda(
            &ctx.accounts.payer,
            &[b"duel-room", args.room_or_match_id.as_ref()],
            DelegateConfig::default(),
        )?,
        DELEGATE_TARGET_RANKED_ROOM => ctx.accounts.delegate_pda(
            &ctx.accounts.payer,
            &[
                b"ranked-room",
                args.player.as_ref(),
                args.room_or_match_id.as_ref(),
            ],
            DelegateConfig::default(),
        )?,
        DELEGATE_TARGET_REWARD_CLAIM => {
            require!(
                args.mode == MATCH_MODE_DUEL || args.mode == MATCH_MODE_RANKED_SOLO,
                GuessrError::InvalidMatchMode
            );
            let mode_seed = [args.mode];
            ctx.accounts.delegate_pda(
                &ctx.accounts.payer,
                &[
                    b"reward-claim",
                    args.player.as_ref(),
                    args.room_or_match_id.as_ref(),
                    &mode_seed,
                ],
                DelegateConfig::default(),
            )?
        }
        DELEGATE_TARGET_LEADERBOARD => ctx.accounts.delegate_pda(
            &ctx.accounts.payer,
            &[b"leaderboard"],
            DelegateConfig::default(),
        )?,
        _ => return err!(GuessrError::InvalidDelegationTarget),
    }

    Ok(())
}

pub fn commit_guessr_state_handler<'info>(
    ctx: Context<'_, '_, '_, 'info, CommitGuessrState<'info>>,
) -> Result<()> {
    // Commit global Guessr PDAs + any additional PDA accounts passed in remaining_accounts.
    // This allows batching many commits with one instruction when transaction size permits.
    let lobby_state_info = ctx.accounts.lobby_state.to_account_info();
    let ranked_config_info = ctx.accounts.ranked_config.to_account_info();
    let leaderboard_info = ctx.accounts.leaderboard.to_account_info();

    let mut commit_targets = vec![&lobby_state_info, &ranked_config_info, &leaderboard_info];
    let mut seen = HashSet::from([
        *lobby_state_info.key,
        *ranked_config_info.key,
        *leaderboard_info.key,
    ]);

    for account in ctx.remaining_accounts.iter() {
        if !account.is_writable {
            continue;
        }
        if seen.insert(*account.key) {
            commit_targets.push(account);
        }
    }

    commit_accounts(
        &ctx.accounts.payer,
        commit_targets,
        &ctx.accounts.magic_context,
        &ctx.accounts.magic_program,
    )?;
    Ok(())
}

#[delegate]
#[derive(Accounts)]
pub struct DelegateGuessrState<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: PDA to delegate (admin decides which one by passing the PDA + seeds)
    #[account(mut, del)]
    pub pda: AccountInfo<'info>,
}

#[commit]
#[derive(Accounts)]
pub struct CommitGuessrState<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        mut,
        seeds = [b"lobby-state"],
        bump = lobby_state.bump,
    )]
    pub lobby_state: Account<'info, LobbyState>,
    #[account(
        mut,
        seeds = [b"ranked-config"],
        bump = ranked_config.bump,
    )]
    pub ranked_config: Account<'info, RankedConfig>,
    #[account(
        mut,
        seeds = [b"leaderboard"],
        bump = leaderboard.bump,
    )]
    pub leaderboard: Account<'info, LeaderboardState>,
}
