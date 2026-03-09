import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  TokenAccountNotFoundError,
  createAssociatedTokenAccountInstruction,
  getAccount,
  getAssociatedTokenAddressSync,
} from '@solana/spl-token';
import { Buffer } from 'buffer';
import { PublicKey, SystemProgram, TransactionInstruction } from '@solana/web3.js';
import {
  anchorDiscriminator,
  concatBinary,
  encodeU64,
  ensureMinimumBalance,
  getConnection,
  loadKeypair,
  requireEnv,
  resolveGuessrProgramId,
  sendInstruction,
  toFixedBytes32,
} from './_shared';

const RANKED_CONFIG_SEED = new TextEncoder().encode('ranked-config');
const RANKED_ROOM_SEED = new TextEncoder().encode('ranked-room');
const MINT_AUTHORITY_SEED = new TextEncoder().encode('mint-authority');
const LOBBY_STATE_SEED = new TextEncoder().encode('lobby-state');
const PLAYER_STATUS_SEED = new TextEncoder().encode('player-status');
const PLAYER_LIVE_STATE_SEED = new TextEncoder().encode('player-live-state');
const PLAYER_PROFILE_SEED = new TextEncoder().encode('player-profile');
const PLAYER_REWARDS_SEED = new TextEncoder().encode('player-rewards');
const LEADERBOARD_SEED = new TextEncoder().encode('leaderboard');
const ACTION_HINT_OPEN = 0;
const ACTION_MARK_MOVE = 1;
const ACTION_GUESS_SUBMIT = 2;

type RankedConfigState = {
  rewardMint: PublicKey;
  treasuryTokenAccount: PublicKey;
  rewardMultiplier: bigint;
  penaltyDivisor: bigint;
  penaltyThreshold: bigint;
  mintAuthorityBump: number;
};

function deriveRankedConfig(programId: PublicKey) {
  const [rankedConfigPda] = PublicKey.findProgramAddressSync([RANKED_CONFIG_SEED], programId);
  return rankedConfigPda;
}

function deriveMintAuthority(programId: PublicKey) {
  const [mintAuthorityPda] = PublicKey.findProgramAddressSync([MINT_AUTHORITY_SEED], programId);
  return mintAuthorityPda;
}

function deriveRankedRoom(programId: PublicKey, player: PublicKey, challengeHash: Uint8Array) {
  const [rankedRoomPda] = PublicKey.findProgramAddressSync(
    [RANKED_ROOM_SEED, player.toBytes(), challengeHash],
    programId
  );
  return rankedRoomPda;
}

function derivePlayerRewards(programId: PublicKey, player: PublicKey) {
  const [playerRewardsPda] = PublicKey.findProgramAddressSync(
    [PLAYER_REWARDS_SEED, player.toBytes()],
    programId
  );
  return playerRewardsPda;
}

function deriveLeaderboard(programId: PublicKey) {
  const [leaderboardPda] = PublicKey.findProgramAddressSync([LEADERBOARD_SEED], programId);
  return leaderboardPda;
}

function deriveLobby(programId: PublicKey) {
  const [lobbyPda] = PublicKey.findProgramAddressSync([LOBBY_STATE_SEED], programId);
  return lobbyPda;
}

function derivePlayerStatus(programId: PublicKey, player: PublicKey) {
  const [playerStatusPda] = PublicKey.findProgramAddressSync(
    [PLAYER_STATUS_SEED, player.toBytes()],
    programId
  );
  return playerStatusPda;
}

function derivePlayerLiveState(programId: PublicKey, player: PublicKey) {
  const [playerLiveStatePda] = PublicKey.findProgramAddressSync(
    [PLAYER_LIVE_STATE_SEED, player.toBytes()],
    programId
  );
  return playerLiveStatePda;
}

function derivePlayerProfile(programId: PublicKey, player: PublicKey) {
  const [playerProfilePda] = PublicKey.findProgramAddressSync(
    [PLAYER_PROFILE_SEED, player.toBytes()],
    programId
  );
  return playerProfilePda;
}

