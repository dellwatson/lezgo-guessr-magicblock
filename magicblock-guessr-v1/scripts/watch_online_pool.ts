import { PublicKey } from '@solana/web3.js';
import { getConnection, resolveGuessrProgramId } from './_shared';

const PLAYER_LIVE_STATE_SIZE = 204; // bytes, from PlayerLiveState layout in the program
const POLL_INTERVAL_MS = 1000;
const STALE_AFTER_SEC = 10;

const pool = new Map<
  string,
  {
    wallet: string;
    session: string;
    roomId: string;
    lastUpdate: number;
  }
>();

function parsePlayerLiveState(data: Buffer) {
  if (data.length !== PLAYER_LIVE_STATE_SIZE) return null;

  // Layout (after 8-byte Anchor discriminator):
  // 0..32  player
  // 32..64 wallet_address
  // 64..96 session_address
  // 96..128 room_id ([u8;32])
  // 128..130 round_index (u16)
  // 130..132 hp (u16)
  // 132..140 total_score (u64)
  // 140..148 earned_amount (u64)
  // 148..180 movement_hash ([u8;32])
  // 180..188 last_update_ts (i64)

  const walletBytes = data.subarray(8 + 32, 8 + 64);
  const sessionBytes = data.subarray(8 + 64, 8 + 96);
  const roomBytes = data.subarray(8 + 96, 8 + 128);
  const lastUpdateBytes = data.subarray(8 + 180, 8 + 188);

  const wallet = new PublicKey(walletBytes).toBase58();
  const session = new PublicKey(sessionBytes).toBase58();
  const roomId = Buffer.from(roomBytes).toString('hex');
  const lastUpdate = Number(lastUpdateBytes.readBigInt64LE(0));

  return { wallet, session, roomId, lastUpdate };
}

async function pollOnce() {
  const connection = getConnection();
  const programId = resolveGuessrProgramId();

  const accounts = await connection.getProgramAccounts(programId, {
    filters: [{ dataSize: PLAYER_LIVE_STATE_SIZE }],
    commitment: 'confirmed',
  });

  const nowSec = Math.floor(Date.now() / 1000);

  for (const { account } of accounts) {
    const parsed = parsePlayerLiveState(account.data as Buffer);
    if (!parsed) continue;

    const key = `${parsed.wallet}:${parsed.session}:${parsed.roomId}`;
    pool.set(key, {
      wallet: parsed.wallet,
      session: parsed.session,
      roomId: parsed.roomId,
      lastUpdate: parsed.lastUpdate,
    });
  }

  // Drop stale entries
  for (const [key, value] of pool.entries()) {
    if (nowSec - value.lastUpdate > STALE_AFTER_SEC) {
      pool.delete(key);
    }
  }

  const snapshot = Array.from(pool.values());
  console.log('Online pool size:', snapshot.length);
  console.log(snapshot);
}

async function main() {
  // Simple polling loop, let Bun/Node keep this process alive.
  // You can later replace this with onProgramAccountChange if desired.
  // eslint-disable-next-line no-constant-condition
  while (true) {
    try {
      await pollOnce();
    } catch (error) {
      console.error('watch_online_pool poll error', error);
    }
    await new Promise(resolve => setTimeout(resolve, POLL_INTERVAL_MS));
  }
}

main().catch(error => {
  console.error(error);
  process.exit(1);
});
