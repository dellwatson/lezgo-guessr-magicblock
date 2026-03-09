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
        self.xp = [LeaderboardEntry::default(); LEADERBOARD_ENTRIES];
        self.winrate = [LeaderboardEntry::default(); LEADERBOARD_ENTRIES];
        self.net_earnings = [LeaderboardEntry::default(); LEADERBOARD_ENTRIES];
        self.reward_earned = [LeaderboardEntry::default(); LEADERBOARD_ENTRIES];
        self.reserved = [0; 32];
    }
}
