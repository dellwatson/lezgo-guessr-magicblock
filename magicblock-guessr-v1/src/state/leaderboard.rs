use anchor_lang::prelude::*;
use bytemuck::{Pod, Zeroable};

use crate::constants::LEADERBOARD_ENTRIES;

#[repr(C)]
#[derive(Clone, Copy, Default, Pod, Zeroable)]
pub struct LeaderboardEntry {
    pub player: [u8; 32],
    pub value: u64,
}

#[account(zero_copy)]
#[repr(C)]
pub struct LeaderboardState {
    pub last_update_ts: i64,
    pub bump: u8,
    pub padding: [u8; 7],
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
        self.padding = [0; 7];
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
