use anchor_lang::prelude::*;
use bytemuck::{Pod, Zeroable};

use crate::constants::ROOM_POOL_MAX_ENTRIES;

#[repr(C)]
#[derive(Clone, Copy, Default, Pod, Zeroable)]
pub struct RoomPoolEntry {
    pub room_id: [u8; 32],
    pub wallet: [u8; 32],
    pub session: [u8; 32],
    pub status: u8,
    pub slot_filled: u8,
    pub slot_total: u8,
    pub match_mode: u8,
    pub players: [[u8; 32]; 2],
    pub padding: [u8; 4],
    pub last_update_ts: i64,
}

#[account(zero_copy)]
#[repr(C)]
pub struct RoomPool {
    pub write_index: u32,
    pub entry_count: u32,
    pub entries: [RoomPoolEntry; ROOM_POOL_MAX_ENTRIES],
    pub bump: u8,
    pub reserved: [u8; 7],
}

impl RoomPool {
    pub fn push_entry(&mut self, entry: RoomPoolEntry) {
        let idx = (self.write_index as usize) % ROOM_POOL_MAX_ENTRIES;
        self.entries[idx] = entry;
        self.write_index = self.write_index.wrapping_add(1);
        if self.entry_count < ROOM_POOL_MAX_ENTRIES as u32 {
            self.entry_count = self.entry_count.saturating_add(1);
        }
    }
}
