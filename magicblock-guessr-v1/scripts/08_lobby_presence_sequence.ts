import { PublicKey, SystemProgram, TransactionInstruction } from '@solana/web3.js';
import {
  anchorDiscriminator,
  concatBinary,
  ensureMinimumBalance,
  getConnection,
  loadUserKeypairFromEnv,
  resolveGuessrProgramId,
  sendInstruction,
  writeReport,
} from './_shared';

const LOBBY_STATE_SEED = new TextEncoder().encode('lobby-state');
const PLAYER_STATUS_SEED = new TextEncoder().encode('player-status');

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

function parseLobbyOnlinePlayers(data: Buffer | null) {
  if (!data || data.length < 8 + 32 + 8 + 4) {
    return null;
  }
  return data.readUInt32LE(8 + 32 + 8);
}

async function joinLobby(params: {
  programId: PublicKey;
  wallet: ReturnType<typeof loadUserKeypairFromEnv>;
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

async function leaveLobby(params: {
  programId: PublicKey;
  wallet: ReturnType<typeof loadUserKeypairFromEnv>;
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
    data: concatBinary([anchorDiscriminator('leave_lobby'), params.wallet.publicKey.toBytes()]),
  });

  return sendInstruction({
    payer: params.wallet,
    instruction: ix,
  });
}

async function readOnlinePlayers(lobbyPda: PublicKey) {
  const connection = getConnection();
  const lobby = await connection.getAccountInfo(lobbyPda, 'confirmed');
  return parseLobbyOnlinePlayers(lobby?.data ?? null);
}

async function main() {
  const connection = getConnection();
  const multiplayerProgramId = resolveGuessrProgramId();

  const userA = loadUserKeypairFromEnv('USER_A');
  const userB = loadUserKeypairFromEnv('USER_B');
  const userC = loadUserKeypairFromEnv('USER_C');
  const lobbyPda = deriveLobby(multiplayerProgramId);

  await ensureMinimumBalance({ connection, wallet: userA, minSol: 0.05 });
  await ensureMinimumBalance({ connection, wallet: userB, minSol: 0.05 });
  await ensureMinimumBalance({ connection, wallet: userC, minSol: 0.05 });

  // Reset only A/B/C participation first to avoid stale state from previous script runs.
  for (const wallet of [userA, userB, userC]) {
    try {
      await leaveLobby({
        programId: multiplayerProgramId,
        wallet,
        lobbyPda,
      });
    } catch {
      // ignore if account not initialized yet
    }
  }

  const baseline = (await readOnlinePlayers(lobbyPda)) ?? 0;
  console.log('Lobby baseline online players:', baseline);
  writeReport('08_lobby_presence_sequence.log', `baseline=${baseline}`);

  const txA = await joinLobby({
    programId: multiplayerProgramId,
    wallet: userA,
    lobbyPda,
  });
  const afterA = (await readOnlinePlayers(lobbyPda)) ?? 0;
  console.log('User-A joined. Online:', afterA, `(delta +${afterA - baseline})`);
  console.log('User-A tx:', txA);
  writeReport('08_lobby_presence_sequence.log', `afterA=${afterA} txA=${txA}`);

  const txB = await joinLobby({
    programId: multiplayerProgramId,
    wallet: userB,
    lobbyPda,
  });
  const afterB = (await readOnlinePlayers(lobbyPda)) ?? 0;
  console.log('User-B joined. Online:', afterB, `(delta +${afterB - baseline})`);
  console.log('User-B tx:', txB);
  writeReport('08_lobby_presence_sequence.log', `afterB=${afterB} txB=${txB}`);

  const txC = await joinLobby({
    programId: multiplayerProgramId,
    wallet: userC,
    lobbyPda,
  });
  const afterC = (await readOnlinePlayers(lobbyPda)) ?? 0;
  console.log('User-C joined. Online:', afterC, `(delta +${afterC - baseline})`);
  console.log('User-C tx:', txC);
  writeReport('08_lobby_presence_sequence.log', `afterC=${afterC} txC=${txC}`);

  console.log('Expected deltas after A/B/C: +1, +2, +3 (relative to baseline).');
}

main().catch(error => {
  console.error(error);
  process.exit(1);
});
