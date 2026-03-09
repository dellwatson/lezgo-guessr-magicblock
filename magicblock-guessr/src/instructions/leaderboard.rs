use anchor_lang::prelude::*;

use crate::constants::LEADERBOARD_ENTRIES;
use crate::state::{LeaderboardEntry, LeaderboardState, PlayerProfile, PlayerRewardStats};

fn update_list(list: &mut [LeaderboardEntry; LEADERBOARD_ENTRIES], player: Pubkey, value: u64) {
    for entry in list.iter_mut() {
        if entry.player == player {
            *entry = LeaderboardEntry::default();
        }
    }

    if value == 0 {
        return;
    }

    let mut insert_idx: Option<usize> = None;
    for (idx, entry) in list.iter().enumerate() {
        if entry.player == Pubkey::default() || value >= entry.value {
            insert_idx = Some(idx);
            break;
        }
    }

    let idx = match insert_idx {
        Some(value) => value,
        None => return,
    };

    for i in (idx + 1..LEADERBOARD_ENTRIES).rev() {
        list[i] = list[i - 1];
    }
    list[idx] = LeaderboardEntry { player, value };
}

fn compute_winrate_bps(profile: &PlayerProfile) -> u64 {
    let wins = profile.duel_wins as u64 + profile.ranked_wins as u64;
    let losses = profile.duel_losses as u64 + profile.ranked_losses as u64;
    let total = wins + losses;
    if total == 0 {
        return 0;
    }
    wins.saturating_mul(10_000).checked_div(total).unwrap_or(0)
}

pub fn update_leaderboards(
    leaderboard: &mut LeaderboardState,
    profile: &PlayerProfile,
    reward_stats: &PlayerRewardStats,
    now_ts: i64,
) {
    leaderboard.last_update_ts = now_ts;

    update_list(&mut leaderboard.xp, profile.player, profile.total_xp);
    update_list(&mut leaderboard.winrate, profile.player, compute_winrate_bps(profile));

    let earnings = if profile.net_earnings > 0 {
        profile.net_earnings as u64
    } else {
        0
    };
    update_list(&mut leaderboard.net_earnings, profile.player, earnings);
    update_list(
        &mut leaderboard.reward_earned,
        profile.player,
        reward_stats.total_earned,
    );
}
