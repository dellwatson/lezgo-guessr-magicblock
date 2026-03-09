pub const LOBBY_STATE_SPACE: usize = 8 + 32 + 8 + 4 + 1 + 19;
pub const PLAYER_STATUS_SPACE: usize = 8 + 32 + 32 + 32 + 8 + 1 + 1 + 6;
pub const PLAYER_LIVE_STATE_SPACE: usize = 8 + 32 + 32 + 32 + 32 + 2 + 2 + 8 + 8 + 32 + 8 + 1 + 7;
pub const PLAYER_PROFILE_SPACE: usize = 8 + 32 + 8 + 4 + 4 + 4 + 4 + 8 + 8 + 4 + 8 + 1 + 7;
pub const PLAYER_REWARD_STATS_SPACE: usize = 8 + 32 + 8 + 8 + 1 + 15;
pub const DUEL_ROOM_SPACE: usize =
    8 + 32 + 32 + 32 + 1 + 1 + 8 + 8 + 8 + 8 + 8 + 8 + 32 + 8 + 1 + 14;
pub const REWARD_CLAIM_SPACE: usize = 8 + 32 + 32 + 1 + 1 + 8 + 1 + 8 + 1 + 6;

pub const RANKED_CONFIG_SPACE: usize = 8 + 32 + 32 + 32 + 8 + 8 + 8 + 1 + 1 + 13;
pub const RANKED_ROOM_SPACE: usize = 8 + 154;
pub const LEADERBOARD_ENTRIES: usize = 25;
pub const LEADERBOARD_SPACE: usize = 8 + 8 + 1 + (LEADERBOARD_ENTRIES * 40 * 4) + 32;

pub const MATCH_MODE_DUEL: u8 = 0;
pub const MATCH_MODE_RANKED_SOLO: u8 = 1;

pub const ACTION_HINT_OPEN: u8 = 0;
pub const ACTION_MARK_MOVE: u8 = 1;
pub const ACTION_GUESS_SUBMIT: u8 = 2;
pub const MAX_ACCURACY_BPS: u16 = 10_000;