function parseRankedConfig(data: Buffer | null): RankedConfigState | null {
  if (!data || data.length < 8 + 32 + 32 + 32 + 8 + 8 + 8 + 1 + 1) {
    return null;
  }

  const rewardMintOffset = 8 + 32;
  const treasuryOffset = rewardMintOffset + 32;
  const rewardMultiplierOffset = treasuryOffset + 32;
  const penaltyDivisorOffset = rewardMultiplierOffset + 8;
  const penaltyThresholdOffset = penaltyDivisorOffset + 8;
  const mintAuthorityBumpOffset = penaltyThresholdOffset + 8;

  return {
    rewardMint: new PublicKey(data.subarray(rewardMintOffset, rewardMintOffset + 32)),
    treasuryTokenAccount: new PublicKey(data.subarray(treasuryOffset, treasuryOffset + 32)),
    rewardMultiplier: data.readBigUInt64LE(rewardMultiplierOffset),
    penaltyDivisor: data.readBigUInt64LE(penaltyDivisorOffset),
    penaltyThreshold: data.readBigUInt64LE(penaltyThresholdOffset),
    mintAuthorityBump: data[mintAuthorityBumpOffset] ?? 0,
  };
}

async function readTokenBalance(ownerAta: PublicKey) {
  try {
    const account = await getAccount(getConnection(), ownerAta);
    return Number(account.amount);
  } catch (error) {
    if (error instanceof TokenAccountNotFoundError) {
      return 0;
    }
    throw error;
  }
}

async function ensurePlayerAta(params: {
  payer: ReturnType<typeof loadKeypair>;
  owner: PublicKey;
  mint: PublicKey;
}) {
  const ata = getAssociatedTokenAddressSync(params.mint, params.owner);
  const accountInfo = await getConnection().getAccountInfo(ata, 'confirmed');
  if (accountInfo) {
    return ata;
  }

  const ix = createAssociatedTokenAccountInstruction(
    params.payer.publicKey,
    ata,
    params.owner,
    params.mint,
    TOKEN_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID
  );

  await sendInstruction({
    payer: params.payer,
    instruction: ix,
  });

  return ata;
}

function encodeU16(value: number) {
  const buffer = Buffer.alloc(2);
  buffer.writeUInt16LE(Math.max(0, Math.trunc(value)), 0);
  return Uint8Array.from(buffer);
}

function encodeU32(value: number) {
  const buffer = Buffer.alloc(4);
  buffer.writeUInt32LE(Math.max(0, Math.trunc(value)), 0);
  return Uint8Array.from(buffer);
}

