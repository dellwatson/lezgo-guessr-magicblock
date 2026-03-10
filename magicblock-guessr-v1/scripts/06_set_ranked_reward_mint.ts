import { PublicKey, TransactionInstruction } from '@solana/web3.js';
import {
  anchorDiscriminator,
  concatBinary,
  getConnection,
  loadKeypair,
  parsePublicKey,
  requireEnv,
  resolveGuessrProgramId,
  sendInstruction,
  writeReport,
} from './_shared';

async function main() {
  const payer = loadKeypair(requireEnv('SOLANA_PAYER_KEYPAIR'));
  const rankedProgramId = resolveGuessrProgramId();
  const nextRewardMint = parsePublicKey(requireEnv('NEXT_REWARD_MINT'));
  const nextTreasuryTokenAccount = parsePublicKey(requireEnv('NEXT_REWARD_TREASURY_TOKEN_ACCOUNT'));

  const [rankedConfigPda] = PublicKey.findProgramAddressSync(
    [new TextEncoder().encode('ranked-config')],
    rankedProgramId
  );

  const ix = new TransactionInstruction({
    programId: rankedProgramId,
    keys: [
      { pubkey: payer.publicKey, isSigner: true, isWritable: true },
      { pubkey: rankedConfigPda, isSigner: false, isWritable: true },
      { pubkey: nextTreasuryTokenAccount, isSigner: false, isWritable: false },
    ],
    data: concatBinary([anchorDiscriminator('set_reward_mint'), nextRewardMint.toBytes()]),
  });

  const signature = await sendInstruction({
    connection: getConnection(),
    payer,
    instruction: ix,
  });

  console.log('Ranked reward mint updated');
  console.log('New reward mint:', nextRewardMint.toBase58());
  console.log('New treasury token account:', nextTreasuryTokenAccount.toBase58());
  console.log('Signature:', signature);
  writeReport(
    '06_set_ranked_reward_mint.log',
    `signature=${signature} newRewardMint=${nextRewardMint.toBase58()} newTreasuryTokenAccount=${nextTreasuryTokenAccount.toBase58()} rankedConfigPda=${rankedConfigPda.toBase58()}`
  );
}

main().catch(error => {
  console.error(error);
  process.exit(1);
});
