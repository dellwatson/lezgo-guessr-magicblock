use anchor_lang::prelude::*;

#[account]
pub struct RankedConfig {
    pub authority: Pubkey,
    pub reward_mint: Pubkey,
    pub treasury_token_account: Pubkey,
    pub reward_multiplier: u64,
    pub penalty_divisor: u64,
    pub penalty_threshold: u64,
    pub mint_authority_bump: u8,
    pub bump: u8,
    pub reserved: [u8; 13],
}

#[account]
pub struct RankedRoom {
    pub player: Pubkey,
    pub challenge_hash: [u8; 32],
    pub score: u64,
    pub total_earned: u64,
    pub total_lost: u64,
    pub action_count: u32,
    pub hints_opened: u16,
    pub last_round_index: u16,
    pub last_hp: u16,
    pub last_accuracy_bps: u16,
    pub last_distance_km: u32,
    pub last_action_kind: u8,
    pub is_settled: bool,
    pub last_movement_hash: [u8; 32],
    pub last_action_ts: i64,
    pub bump: u8,
    pub reserved: [u8; 7],
}
