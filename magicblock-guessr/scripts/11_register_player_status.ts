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

async function main() {
  const connection = getConnection();
  const programId = resolveGuessrProgramId();
  const player = loadUserKeypairFromEnv('PLAYER');
  const lobbyPda = deriveLobby(programId);
  const sessionAddressRaw = process.env.SESSION_ADDRESS?.trim();
  const sessionAddress = sessionAddressRaw
    ? new PublicKey(sessionAddressRaw)
    : player.publicKey;

  if (!sessionAddressRaw) {
    console.warn(
      'SESSION_ADDRESS is not set. Using wallet pubkey as session address. Session key auth will be disabled until updated.'
    );
  }

  await ensureMinimumBalance({ connection, wallet: player, minSol: 0.02 });

  const playerStatusPda = derivePlayerStatus(programId, player.publicKey);
  const ix = new TransactionInstruction({
    programId,
    keys: [
      { pubkey: player.publicKey, isSigner: true, isWritable: true },
      { pubkey: lobbyPda, isSigner: false, isWritable: true },
      { pubkey: playerStatusPda, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: concatBinary([
      anchorDiscriminator('join_lobby'),
      player.publicKey.toBytes(),
      sessionAddress.toBytes(),
    ]),
  });

  const signature = await sendInstruction({
    payer: player,
    instruction: ix,
  });

  console.log('Registered player status on base:', player.publicKey.toBase58());
  console.log('Session address set to:', sessionAddress.toBase58());
  console.log('Signature:', signature);
  writeReport(
    '11_register_player_status.log',
    `player=${player.publicKey.toBase58()} session=${sessionAddress.toBase58()} sig=${signature}`
  );
}

main().catch(error => {
  console.error(error);
  process.exit(1);
});
