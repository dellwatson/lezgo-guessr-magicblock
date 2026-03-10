use anchor_lang::prelude::*;

use crate::constants::LEADERBOARD_ENTRIES;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default)]
pub struct LeaderboardEntry {
    pub player: Pubkey,
    pub value: u64,
}

#[account]
pub struct LeaderboardState {
    pub last_update_ts: i64,
    pub bump: u8,
    pub xp: [LeaderboardEntry; LEADERBOARD_ENTRIES],
    pub winrate: [LeaderboardEntry; LEADERBOARD_ENTRIES],
    pub net_earnings: [LeaderboardEntry; LEADERBOARD_ENTRIES],
    pub reward_earned: [LeaderboardEntry; LEADERBOARD_ENTRIES],
    pub reserved: [u8; 32],
}

impl LeaderboardState {
    pub fn reset(&mut self, bump: u8, now_ts: i64) {
        self.last_update_ts = now_ts;
        self.bump = bump;
        for entry in self.xp.iter_mut() {
            *entry = LeaderboardEntry::default();
        }
        for entry in self.winrate.iter_mut() {
            *entry = LeaderboardEntry::default();
        }
        for entry in self.net_earnings.iter_mut() {
            *entry = LeaderboardEntry::default();
        }
        for entry in self.reward_earned.iter_mut() {
            *entry = LeaderboardEntry::default();
        }
        self.reserved = [0; 32];
    }
}
