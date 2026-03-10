use anchor_lang::prelude::*;

#[account]
pub struct RewardClaim {
    pub player: Pubkey,
    pub match_id: [u8; 32],
    pub mode: u8,
    pub region: u8,
    pub amount: u64,
    pub claimed: bool,
    pub created_ts: i64,
    pub bump: u8,
    pub reserved: [u8; 6],
}
