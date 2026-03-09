import { AuthorityType, createSetAuthorityInstruction, getMint } from '@solana/spl-token';
import { PublicKey } from '@solana/web3.js';
import {
  getConnection,
  loadKeypair,
  requireEnv,
  resolveGuessrProgramId,
  sendInstruction,
  writeReport,
} from './_shared';

const MINT_AUTHORITY_SEED = new TextEncoder().encode('mint-authority');

async function main() {
  const payer = loadKeypair(requireEnv('SOLANA_PAYER_KEYPAIR'));
  const programId = resolveGuessrProgramId();
  const rewardMint = new PublicKey(requireEnv('REWARD_MINT'));

  const connection = getConnection();
  const mintInfo = await getMint(connection, rewardMint);
  const [mintAuthorityPda] = PublicKey.findProgramAddressSync([MINT_AUTHORITY_SEED], programId);

  const currentAuthority = mintInfo.mintAuthority;
  if (!currentAuthority) {
    throw new Error('Reward mint has no mint authority set.');
  }

  if (!currentAuthority.equals(payer.publicKey)) {
    throw new Error(
      `Payer is not current mint authority. Current authority: ${currentAuthority.toBase58()}`
    );
  }

  const ix = createSetAuthorityInstruction(
    rewardMint,
    payer.publicKey,
    AuthorityType.MintTokens,
    mintAuthorityPda
  );

  const signature = await sendInstruction({
    connection,
    payer,
    instruction: ix,
  });

  console.log('Reward mint authority updated');
  console.log('Reward mint:', rewardMint.toBase58());
  console.log('New mint authority PDA:', mintAuthorityPda.toBase58());
  console.log('Signature:', signature);

  writeReport(
    '01_set_reward_mint_authority.log',
    `signature=${signature} mint=${rewardMint.toBase58()} mintAuthority=${mintAuthorityPda.toBase58()}`
  );
}

main().catch(error => {
  console.error(error);
  process.exit(1);
});
