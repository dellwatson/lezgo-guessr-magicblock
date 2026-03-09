use anchor_lang::prelude::*;
use ephemeral_rollups_sdk::anchor::ephemeral;

pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use instructions::*;

declare_id!("Cs16ovMuy7gBSJGSDRECLEso4v56PCMENGKWMjdbvECv");

#[ephemeral]
#[program]
pub mod guessr_multiplayer_program {
    use super::*;

    pub fn initialize_system(
        ctx: Context<InitializeSystem>,
        heartbeat_ttl_sec: i64,
        reward_mint: Pubkey,
        reward_multiplier: u64,
        penalty_divisor: u64,
        penalty_threshold: u64,
    ) -> Result<()> {
        instructions::setup::initialize_system_handler(
            ctx,
            heartbeat_ttl_sec,
            reward_mint,
            reward_multiplier,
            penalty_divisor,
            penalty_threshold,
        )
    }

    pub fn join_lobby(
        ctx: Context<JoinLobby>,
        wallet_address: Pubkey,
        session_address: Pubkey,
    ) -> Result<()> {
        instructions::lobby::join_lobby_handler(ctx, wallet_address, session_address)
    }

    pub fn heartbeat(ctx: Context<Heartbeat>, wallet_address: Pubkey) -> Result<()> {
        instructions::lobby::heartbeat_handler(ctx, wallet_address)
    }

    pub fn leave_lobby(ctx: Context<LeaveLobby>, wallet_address: Pubkey) -> Result<()> {
        instructions::lobby::leave_lobby_handler(ctx, wallet_address)
    }

    pub fn enter_room(
        ctx: Context<EnterRoom>,
        wallet_address: Pubkey,
        room_id: [u8; 32],
    ) -> Result<()> {
        instructions::room::enter_room_handler(ctx, wallet_address, room_id)
    }

    pub fn clear_room(ctx: Context<ClearRoom>, wallet_address: Pubkey) -> Result<()> {
        instructions::room::clear_room_handler(ctx, wallet_address)
    }

    pub fn prune_stale_player(ctx: Context<PruneStalePlayer>) -> Result<()> {
        instructions::lobby::prune_stale_player_handler(ctx)
    }

    pub fn update_player_state(
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
        instructions::player_state::update_player_state_handler(
            ctx,
            wallet_address,
            session_address,
            room_id,
            round_index,
            hp,
            total_score,
            earned_amount,
            movement_hash,
        )
    }

    pub fn update_duel_state(
        ctx: Context<UpdateDuelState>,
        wallet_address: Pubkey,
        session_address: Pubkey,
        room_id: [u8; 32],
        round_index: u16,
        hp: u16,
        total_score: u64,
        earning_delta: i64,
        movement_hash: [u8; 32],
    ) -> Result<()> {
        instructions::player_state::update_duel_state_handler(
            ctx,
            wallet_address,
            session_address,
            room_id,
            round_index,
            hp,
            total_score,
            earning_delta,
            movement_hash,
        )
    }

    pub fn settle_duel_room(
        ctx: Context<SettleDuelRoom>,
        wallet_address: Pubkey,
        room_id: [u8; 32],
        winner: Pubkey,
        is_draw: bool,
        host_score: u64,
        challenger_score: u64,
    ) -> Result<()> {
        instructions::duel::settle_duel_room_handler(
            ctx,
            wallet_address,
            room_id,
            winner,
            is_draw,
            host_score,
            challenger_score,
        )
    }

    pub fn commit_match_result(
        ctx: Context<CommitMatchResult>,
        match_mode: u8,
        did_win: bool,
        xp_gained: u64,
        earning_delta: i64,
        final_score: u64,
    ) -> Result<()> {
        instructions::profile::commit_match_result_handler(
            ctx,
            match_mode,
            did_win,
            xp_gained,
            earning_delta,
            final_score,
        )
    }

    pub fn set_reward_mint(ctx: Context<SetRewardMint>, reward_mint: Pubkey) -> Result<()> {
        instructions::ranked::set_reward_mint_handler(ctx, reward_mint)
    }

    pub fn open_ranked_room(
        ctx: Context<OpenRankedRoom>,
        wallet_address: Pubkey,
        challenge_hash: [u8; 32],
    ) -> Result<()> {
        instructions::ranked::open_ranked_room_handler(ctx, wallet_address, challenge_hash)
    }

    pub fn update_ranked_state(
        ctx: Context<UpdateRankedState>,
        wallet_address: Pubkey,
        round_index: u16,
        hp_after: u16,
        distance_km: u32,
        accuracy_bps: u16,
        action_kind: u8,
        is_correct_country: bool,
        total_score: u64,
        movement_hash: [u8; 32],
    ) -> Result<()> {
        instructions::ranked::update_ranked_state_handler(
            ctx,
            wallet_address,
            round_index,
            hp_after,
            distance_km,
            accuracy_bps,
            action_kind,
            is_correct_country,
            total_score,
            movement_hash,
        )
    }

    pub fn settle_ranked_room(
        ctx: Context<SettleRankedRoom>,
        wallet_address: Pubkey,
        score: u64,
    ) -> Result<()> {
        instructions::ranked::settle_ranked_room_handler(ctx, wallet_address, score)
    }

    pub fn close_ranked_room(ctx: Context<CloseRankedRoom>, wallet_address: Pubkey) -> Result<()> {
        instructions::ranked::close_ranked_room_handler(ctx, wallet_address)
    }

    pub fn create_reward_claim(
        ctx: Context<CreateRewardClaim>,
        match_id: [u8; 32],
        mode: u8,
        region: u8,
        amount: u64,
    ) -> Result<()> {
        instructions::reward::create_reward_claim_handler(ctx, match_id, mode, region, amount)
    }

    pub fn claim_reward(ctx: Context<ClaimReward>) -> Result<()> {
        instructions::reward::claim_reward_handler(ctx)
    }

    pub fn delegate_guessr_state(
        ctx: Context<DelegateGuessrState>,
        args: DelegateGuessrStateArgs,
    ) -> Result<()> {
        instructions::delegate_guessr_state_handler(ctx, args)
    }

    pub fn commit_guessr_state<'info>(
        ctx: Context<'_, '_, '_, 'info, CommitGuessrState<'info>>,
    ) -> Result<()> {
        instructions::commit_guessr_state_handler(ctx)
    }
}
