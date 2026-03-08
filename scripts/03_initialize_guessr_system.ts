import { PublicKey, SystemProgram, TransactionInstruction } from '@solana/web3.js';
import {
  anchorDiscriminator,
  concatBinary,
  encodeI64,
  encodeU64,
  getConnection,
  loadKeypair,
  parsePublicKey,
  requireEnv,
  resolveGuessrProgramId,
  sendInstruction,
  writeReport,
} from './_shared';

const LOBBY_STATE_SEED = new TextEncoder().encode('lobby-state');
const RANKED_CONFIG_SEED = new TextEncoder().encode('ranked-config');
const MINT_AUTHORITY_SEED = new TextEncoder().encode('mint-authority');

async function main() {
  const payer = loadKeypair(requireEnv('SOLANA_PAYER_KEYPAIR'));
  const guessrProgramId = resolveGuessrProgramId();
  const rewardMint = parsePublicKey(requireEnv('REWARD_MINT'));
  const treasuryTokenAccount = parsePublicKey(requireEnv('REWARD_TREASURY_TOKEN_ACCOUNT'));
  const heartbeatTtlSec = Number(process.env.MULTIPLAYER_HEARTBEAT_TTL_SEC || '300');
  const rewardMultiplier = Number(process.env.REWARD_MULTIPLIER || '140');
  const penaltyDivisor = Number(process.env.PENALTY_DIVISOR || '160');
  const penaltyThreshold = Number(process.env.PENALTY_THRESHOLD || '420');

  const [lobbyStatePda] = PublicKey.findProgramAddressSync([LOBBY_STATE_SEED], guessrProgramId);
  const [rankedConfigPda] = PublicKey.findProgramAddressSync([RANKED_CONFIG_SEED], guessrProgramId);
  const [mintAuthorityPda] = PublicKey.findProgramAddressSync(
    [MINT_AUTHORITY_SEED],
    guessrProgramId
  );

  const ix = new TransactionInstruction({
    programId: guessrProgramId,
    keys: [
      { pubkey: payer.publicKey, isSigner: true, isWritable: true },
      { pubkey: lobbyStatePda, isSigner: false, isWritable: true },
      { pubkey: rankedConfigPda, isSigner: false, isWritable: true },
      { pubkey: rewardMint, isSigner: false, isWritable: false },
      { pubkey: treasuryTokenAccount, isSigner: false, isWritable: true },
      { pubkey: mintAuthorityPda, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: concatBinary([
      anchorDiscriminator('initialize_system'),
      encodeI64(heartbeatTtlSec),
      rewardMint.toBytes(),
      encodeU64(rewardMultiplier),
      encodeU64(penaltyDivisor),
      encodeU64(penaltyThreshold),
    ]),
  });

  const signature = await sendInstruction({
    connection: getConnection(),
    payer,
    instruction: ix,
  });

  console.log('Guessr system initialized');
  console.log('Lobby PDA:', lobbyStatePda.toBase58());
  console.log('Ranked config PDA:', rankedConfigPda.toBase58());
  console.log('Mint authority PDA:', mintAuthorityPda.toBase58());
  console.log('Signature:', signature);
  writeReport(
    '03_initialize_guessr_system.log',
    `signature=${signature} lobby=${lobbyStatePda.toBase58()} rankedConfig=${rankedConfigPda.toBase58()} mintAuthority=${mintAuthorityPda.toBase58()}`
  );
}

main().catch(error => {
  console.error(error);
  process.exit(1);
});
