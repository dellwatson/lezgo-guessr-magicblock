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

const LOBBY_POOL_SEED = new TextEncoder().encode('lobby-pool-v1');

async function main() {
  const payer = loadKeypair(requireEnv('SOLANA_PAYER_KEYPAIR'));
  const guessrProgramId = resolveGuessrProgramId();

  const [lobbyPoolPda] = PublicKey.findProgramAddressSync([LOBBY_POOL_SEED], guessrProgramId);

  const ix = new TransactionInstruction({
    programId: guessrProgramId,
    keys: [
      { pubkey: payer.publicKey, isSigner: true, isWritable: true },
      { pubkey: lobbyPoolPda, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: Buffer.from(anchorDiscriminator('initialize_lobby_pool')),
  });

  const signature = await sendInstruction({
    connection: getConnection(),
    payer,
    instruction: ix,
  });

  console.log('Lobby pool initialized');
  console.log('Lobby pool PDA:', lobbyPoolPda.toBase58());
  console.log('Signature:', signature);
  writeReport(
    '03c_initialize_lobby_pool.log',
    `signature=${signature} lobby_pool=${lobbyPoolPda.toBase58()}`
  );
}

main().catch(error => {
  console.error(error);
  process.exit(1);
});
