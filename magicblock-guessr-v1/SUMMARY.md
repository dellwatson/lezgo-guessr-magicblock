# MagicBlock Guessr v1 (Pooling Demo) Summary

This `v1` program is a demo-oriented simplification of the original Guessr (`v0`) design.

Key goals:
- Avoid per-room PDAs (no "create PDA + delegate again" every match).
- Use a single on-chain `room_pool` PDA as a matchmaking signal bus (ring buffer).
- Keep base-layer as the source of truth for long-term stats + SPL minting via Magic Actions on commit.

---

## What Changed vs v0

- **No per-match/room PDAs**: matchmaking + room discovery uses `room_pool` (one PDA).
- **ER gameplay**: most gameplay txs are executed on the Ephemeral Rollup (ER) using the session key.
- **Commit + mint on base**: end-of-game triggers an ER commit, then a Magic Action mints SPL reward on base.

---

## Scripts (Admin / Devnet)

All scripts live in `@programs/magicblock-guessr-v1/scripts`.

Required env:

```bash
export SOLANA_RPC_URL="https://api.devnet.solana.com"
export SOLANA_PAYER_KEYPAIR="$HOME/.config/solana/id.json"
export GUESSR_PROGRAM_ID="<deployed-program-id>"
export REWARD_MINT="<spl-mint>"
export REWARD_TREASURY_TOKEN_ACCOUNT="<treasury-token-account>"
```

Order:
1. `01_create_reward_mint.sh` (creates mint + treasury token account)
2. `01_set_reward_mint_authority.ts` (sets mint authority to the program’s `mint_authority` PDA)
3. `02_deploy_multiplayer_program.sh` (deploy/upgrade v1 program)
4. `03_initialize_guessr_system.ts` (creates: `lobby_state`, `ranked_config`, `room_pool`)
5. `03b_initialize_leaderboard.ts` (creates: `leaderboard`)
6. `04_delegate_guessr_state.ts` (delegates required PDAs to your ER validator)
7. (Optional) `11_check_delegation_status.ts`

Notes:
- `03` and `03b` are separate to avoid Solana BPF stack limits during initialization.
- Delegation only works for accounts that already exist on base (because they must be initialized first).

---

## PDAs (v1)

Legend:
- **Scope**: Global (one per program) vs Per-user (one per wallet) vs Pool (ring buffer)
- **Created by**: Admin script vs Client session key (base tx)
- **Delegated by**: Admin vs Client session key (recommended) vs both
- **Used on**: Base layer only vs ER execution

| PDA / Account | Seeds | Scope | Created by | Delegated by | Used on ER? | Purpose |
|---|---|---|---|---|---|---|
| `lobby_state` | `["lobby-state"]` | Global | Admin (`03`) | Admin (`04`) | Yes | Heartbeat TTL + global lobby config |
| `ranked_config` | `["ranked-config"]` | Global | Admin (`03`) | Admin (`04`) | Yes | Reward mint + treasury + params |
| `room_pool` | `["room-pool"]` | Pool | Admin (`03`) | Admin (`04`) | Yes | Matchmaking signal bus (ring buffer) |
| `leaderboard` | `["leaderboard"]` | Global | Admin (`03b`) | Admin (`04`) | Yes | Top-N snapshots (XP / winrate / earnings / earned) |
| `mint_authority` | `["mint-authority"]` | Global (PDA signer) | N/A | N/A | Used by Magic Action | PDA signer used to mint rewards on base |
| `player_status` | `["player-status", wallet]` | Per-user | Client (base `join_lobby`) | Client session key (recommended) | Yes | Lobby presence + “is_online” |
| `player_live_state` | `["player-live-state", wallet]` | Per-user | Client (base `init_player_accounts`) | Client session key (recommended) | Yes | Real-time state (room_id, movement_hash, score…) |
| `player_profile` | `["player-profile", wallet]` | Per-user | Client (base `init_player_accounts`) | Client session key (recommended) | Yes | XP + win/loss + net earnings |
| `player_reward_stats` | `["player-reward-stats", wallet]` | Per-user | Client (base `init_player_accounts`) | Client session key (recommended) | Yes | Total earned (lifetime metric) |

Important:
- **Admin does not need to delegate per-user PDAs** if the client session key runs:
  - `init_player_accounts` (base, once), then
  - `delegate_guessr_state` for those per-user PDAs (base, once).

---

## Client Flow (Expo) with Session Key

### Bootstrap (one-time per install/session)
1. User connects wallet (MWA).
2. App generates a local **session keypair** and stores it in AsyncStorage with the user wallet address.
3. User funds session key with SOL (so the session can pay fees).
4. Session key sends base txs (no wallet popup after funding):
   - `join_lobby(wallet, session)` to create/update `player_status`
   - `init_player_accounts(wallet)` to create `player_profile`, `player_reward_stats`, `player_live_state`
   - `delegate_guessr_state(...)` to delegate: global PDAs + the user PDAs above

After this point:
- Gameplay txs go to **ER RPC** and are signed by the session key (no wallet prompts).

### Lobby Presence
- Client periodically calls `heartbeat(wallet)` (signed by session key).
- UI “online players” can be shown in 2 ways:
  1. Fast: read `lobby_state.online_players` (can drift unless stale players are pruned).
  2. Demo-friendly: derive online list by scanning `player_live_state` or `player_status` accounts and filtering by `last_update_ts` within TTL (see `scripts/watch_online_pool.ts` for reference).

---

## Matchmaking Pool (room_pool)

`room_pool` is a fixed-size ring buffer. Each `enter_room` call appends one entry.

### Entry Fields (conceptual)
- `room_id`: `[u8;32]` (we store a 32-byte ID, derived from your “challenge id / room id”)
- `wallet`, `session`: `[u8;32]` (pubkey bytes)
- `status`: `WAITING`, `JOINING`, `CONFIRMED`, `CLEARED`
- `slot_filled/slot_total`: `1/2`, `2/2`, or `1/1` for ranked
- `players[2]`: the agreed pair `[host, joiner]` for duel, or `[wallet, 0]` for ranked
- `last_update_ts`: timestamp used for client-side TTL filtering

### Duel Flow (recommended)
1. Host writes: `WAITING (1/2)` for `room_id`
2. Joiner writes: `JOINING (1/2, players=[host, joiner])`
3. Host picks the first valid joiner request and writes: `CONFIRMED (2/2, players=[host, joiner])`
4. Both clients observe the `CONFIRMED` entry and navigate to the match screen.

This resolves “multiple joiners” by making the host the final confirmer.

### Ranked Solo Flow
- Client writes `CONFIRMED (1/1)` and immediately navigates (no join/confirm handshake).

### Reading the Pool on Client
- Subscribe to the `room_pool` account on ER.
- Decode the ring buffer; sort by newest (`write_index`).
- Filter by:
  - `now - last_update_ts <= ttl`
  - `match_mode`
  - `status`
  - and `room_id` when you’re tracking a specific match.

---

## End of Game (Commit + Magic Action Mint)

For demo, client drives end-of-game:
1. On ER: `commit_match_result_pool(wallet, match_mode, did_win, xp_gained, earning_delta, final_score)`
   - updates per-user stats + leaderboard in ER state
2. On ER: `commit_pool_with_reward(mint_amount)`
   - commits delegated PDAs back to base
   - Magic Action mints SPL reward on base to the player’s reward-mint ATA

Production note:
- `mint_amount` should be computed on-chain from committed state, not provided by the client.

