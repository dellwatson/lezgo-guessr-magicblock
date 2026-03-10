import { PublicKey, SystemProgram, TransactionInstruction } from '@solana/web3.js';
import {
  anchorDiscriminator,
  concatBinary,
  ensureMinimumBalance,
  getConnection,
  loadKeypair,
  requireEnv,
  resolveGuessrProgramId,
  sendInstruction,
  toFixedBytes32,
} from './_shared';

const LOBBY_STATE_SEED = new TextEncoder().encode('lobby-state');
const PLAYER_STATUS_SEED = new TextEncoder().encode('player-status');
const ROOM_ID_SEED = new TextEncoder().encode('room-id');
const DUEL_ROOM_SEED = new TextEncoder().encode('duel-room');

function deriveLobby(programId: PublicKey) {
  const [lobbyPda] = PublicKey.findProgramAddressSync([LOBBY_STATE_SEED], programId);
  return lobbyPda;
}

function derivePlayerStatus(programId: PublicKey, player: PublicKey) {
  const [playerPda] = PublicKey.findProgramAddressSync(
    [PLAYER_STATUS_SEED, player.toBytes()],
    programId
  );
  return playerPda;
}

function deriveRoomPda(programId: PublicKey, roomId: string) {
  const [roomPda] = PublicKey.findProgramAddressSync([ROOM_ID_SEED, toFixedBytes32(roomId)], programId);
  return roomPda;
}

function deriveDuelRoomPda(programId: PublicKey, roomPda: PublicKey) {
  const [duelRoomPda] = PublicKey.findProgramAddressSync(
    [DUEL_ROOM_SEED, roomPda.toBytes()],
    programId
  );
  return duelRoomPda;
}

