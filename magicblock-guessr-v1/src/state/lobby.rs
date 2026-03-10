use anchor_lang::prelude::*;

#[account]
pub struct LobbyState {
    pub authority: Pubkey,
    pub heartbeat_ttl_sec: i64,
    pub online_players: u32,
    pub bump: u8,
    pub reserved: [u8; 19],
}
