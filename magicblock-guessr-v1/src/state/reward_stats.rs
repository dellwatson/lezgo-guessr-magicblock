use anchor_lang::prelude::*;

#[account]
pub struct PlayerRewardStats {
    pub player: Pubkey,
    pub total_earned: u64,
    pub last_minted_total: u64,
    pub last_update_ts: i64,
    pub bump: u8,
    pub reserved: [u8; 7],
}

impl PlayerRewardStats {
    pub fn touch(&mut self, player: Pubkey, bump: u8, now_ts: i64) {
        self.player = player;
        self.bump = bump;
        self.last_minted_total = 0;
        self.last_update_ts = now_ts;
        self.reserved = [0; 7];
    }
}
