use anchor_lang::prelude::*;

#[account]
pub struct PlayerStatus {
    pub player: Pubkey,
    pub session_address: Pubkey,
    pub active_room: [u8; 32],
    pub last_heartbeat_ts: i64,
    pub is_online: bool,
    pub bump: u8,
    pub reserved: [u8; 6],
}

#[account]
pub struct PlayerLiveState {
    pub player: Pubkey,
    pub wallet_address: Pubkey,
    pub session_address: Pubkey,
    pub room_id: [u8; 32],
    pub round_index: u16,
    pub hp: u16,
    pub total_score: u64,
    pub earned_amount: u64,
    pub movement_hash: [u8; 32],
    pub last_update_ts: i64,
    pub bump: u8,
    pub reserved: [u8; 7],
}

#[account]
pub struct PlayerProfile {
    pub player: Pubkey,
    pub total_xp: u64,
    pub duel_wins: u32,
    pub duel_losses: u32,
    pub ranked_wins: u32,
    pub ranked_losses: u32,
    pub net_earnings: i64,
    pub total_matches: u32,
    pub last_match_score: u64,
    pub last_update_ts: i64,
    pub bump: u8,
    pub reserved: [u8; 7],
}
