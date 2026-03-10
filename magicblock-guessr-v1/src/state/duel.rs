use anchor_lang::prelude::*;

#[account]
pub struct DuelRoom {
    pub room_id: [u8; 32],
    pub host: Pubkey,
    pub challenger: Pubkey,
    pub player_count: u8,
    pub is_settled: bool,
    pub host_score: u64,
    pub challenger_score: u64,
    pub host_earned: u64,
    pub host_lost: u64,
    pub challenger_earned: u64,
    pub challenger_lost: u64,
    pub winner: Pubkey,
    pub last_update_ts: i64,
    pub bump: u8,
    pub reserved: [u8; 14],
}