async function main() {
  const connection = getConnection();
  const rankedProgramId = resolveGuessrProgramId();
  const player = loadKeypair(
    process.env.RANKED_PLAYER_KEYPAIR || requireEnv('USER_A_KEYPAIR')
  );
  const score = Number(process.env.RANKED_TEST_SCORE || '500');
  const challengeId = process.env.RANKED_CHALLENGE_ID || `ranked-${Date.now()}`;
  const challengeHash = toFixedBytes32(challengeId);

  await ensureMinimumBalance({ connection, wallet: player, minSol: 0.05 });

  const rankedConfigPda = deriveRankedConfig(rankedProgramId);
  const mintAuthorityPda = deriveMintAuthority(rankedProgramId);
  const rankedConfigAccount = await connection.getAccountInfo(rankedConfigPda, 'confirmed');
  const config = parseRankedConfig(rankedConfigAccount?.data ?? null);

  if (!config) {
    throw new Error('Ranked config not initialized. Run step 05 first.');
  }

  const rankedRoomPda = deriveRankedRoom(rankedProgramId, player.publicKey, challengeHash);
  const lobbyPda = deriveLobby(rankedProgramId);
  const playerStatusPda = derivePlayerStatus(rankedProgramId, player.publicKey);
  const playerLiveStatePda = derivePlayerLiveState(rankedProgramId, player.publicKey);
  const playerProfilePda = derivePlayerProfile(rankedProgramId, player.publicKey);
  const playerRewardsPda = derivePlayerRewards(rankedProgramId, player.publicKey);
  const leaderboardPda = deriveLeaderboard(rankedProgramId);
  const playerAta = await ensurePlayerAta({
    payer: player,
    owner: player.publicKey,
    mint: config.rewardMint,
  });

  const balanceBefore = await readTokenBalance(playerAta);

  const joinLobbyIx = new TransactionInstruction({
    programId: rankedProgramId,
    keys: [
      { pubkey: player.publicKey, isSigner: true, isWritable: true },
      { pubkey: lobbyPda, isSigner: false, isWritable: true },
      { pubkey: playerStatusPda, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: concatBinary([
      anchorDiscriminator('join_lobby'),
      player.publicKey.toBytes(),
      player.publicKey.toBytes(),
    ]),
  });
  const joinTx = await sendInstruction({
    payer: player,
    instruction: joinLobbyIx,
  });

  const openIx = new TransactionInstruction({
    programId: rankedProgramId,
    keys: [
      { pubkey: player.publicKey, isSigner: true, isWritable: true },
      { pubkey: rankedRoomPda, isSigner: false, isWritable: true },
      { pubkey: playerStatusPda, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: concatBinary([
      anchorDiscriminator('open_ranked_room'),
      player.publicKey.toBytes(),
      challengeHash,
    ]),
  });
  const openTx = await sendInstruction({
    payer: player,
    instruction: openIx,
  });

  const hintHash = toFixedBytes32(`${challengeId}:hint:1`);
  const hintIx = new TransactionInstruction({
    programId: rankedProgramId,
    keys: [
      { pubkey: player.publicKey, isSigner: true, isWritable: true },
      { pubkey: playerStatusPda, isSigner: false, isWritable: true },
      { pubkey: playerLiveStatePda, isSigner: false, isWritable: true },
      { pubkey: rankedRoomPda, isSigner: false, isWritable: true },
      { pubkey: rankedConfigPda, isSigner: false, isWritable: false },
      { pubkey: config.rewardMint, isSigner: false, isWritable: true },
      { pubkey: mintAuthorityPda, isSigner: false, isWritable: false },
      { pubkey: playerAta, isSigner: false, isWritable: true },
      { pubkey: config.treasuryTokenAccount, isSigner: false, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: concatBinary([
      anchorDiscriminator('update_ranked_state'),
      player.publicKey.toBytes(),
      encodeU16(0),
      encodeU16(100),
      encodeU32(0),
      encodeU16(0),
      Uint8Array.from([ACTION_HINT_OPEN]),
      Uint8Array.from([1]),
      encodeU64(0),
      hintHash,
    ]),
  });
  const hintTx = await sendInstruction({
    payer: player,
    instruction: hintIx,
  });

  const moveHash = toFixedBytes32(`${challengeId}:move:1`);
  const moveIx = new TransactionInstruction({
    programId: rankedProgramId,
    keys: [
      { pubkey: player.publicKey, isSigner: true, isWritable: true },
      { pubkey: playerStatusPda, isSigner: false, isWritable: true },
      { pubkey: playerLiveStatePda, isSigner: false, isWritable: true },
      { pubkey: rankedRoomPda, isSigner: false, isWritable: true },
      { pubkey: rankedConfigPda, isSigner: false, isWritable: false },
      { pubkey: config.rewardMint, isSigner: false, isWritable: true },
      { pubkey: mintAuthorityPda, isSigner: false, isWritable: false },
      { pubkey: playerAta, isSigner: false, isWritable: true },
      { pubkey: config.treasuryTokenAccount, isSigner: false, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: concatBinary([
      anchorDiscriminator('update_ranked_state'),
      player.publicKey.toBytes(),
      encodeU16(0),
      encodeU16(96),
      encodeU32(530),
      encodeU16(7800),
      Uint8Array.from([ACTION_MARK_MOVE]),
      Uint8Array.from([1]),
      encodeU64(120),
      moveHash,
    ]),
  });
  const moveTx = await sendInstruction({
    payer: player,
    instruction: moveIx,
  });

  const guessHash = toFixedBytes32(`${challengeId}:guess:1`);
  const guessIx = new TransactionInstruction({
    programId: rankedProgramId,
    keys: [
      { pubkey: player.publicKey, isSigner: true, isWritable: true },
      { pubkey: playerStatusPda, isSigner: false, isWritable: true },
      { pubkey: playerLiveStatePda, isSigner: false, isWritable: true },
      { pubkey: rankedRoomPda, isSigner: false, isWritable: true },
      { pubkey: rankedConfigPda, isSigner: false, isWritable: false },
      { pubkey: config.rewardMint, isSigner: false, isWritable: true },
      { pubkey: mintAuthorityPda, isSigner: false, isWritable: false },
      { pubkey: playerAta, isSigner: false, isWritable: true },
      { pubkey: config.treasuryTokenAccount, isSigner: false, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: concatBinary([
      anchorDiscriminator('update_ranked_state'),
      player.publicKey.toBytes(),
      encodeU16(0),
      encodeU16(92),
      encodeU32(180),
      encodeU16(9200),
      Uint8Array.from([ACTION_GUESS_SUBMIT]),
      Uint8Array.from([1]),
      encodeU64(score),
      guessHash,
    ]),
  });
  const guessTx = await sendInstruction({
    payer: player,
    instruction: guessIx,
  });

  const settleIx = new TransactionInstruction({
    programId: rankedProgramId,
    keys: [
      { pubkey: player.publicKey, isSigner: true, isWritable: true },
      { pubkey: rankedRoomPda, isSigner: false, isWritable: true },
      { pubkey: playerStatusPda, isSigner: false, isWritable: false },
      { pubkey: playerProfilePda, isSigner: false, isWritable: true },
      { pubkey: playerRewardsPda, isSigner: false, isWritable: true },
      { pubkey: leaderboardPda, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: concatBinary([
      anchorDiscriminator('settle_ranked_room'),
      player.publicKey.toBytes(),
      encodeU64(score),
    ]),
  });
  const settleTx = await sendInstruction({
    payer: player,
    instruction: settleIx,
  });

  const closeIx = new TransactionInstruction({
    programId: rankedProgramId,
    keys: [
      { pubkey: player.publicKey, isSigner: true, isWritable: true },
      { pubkey: playerStatusPda, isSigner: false, isWritable: true },
      { pubkey: rankedRoomPda, isSigner: false, isWritable: true },
    ],
    data: concatBinary([anchorDiscriminator('close_ranked_room'), player.publicKey.toBytes()]),
  });
  const closeTx = await sendInstruction({
    payer: player,
    instruction: closeIx,
  });

  const balanceAfter = await readTokenBalance(playerAta);
  const mintedEstimate =
    config.rewardMultiplier > 0n
      ? Number(BigInt(Math.max(score, 0)) / config.rewardMultiplier)
      : 0;

  let penaltyEstimate = 0;
  const scoreBig = BigInt(Math.max(score, 0));
  if (scoreBig < config.penaltyThreshold && config.penaltyDivisor > 0n) {
    penaltyEstimate = Number((config.penaltyThreshold - scoreBig) / config.penaltyDivisor);
  }

  console.log('--- Ranked solo simulation complete ---');
  console.log('Challenge ID:', challengeId);
  console.log('Ranked room PDA:', rankedRoomPda.toBase58());
  console.log('Player ATA:', playerAta.toBase58());
  console.log('Score submitted:', score);
  console.log('Join lobby tx:', joinTx);
  console.log('Open tx:', openTx);
  console.log('Hint update tx:', hintTx);
  console.log('Movement update tx:', moveTx);
  console.log('Guess update tx:', guessTx);
  console.log('Settle tx:', settleTx);
  console.log('Close tx:', closeTx);
  console.log('Balance before:', balanceBefore);
  console.log('Balance after:', balanceAfter);
  console.log('Balance delta:', balanceAfter - balanceBefore);
  console.log('Estimated minted:', mintedEstimate);
  console.log('Estimated penalty:', penaltyEstimate);
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
