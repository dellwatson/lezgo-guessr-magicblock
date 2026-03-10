# magicblock-guessr – ER Integration & Rewards Design

## Goals

- Use **Ephemeral Rollups** to run game logic off L1:
  - Delegate all relevant state PDAs to ER validators.
  - Commit final state back to Solana when needed.
- Use **string-based IDs** (hashed to `[u8; 32]`) for:
  - `room_id` (duel / multiplayer rooms).
  - `challenge_hash` (ranked solo matches).
- Track **live player presence & input** via:
  - [PlayerStatus](cci:2://file:///Users/dellwatson/Desktop/2026-LEZGO-MOGATE/tixia-expo/@programs/magicblock-guessr/src/state/player.rs:3:0-10:1) (online / active_room).
  - [PlayerLiveState](cci:2://file:///Users/dellwatson/Desktop/2026-LEZGO-MOGATE/tixia-expo/@programs/magicblock-guessr/src/state/player.rs:13:0-26:1) (wallet, session, room, score, hp, movement, timestamps).
- Implement a **RewardClaim** system for `$TESTGO`:
  - Record rewards and regions on ER/L1.
  - Support both auto-claim and manual claim.
- Drive **presence / matchmaking** mostly from **client/backends**:
  - Load all [PlayerLiveState](cci:2://file:///Users/dellwatson/Desktop/2026-LEZGO-MOGATE/tixia-expo/@programs/magicblock-guessr/src/state/player.rs:13:0-26:1) updates.
  - Filter by `wallet_address`, `session_address`, `room_id`.
  - Drop stale entries locally (no on-chain prune needed).

---

## Core State Model

### Accounts

- `LobbyState`
  - Global lobby config: `heartbeat_ttl_sec`, `online_players`, `bump`.

- [PlayerStatus](cci:2://file:///Users/dellwatson/Desktop/2026-LEZGO-MOGATE/tixia-expo/@programs/magicblock-guessr/src/state/player.rs:3:0-10:1)
  - `player: Pubkey`
  - `active_room: [u8; 32]` (room or match id hash)
  - `last_heartbeat_ts: i64`
  - `is_online: bool`
  - `bump, reserved`.

- [PlayerLiveState](cci:2://file:///Users/dellwatson/Desktop/2026-LEZGO-MOGATE/tixia-expo/@programs/magicblock-guessr/src/state/player.rs:13:0-26:1)
  - `player: Pubkey` (payer / signer)
  - `wallet_address: Pubkey` (L1 wallet id)
  - `session_address: Pubkey` (ephemeral session key)
  - `room_id: [u8; 32]` (string-hash)
  - `round_index: u16`
  - `hp: u16`
  - `total_score: u64`
  - `earned_amount: u64`
  - `movement_hash: [u8; 32]`
  - `last_update_ts: i64`
  - `bump, reserved`.

- [RankedConfig](cci:2://file:///Users/dellwatson/Desktop/2026-LEZGO-MOGATE/tixia-expo/@programs/magicblock-guessr/src/state/ranked.rs:3:0-13:1)
  - `authority` (admin / treasury signer)
  - `reward_mint`, `treasury_token_account`
  - `reward_multiplier`, `penalty_divisor`, `penalty_threshold`
  - `mint_authority_bump`, `bump`.

- [RankedRoom](cci:2://file:///Users/dellwatson/Desktop/2026-LEZGO-MOGATE/tixia-expo/@programs/magicblock-guessr/src/state/ranked.rs:16:0-34:1)
  - `player: Pubkey`
  - `challenge_hash: [u8; 32]` (match id hash)
  - Various stats: `score`, `total_earned`, `total_lost`, `action_count`, etc.
  - `is_settled: bool`, `last_movement_hash`, `last_action_ts`, `bump`.

- [DuelRoom](cci:2://file:///Users/dellwatson/Desktop/2026-LEZGO-MOGATE/tixia-expo/@programs/magicblock-guessr/src/state/duel.rs:3:0-19:1)
  - `room_id: [u8; 32]`
  - `host`, `challenger`
  - `player_count`, `is_settled`
  - `host_score`, `challenger_score`
  - `host_earned`, `host_lost`, `challenger_earned`, `challenger_lost`
  - `winner`, `last_update_ts`, `bump`.

- [PlayerProfile](cci:2://file:///Users/dellwatson/Desktop/2026-LEZGO-MOGATE/tixia-expo/@programs/magicblock-guessr/src/state/player.rs:29:0-42:1)
  - XP, wins/losses, net_earnings, last_match_score, etc.

- [RewardClaim](cci:2://file:///Users/dellwatson/Desktop/2026-LEZGO-MOGATE/tixia-expo/@programs/magicblock-guessr/src/state/reward.rs:3:0-13:1)
  - `player: Pubkey`
  - `match_id: [u8; 32]` (ranked challenge_hash or duel room_id)
  - `mode: u8` (`MATCH_MODE_DUEL` or `MATCH_MODE_RANKED_SOLO`)
  - `region: u8` (app-defined: e.g. Asia/EU/US)
  - `amount: u64`
  - `claimed: bool`
  - `created_ts: i64`
  - `bump`.

---

## ER Delegation Model

### On-chain

- Program is marked with `#[ephemeral]` in [lib.rs](cci:7://file:///Users/dellwatson/Desktop/2026-LEZGO-MOGATE/tixia-expo/@programs/magicblock-guessr/src/lib.rs:0:0-0:0).
- ER glue uses MagicBlock SDK attributes:
  - `#[delegate]` on [DelegateGuessrState<'info>](cci:2://file:///Users/dellwatson/Desktop/2026-LEZGO-MOGATE/tixia-expo/@programs/magicblock-guessr/src/instructions/er.rs:48:0-54:1)  
    ⇒ generates `delegate_pda(&payer, seeds, DelegateConfig)` for **one** `#[account(del)]` field (`pda`).

  - `#[commit]` on [CommitGuessrState<'info>](cci:2://file:///Users/dellwatson/Desktop/2026-LEZGO-MOGATE/tixia-expo/@programs/magicblock-guessr/src/instructions/er.rs:63:0-87:1)  
    ⇒ injects ER magic accounts, while handler logic commits `LobbyState` + `RankedConfig` and any additional writable `remaining_accounts`.

- Delegation handler now accepts a target selector + seed inputs and can derive:
  - `LobbyState`, `RankedConfig`
  - `PlayerStatus`, `PlayerLiveState`, `PlayerProfile`
  - `DuelRoom`, `RankedRoom`, `RewardClaim`
- Delegation is still **one PDA per instruction** on-chain (SDK `#[delegate]` constraint), but client scripts can pack many delegation instructions in one transaction.
- New script: `@programs/magicblock-guessr/scripts/04_delegate_guessr_state.ts`
  - Tries one-transaction delegation first.
  - Automatically splits into multiple transactions only when transaction size limits are hit.
  - Delegates all required existing PDAs from the configured global/player/duel/ranked/reward inputs.
- Commit handler now commits:
  - Always: `LobbyState` + `RankedConfig`.
  - Plus: any additional writable PDAs passed via `remaining_accounts`.
- New script: `@programs/magicblock-guessr/scripts/05_commit_guessr_state.ts`
  - Tries committing all discovered existing PDAs in one transaction.
  - Automatically splits by account chunks if transaction-size limits are hit.

- v1 pooling demo:
  - New `room_pool` PDA (single pool for matchmaking signals).
  - `room_pool` is a fixed-size ring buffer (room_id, wallet, session, status, slots, players, timestamp).
  - Duel matchmaking uses `waiting (1/2) → joining (1/2) → confirmed (2/2)` entries.
  - New `commit_pool_with_reward` magic-action commit for pooled flows (syncs ER state + mints SPL reward).
