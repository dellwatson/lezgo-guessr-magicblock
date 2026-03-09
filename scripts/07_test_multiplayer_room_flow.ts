import { Buffer } from 'buffer';
import { PublicKey, SystemProgram, TransactionInstruction } from '@solana/web3.js';
import * as nacl from 'tweetnacl';
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
  const [roomPda] = PublicKey.findProgramAddressSync(
    [ROOM_ID_SEED, toFixedBytes32(roomId)],
    programId
  );
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
  player: ReturnType<typeof loadKeypair>;
  lobbyPda: PublicKey;
}) {
  const playerStatusPda = derivePlayerStatus(params.programId, params.player.publicKey);
  const ix = new TransactionInstruction({
    programId: params.programId,
    keys: [
      { pubkey: params.player.publicKey, isSigner: true, isWritable: true },
      { pubkey: params.lobbyPda, isSigner: false, isWritable: true },
      { pubkey: playerStatusPda, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: concatBinary([
      anchorDiscriminator('join_lobby'),
      params.player.publicKey.toBytes(),
      params.player.publicKey.toBytes(),
    ]),
  });

  return sendInstruction({
    payer: params.player,
    instruction: ix,
  });
}

async function enterRoom(params: {
  programId: PublicKey;
  player: ReturnType<typeof loadKeypair>;
  roomPda: PublicKey;
  duelRoomPda: PublicKey;
}) {
  const playerStatusPda = derivePlayerStatus(params.programId, params.player.publicKey);
  const ix = new TransactionInstruction({
    programId: params.programId,
    keys: [
      { pubkey: params.player.publicKey, isSigner: true, isWritable: true },
      { pubkey: playerStatusPda, isSigner: false, isWritable: true },
      { pubkey: params.duelRoomPda, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: concatBinary([
      anchorDiscriminator('enter_room'),
      params.player.publicKey.toBytes(),
      params.roomPda.toBytes(),
    ]),
  });

  return sendInstruction({
    payer: params.player,
    instruction: ix,
  });
}

async function heartbeat(params: {
  programId: PublicKey;
  player: ReturnType<typeof loadKeypair>;
  lobbyPda: PublicKey;
}) {
  const playerStatusPda = derivePlayerStatus(params.programId, params.player.publicKey);
  const ix = new TransactionInstruction({
    programId: params.programId,
    keys: [
      { pubkey: params.player.publicKey, isSigner: true, isWritable: true },
      { pubkey: params.lobbyPda, isSigner: false, isWritable: true },
      { pubkey: playerStatusPda, isSigner: false, isWritable: true },
    ],
    data: concatBinary([anchorDiscriminator('heartbeat'), params.player.publicKey.toBytes()]),
  });

  return sendInstruction({
    payer: params.player,
    instruction: ix,
  });
}

async function main() {
  const connection = getConnection();
  const multiplayerProgramId = resolveGuessrProgramId();
  const userA = loadKeypair(requireEnv('USER_A_KEYPAIR'));
  const userB = loadKeypair(requireEnv('USER_B_KEYPAIR'));
  const roomId = process.env.TEST_ROOM_ID || `guessr-room-${Date.now()}`;

  await ensureMinimumBalance({ connection, wallet: userA, minSol: 0.05 });
  await ensureMinimumBalance({ connection, wallet: userB, minSol: 0.05 });

  const lobbyPda = deriveLobby(multiplayerProgramId);
  const roomPda = deriveRoomPda(multiplayerProgramId, roomId);
  const duelRoomPda = deriveDuelRoomPda(multiplayerProgramId, roomPda);

  const delegateMessageA = `lezgo-guessr-er-delegate:${userA.publicKey.toBase58()}:${Date.now()}`;
  const delegateMessageB = `lezgo-guessr-er-delegate:${userB.publicKey.toBase58()}:${Date.now()}`;
  const delegateSigA = Buffer.from(
    nacl.sign.detached(new TextEncoder().encode(delegateMessageA), new Uint8Array(userA.secretKey))
  ).toString('base64');
  const delegateSigB = Buffer.from(
    nacl.sign.detached(new TextEncoder().encode(delegateMessageB), new Uint8Array(userB.secretKey))
  ).toString('base64');

  const userAJoinSig = await joinLobby({
    programId: multiplayerProgramId,
    player: userA,
    lobbyPda,
  });
  const userBJoinSig = await joinLobby({
    programId: multiplayerProgramId,
    player: userB,
    lobbyPda,
  });

  const userAEnterSig = await enterRoom({
    programId: multiplayerProgramId,
    player: userA,
    roomPda,
    duelRoomPda,
  });
  const userBEnterSig = await enterRoom({
    programId: multiplayerProgramId,
    player: userB,
    roomPda,
    duelRoomPda,
  });

  const userAHeartbeatSig = await heartbeat({
    programId: multiplayerProgramId,
    player: userA,
    lobbyPda,
  });
  const userBHeartbeatSig = await heartbeat({
    programId: multiplayerProgramId,
    player: userB,
    lobbyPda,
  });

  const lobbyAccount = await connection.getAccountInfo(lobbyPda, 'confirmed');
  const userAStatusPda = derivePlayerStatus(multiplayerProgramId, userA.publicKey);
  const userBStatusPda = derivePlayerStatus(multiplayerProgramId, userB.publicKey);
  const userAStatus = await connection.getAccountInfo(userAStatusPda, 'confirmed');
  const userBStatus = await connection.getAccountInfo(userBStatusPda, 'confirmed');

  const onlinePlayers =
    lobbyAccount && lobbyAccount.data.length >= 8 + 32 + 8 + 4
      ? lobbyAccount.data.readUInt32LE(8 + 32 + 8)
      : null;

  function parsePlayerStatus(data: Buffer | null) {
    if (!data || data.length < 8 + 32 + 32 + 32 + 8 + 1) {
      return null;
    }
    const activeRoom = new PublicKey(data.subarray(8 + 64, 8 + 96)).toBase58();
    const isOnline = data[8 + 96 + 8] === 1;
    return { activeRoom, isOnline };
  }

  console.log('--- Multiplayer room test complete ---');
  console.log('Delegate proof A:', delegateSigA.slice(0, 20) + '...');
  console.log('Delegate proof B:', delegateSigB.slice(0, 20) + '...');
  console.log('Room ID seed:', roomId);
  console.log('Room PDA:', roomPda.toBase58());
  console.log('Duel room PDA:', duelRoomPda.toBase58());
  console.log('Lobby PDA:', lobbyPda.toBase58());
  console.log('Online players:', onlinePlayers ?? 'N/A');
  console.log('User A join tx:', userAJoinSig);
  console.log('User B join tx:', userBJoinSig);
  console.log('User A enter room tx:', userAEnterSig);
  console.log('User B enter room tx:', userBEnterSig);
  console.log('User A heartbeat tx:', userAHeartbeatSig);
  console.log('User B heartbeat tx:', userBHeartbeatSig);
  console.log('User A status:', parsePlayerStatus(userAStatus?.data ?? null));
  console.log('User B status:', parsePlayerStatus(userBStatus?.data ?? null));
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
