import { PublicKey, SystemProgram, TransactionInstruction } from '@solana/web3.js';
import {
  anchorDiscriminator,
  getConnection,
  loadKeypair,
  requireEnv,
  resolveGuessrProgramId,
  sendInstruction,
  writeReport,
} from './_shared';

const LOBBY_STATE_SEED = new TextEncoder().encode('lobby-state');
const LEADERBOARD_SEED = new TextEncoder().encode('leaderboard');

async function main() {
  const payer = loadKeypair(requireEnv('SOLANA_PAYER_KEYPAIR'));
  const guessrProgramId = resolveGuessrProgramId();

  const [lobbyStatePda] = PublicKey.findProgramAddressSync([LOBBY_STATE_SEED], guessrProgramId);
  const [leaderboardPda] = PublicKey.findProgramAddressSync([LEADERBOARD_SEED], guessrProgramId);

  const ix = new TransactionInstruction({
    programId: guessrProgramId,
    keys: [
      { pubkey: payer.publicKey, isSigner: true, isWritable: true },
      { pubkey: lobbyStatePda, isSigner: false, isWritable: false },
      { pubkey: leaderboardPda, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: Buffer.from(anchorDiscriminator('initialize_leaderboard')),
  });

  const signature = await sendInstruction({
    connection: getConnection(),
    payer,
    instruction: ix,
  });

  console.log('Leaderboard initialized');
  console.log('Leaderboard PDA:', leaderboardPda.toBase58());
  console.log('Signature:', signature);
  writeReport(
    '03b_initialize_leaderboard.log',
    `signature=${signature} leaderboard=${leaderboardPda.toBase58()}`
  );
}

main().catch(error => {
  console.error(error);
  process.exit(1);
});