async function joinLobby(params: {
  programId: PublicKey;
  wallet: ReturnType<typeof loadKeypair>;
  lobbyPda: PublicKey;
}) {
  const playerStatusPda = derivePlayerStatus(params.programId, params.wallet.publicKey);
  const ix = new TransactionInstruction({
    programId: params.programId,
    keys: [
      { pubkey: params.wallet.publicKey, isSigner: true, isWritable: true },
      { pubkey: params.lobbyPda, isSigner: false, isWritable: true },
      { pubkey: playerStatusPda, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: concatBinary([
      anchorDiscriminator('join_lobby'),
      params.wallet.publicKey.toBytes(),
      params.wallet.publicKey.toBytes(),
    ]),
  });

  return sendInstruction({
    payer: params.wallet,
    instruction: ix,
  });
}

async function enterRoom(params: {
  programId: PublicKey;
  wallet: ReturnType<typeof loadKeypair>;
  roomPda: PublicKey;
  duelRoomPda: PublicKey;
}) {
  const playerStatusPda = derivePlayerStatus(params.programId, params.wallet.publicKey);
  const ix = new TransactionInstruction({
    programId: params.programId,
    keys: [
      { pubkey: params.wallet.publicKey, isSigner: true, isWritable: true },
      { pubkey: playerStatusPda, isSigner: false, isWritable: true },
      { pubkey: params.duelRoomPda, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: concatBinary([
      anchorDiscriminator('enter_room'),
      params.wallet.publicKey.toBytes(),
      params.roomPda.toBytes(),
    ]),
  });

  return sendInstruction({
    payer: params.wallet,
    instruction: ix,
  });
}

async function heartbeat(params: {
  programId: PublicKey;
  wallet: ReturnType<typeof loadKeypair>;
  lobbyPda: PublicKey;
}) {
  const playerStatusPda = derivePlayerStatus(params.programId, params.wallet.publicKey);
  const ix = new TransactionInstruction({
    programId: params.programId,
    keys: [
      { pubkey: params.wallet.publicKey, isSigner: true, isWritable: true },
      { pubkey: params.lobbyPda, isSigner: false, isWritable: true },
      { pubkey: playerStatusPda, isSigner: false, isWritable: true },
    ],
    data: concatBinary([anchorDiscriminator('heartbeat'), params.wallet.publicKey.toBytes()]),
  });

  return sendInstruction({
    payer: params.wallet,
    instruction: ix,
  });
}

async function clearRoom(params: {
  programId: PublicKey;
  wallet: ReturnType<typeof loadKeypair>;
}) {
  const playerStatusPda = derivePlayerStatus(params.programId, params.wallet.publicKey);
  const ix = new TransactionInstruction({
    programId: params.programId,
    keys: [
      { pubkey: params.wallet.publicKey, isSigner: true, isWritable: true },
      { pubkey: playerStatusPda, isSigner: false, isWritable: true },
    ],
    data: concatBinary([anchorDiscriminator('clear_room'), params.wallet.publicKey.toBytes()]),
  });

  return sendInstruction({
    payer: params.wallet,
    instruction: ix,
  });
}

function parsePlayerStatus(data: Buffer | null) {
  if (!data || data.length < 8 + 32 + 32 + 32 + 8 + 1) {
    return null;
  }

  const activeRoom = new PublicKey(data.subarray(8 + 64, 8 + 96)).toBase58();
  const lastHeartbeatTs = Number(data.readBigInt64LE(8 + 96));
  const isOnline = data[8 + 96 + 8] === 1;
  return {
    activeRoom,
    lastHeartbeatTs,
    isOnline,
  };
}

async function main() {
  const connection = getConnection();
  const multiplayerProgramId = resolveGuessrProgramId();
  const userA = loadKeypair(requireEnv('USER_A_KEYPAIR'));
  const userB = loadKeypair(requireEnv('USER_B_KEYPAIR'));
  const roomId = process.env.TEST_DUEL_ROOM_ID || `duel-${Date.now()}`;

  await ensureMinimumBalance({ connection, wallet: userA, minSol: 0.05 });
  await ensureMinimumBalance({ connection, wallet: userB, minSol: 0.05 });

  const lobbyPda = deriveLobby(multiplayerProgramId);
  const roomPda = deriveRoomPda(multiplayerProgramId, roomId);
  const duelRoomPda = deriveDuelRoomPda(multiplayerProgramId, roomPda);

  const joinATx = await joinLobby({ programId: multiplayerProgramId, wallet: userA, lobbyPda });
  const joinBTx = await joinLobby({ programId: multiplayerProgramId, wallet: userB, lobbyPda });

  const enterATx = await enterRoom({
    programId: multiplayerProgramId,
    wallet: userA,
    roomPda,
    duelRoomPda,
  });
  const enterBTx = await enterRoom({
    programId: multiplayerProgramId,
    wallet: userB,
    roomPda,
    duelRoomPda,
  });

  const heartbeatATx = await heartbeat({ programId: multiplayerProgramId, wallet: userA, lobbyPda });
  const heartbeatBTx = await heartbeat({ programId: multiplayerProgramId, wallet: userB, lobbyPda });

  // Duel finished, clear room for both players.
  const clearATx = await clearRoom({ programId: multiplayerProgramId, wallet: userA });
  const clearBTx = await clearRoom({ programId: multiplayerProgramId, wallet: userB });

  const playerAStatusPda = derivePlayerStatus(multiplayerProgramId, userA.publicKey);
  const playerBStatusPda = derivePlayerStatus(multiplayerProgramId, userB.publicKey);
  const playerAStatus = await connection.getAccountInfo(playerAStatusPda, 'confirmed');
  const playerBStatus = await connection.getAccountInfo(playerBStatusPda, 'confirmed');

  console.log('--- Duel simulation complete ---');
  console.log('Room ID seed:', roomId);
  console.log('Room PDA:', roomPda.toBase58());
  console.log('Duel room PDA:', duelRoomPda.toBase58());
  console.log('User-A join tx:', joinATx);
  console.log('User-B join tx:', joinBTx);
  console.log('User-A enter room tx:', enterATx);
  console.log('User-B enter room tx:', enterBTx);
  console.log('User-A heartbeat tx:', heartbeatATx);
  console.log('User-B heartbeat tx:', heartbeatBTx);
  console.log('User-A clear room tx:', clearATx);
  console.log('User-B clear room tx:', clearBTx);
  console.log('User-A status:', parsePlayerStatus(playerAStatus?.data ?? null));
  console.log('User-B status:', parsePlayerStatus(playerBStatus?.data ?? null));
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
