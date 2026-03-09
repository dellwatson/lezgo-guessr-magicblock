use anchor_lang::prelude::*;
use anchor_lang::InstructionData;
use anchor_spl::associated_token::get_associated_token_address;
use anchor_spl::token::ID as TOKEN_PROGRAM_ID;
use ephemeral_rollups_sdk::anchor::{commit, delegate};
use ephemeral_rollups_sdk::cpi::DelegateConfig;
use ephemeral_rollups_sdk::ephem::{
    commit_accounts, CallHandler, CommitType, MagicAction, MagicInstructionBuilder,
};
use ephemeral_rollups_sdk::{ActionArgs, ShortAccountMeta};
use std::collections::HashSet;

use crate::constants::{MATCH_MODE_DUEL, MATCH_MODE_RANKED_SOLO};
use crate::error::GuessrError;
use crate::state::{
    DuelRoom, LeaderboardState, LobbyState, PlayerProfile, PlayerRewardStats, PlayerStatus,
    RankedConfig, RankedRoom,
};

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
    let validator = ctx.remaining_accounts.first().map(|acc| *acc.key);
    let config = DelegateConfig {
        validator,
        ..Default::default()
    };
    match args.target {
        DELEGATE_TARGET_LOBBY_STATE => ctx.accounts.delegate_pda(
            &ctx.accounts.payer,
            &[b"lobby-state"],
            config,
        )?,
        DELEGATE_TARGET_RANKED_CONFIG => ctx.accounts.delegate_pda(
            &ctx.accounts.payer,
            &[b"ranked-config"],
            config,
        )?,
        DELEGATE_TARGET_PLAYER_STATUS => ctx.accounts.delegate_pda(
            &ctx.accounts.payer,
            &[b"player-status", args.player.as_ref()],
            config,
        )?,
        DELEGATE_TARGET_PLAYER_LIVE_STATE => ctx.accounts.delegate_pda(
            &ctx.accounts.payer,
            &[b"player-live-state", args.player.as_ref()],
            config,
        )?,
        DELEGATE_TARGET_PLAYER_PROFILE => ctx.accounts.delegate_pda(
            &ctx.accounts.payer,
            &[b"player-profile", args.player.as_ref()],
            config,
        )?,
        DELEGATE_TARGET_PLAYER_REWARDS => ctx.accounts.delegate_pda(
            &ctx.accounts.payer,
            &[b"player-rewards", args.player.as_ref()],
            config,
        )?,
        DELEGATE_TARGET_DUEL_ROOM => ctx.accounts.delegate_pda(
            &ctx.accounts.payer,
            &[b"duel-room", args.room_or_match_id.as_ref()],
            config,
        )?,
        DELEGATE_TARGET_RANKED_ROOM => ctx.accounts.delegate_pda(
            &ctx.accounts.payer,
            &[
                b"ranked-room",
                args.player.as_ref(),
                args.room_or_match_id.as_ref(),
            ],
            config,
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
                config,
            )?
        }
        DELEGATE_TARGET_LEADERBOARD => ctx.accounts.delegate_pda(
            &ctx.accounts.payer,
            &[b"leaderboard"],
            config,
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

pub fn commit_ranked_with_reward_handler(
    ctx: Context<CommitRankedWithReward>,
) -> Result<()> {
    let reward_mint = ctx.accounts.ranked_config.reward_mint;
    let player = ctx.accounts.ranked_room.player;
    let player_token_account = get_associated_token_address(&player, &reward_mint);
    let (mint_authority, _) =
        Pubkey::find_program_address(&[b"mint-authority"], ctx.program_id);

    let instruction_data =
        InstructionData::data(&crate::instruction::MintRankedReward {});
    let action_args = ActionArgs::new(instruction_data);
    let action_accounts = vec![
        ShortAccountMeta {
            pubkey: ctx.accounts.ranked_config.key(),
            is_writable: false,
        },
        ShortAccountMeta {
            pubkey: ctx.accounts.ranked_room.key(),
            is_writable: false,
        },
        ShortAccountMeta {
            pubkey: reward_mint,
            is_writable: true,
        },
        ShortAccountMeta {
            pubkey: mint_authority,
            is_writable: false,
        },
        ShortAccountMeta {
            pubkey: player_token_account,
            is_writable: true,
        },
        ShortAccountMeta {
            pubkey: TOKEN_PROGRAM_ID,
            is_writable: false,
        },
    ];
    let action = CallHandler {
        destination_program: crate::ID,
        accounts: action_accounts,
        args: action_args,
        escrow_authority: ctx.accounts.payer.to_account_info(),
        compute_units: 200_000,
    };

    let mut commit_targets = Vec::new();
    let mut seen = HashSet::new();
    for account in [
        ctx.accounts.lobby_state.to_account_info(),
        ctx.accounts.ranked_config.to_account_info(),
        ctx.accounts.leaderboard.to_account_info(),
        ctx.accounts.player_status.to_account_info(),
        ctx.accounts.player_profile.to_account_info(),
        ctx.accounts.player_rewards.to_account_info(),
        ctx.accounts.ranked_room.to_account_info(),
    ] {
        if seen.insert(*account.key) {
            commit_targets.push(account);
        }
    }

    let magic_action = MagicInstructionBuilder {
        payer: ctx.accounts.payer.to_account_info(),
        magic_context: ctx.accounts.magic_context.to_account_info(),
        magic_program: ctx.accounts.magic_program.to_account_info(),
        magic_action: MagicAction::Commit(CommitType::WithHandler {
            commited_accounts: commit_targets,
            call_handlers: vec![action],
        }),
    };

    magic_action.build_and_invoke()?;
    Ok(())
}

pub fn commit_duel_with_rewards_handler(
    ctx: Context<CommitDuelWithRewards>,
) -> Result<()> {
    let reward_mint = ctx.accounts.ranked_config.reward_mint;
    let host = ctx.accounts.duel_room.host;
    let challenger = ctx.accounts.duel_room.challenger;
    let host_token_account = get_associated_token_address(&host, &reward_mint);
    let challenger_token_account = get_associated_token_address(&challenger, &reward_mint);
    let (mint_authority, _) =
        Pubkey::find_program_address(&[b"mint-authority"], ctx.program_id);

    let instruction_data =
        InstructionData::data(&crate::instruction::MintDuelRewards {});
    let action_args = ActionArgs::new(instruction_data);
    let action_accounts = vec![
        ShortAccountMeta {
            pubkey: ctx.accounts.ranked_config.key(),
            is_writable: false,
        },
        ShortAccountMeta {
            pubkey: ctx.accounts.duel_room.key(),
            is_writable: false,
        },
        ShortAccountMeta {
            pubkey: reward_mint,
            is_writable: true,
        },
        ShortAccountMeta {
            pubkey: mint_authority,
            is_writable: false,
        },
        ShortAccountMeta {
            pubkey: host_token_account,
            is_writable: true,
        },
        ShortAccountMeta {
            pubkey: challenger_token_account,
            is_writable: true,
        },
        ShortAccountMeta {
            pubkey: TOKEN_PROGRAM_ID,
            is_writable: false,
        },
    ];
    let action = CallHandler {
        destination_program: crate::ID,
        accounts: action_accounts,
        args: action_args,
        escrow_authority: ctx.accounts.payer.to_account_info(),
        compute_units: 200_000,
    };

    let mut commit_targets = Vec::new();
    let mut seen = HashSet::new();
    for account in [
        ctx.accounts.lobby_state.to_account_info(),
        ctx.accounts.ranked_config.to_account_info(),
        ctx.accounts.leaderboard.to_account_info(),
        ctx.accounts.player_status.to_account_info(),
        ctx.accounts.duel_room.to_account_info(),
        ctx.accounts.host_profile.to_account_info(),
        ctx.accounts.challenger_profile.to_account_info(),
        ctx.accounts.host_rewards.to_account_info(),
        ctx.accounts.challenger_rewards.to_account_info(),
    ] {
        if seen.insert(*account.key) {
            commit_targets.push(account);
        }
    }

    let magic_action = MagicInstructionBuilder {
        payer: ctx.accounts.payer.to_account_info(),
        magic_context: ctx.accounts.magic_context.to_account_info(),
        magic_program: ctx.accounts.magic_program.to_account_info(),
        magic_action: MagicAction::Commit(CommitType::WithHandler {
            commited_accounts: commit_targets,
            call_handlers: vec![action],
        }),
    };

    magic_action.build_and_invoke()?;
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
    pub leaderboard: Box<Account<'info, LeaderboardState>>,
}

#[commit]
#[derive(Accounts)]
pub struct CommitRankedWithReward<'info> {
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
    pub leaderboard: Box<Account<'info, LeaderboardState>>,
    #[account(mut)]
    pub player_status: Account<'info, PlayerStatus>,
    #[account(mut)]
    pub player_profile: Account<'info, PlayerProfile>,
    #[account(mut)]
    pub player_rewards: Account<'info, PlayerRewardStats>,
    #[account(mut)]
    pub ranked_room: Account<'info, RankedRoom>,
}

#[commit]
#[derive(Accounts)]
pub struct CommitDuelWithRewards<'info> {
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
    pub leaderboard: Box<Account<'info, LeaderboardState>>,
    #[account(mut)]
    pub player_status: Account<'info, PlayerStatus>,
    #[account(mut)]
    pub duel_room: Account<'info, DuelRoom>,
    #[account(mut)]
    pub host_profile: Account<'info, PlayerProfile>,
    #[account(mut)]
    pub challenger_profile: Account<'info, PlayerProfile>,
    #[account(mut)]
    pub host_rewards: Account<'info, PlayerRewardStats>,
    #[account(mut)]
    pub challenger_rewards: Account<'info, PlayerRewardStats>,
}
