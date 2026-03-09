import { PublicKey, TransactionInstruction } from '@solana/web3.js';
import {
  anchorDiscriminator,
  concatBinary,
  getConnection,
  loadKeypair,
  loadUserKeypairFromEnv,
  requireEnv,
  resolveGuessrProgramId,
  sendInstruction,
  toFixedBytes32,
  writeReport,
} from './_shared';

const MAGIC_PROGRAM_ID = new PublicKey('Magic11111111111111111111111111111111111111');
const MAGIC_CONTEXT_ID = new PublicKey('MagicContext1111111111111111111111111111111');

const LOBBY_STATE_SEED = new TextEncoder().encode('lobby-state');
const RANKED_CONFIG_SEED = new TextEncoder().encode('ranked-config');
const LEADERBOARD_SEED = new TextEncoder().encode('leaderboard');
const PLAYER_STATUS_SEED = new TextEncoder().encode('player-status');
const PLAYER_LIVE_STATE_SEED = new TextEncoder().encode('player-live-state');
const PLAYER_PROFILE_SEED = new TextEncoder().encode('player-profile');
const PLAYER_REWARDS_SEED = new TextEncoder().encode('player-rewards');
const ROOM_ID_SEED = new TextEncoder().encode('room-id');
const DUEL_ROOM_SEED = new TextEncoder().encode('duel-room');
const RANKED_ROOM_SEED = new TextEncoder().encode('ranked-room');
const REWARD_CLAIM_SEED = new TextEncoder().encode('reward-claim');

const MATCH_MODE_DUEL = 0;
const MATCH_MODE_RANKED_SOLO = 1;

type CommitSpec = {
  label: string;
  pda: PublicKey;
};

type RewardClaimInput = {
  player: PublicKey;
  matchId: Uint8Array;
  mode: number;
};

function parseCsv(value: string | undefined) {
  if (!value) {
    return [];
  }

  return value
    .split(',')
    .map(item => item.trim())
    .filter(Boolean);
}

function tryParsePublicKey(value: string) {
  try {
    return new PublicKey(value);
  } catch {
    return null;
  }
}

function parseRewardClaimInputs(value: string | undefined) {
  const entries = parseCsv(value);
  const claims: RewardClaimInput[] = [];

  for (const entry of entries) {
    const [playerRaw, matchRaw, modeRaw] = entry.split(':').map(valuePart => valuePart.trim());
    if (!playerRaw || !matchRaw || !modeRaw) {
      throw new Error(
        `Invalid DELEGATE_REWARD_CLAIMS entry "${entry}". Expected format: wallet:matchId:mode`
      );
    }

    const player = tryParsePublicKey(playerRaw);
    if (!player) {
      throw new Error(`Invalid wallet in DELEGATE_REWARD_CLAIMS entry "${entry}"`);
    }

    const mode = Number(modeRaw);
    if (mode !== MATCH_MODE_DUEL && mode !== MATCH_MODE_RANKED_SOLO) {
      throw new Error(
        `Invalid mode in DELEGATE_REWARD_CLAIMS entry "${entry}". Use 0 (duel) or 1 (ranked).`
      );
    }

    const parsedMatch = tryParsePublicKey(matchRaw);
    claims.push({
      player,
      matchId: parsedMatch ? parsedMatch.toBytes() : toFixedBytes32(matchRaw),
      mode,
    });
  }

  return claims;
}

function collectPlayerWallets(payer: PublicKey) {
  const byBase58 = new Set<string>();
  const players: PublicKey[] = [];

  const add = (player: PublicKey) => {
    const address = player.toBase58();
    if (byBase58.has(address)) {
      return;
    }
    byBase58.add(address);
    players.push(player);
  };

  add(payer);

  for (const raw of parseCsv(process.env.DELEGATE_PLAYER_WALLETS)) {
    const parsed = tryParsePublicKey(raw);
    if (!parsed) {
      throw new Error(`Invalid public key in DELEGATE_PLAYER_WALLETS: ${raw}`);
    }
    add(parsed);
  }

  for (const prefix of ['USER_A', 'USER_B', 'USER_C', 'RANKED_PLAYER']) {
    try {
      add(loadUserKeypairFromEnv(prefix).publicKey);
    } catch {
      // Ignore unset env vars; this script supports partial inputs.
    }
  }

  return players;
}

function addSpec(store: Map<string, CommitSpec>, spec: CommitSpec) {
  const key = spec.pda.toBase58();
  if (!store.has(key)) {
    store.set(key, spec);
  }
}

function parseRoomIdBytes(roomIdOrAddress: string, programId: PublicKey) {
  const parsed = tryParsePublicKey(roomIdOrAddress);
  if (parsed) {
    return parsed.toBytes();
  }

  const [derivedRoomAddress] = PublicKey.findProgramAddressSync(
    [ROOM_ID_SEED, toFixedBytes32(roomIdOrAddress)],
    programId
  );
  return derivedRoomAddress.toBytes();
}

function buildCommitInstruction(params: {
  programId: PublicKey;
  payer: ReturnType<typeof loadKeypair>;
  lobbyStatePda: PublicKey;
  rankedConfigPda: PublicKey;
  leaderboardPda: PublicKey;
  extraTargets: CommitSpec[];
}) {
  const keys = [
    { pubkey: params.payer.publicKey, isSigner: true, isWritable: true },
    { pubkey: params.lobbyStatePda, isSigner: false, isWritable: true },
    { pubkey: params.rankedConfigPda, isSigner: false, isWritable: true },
    { pubkey: params.leaderboardPda, isSigner: false, isWritable: true },
    { pubkey: MAGIC_PROGRAM_ID, isSigner: false, isWritable: false },
    { pubkey: MAGIC_CONTEXT_ID, isSigner: false, isWritable: true },
    ...params.extraTargets.map(target => ({
      pubkey: target.pda,
      isSigner: false,
      isWritable: true,
    })),
  ];

  return new TransactionInstruction({
    programId: params.programId,
    keys,
    data: concatBinary([anchorDiscriminator('commit_guessr_state')]),
  });
}

function formatError(error: unknown) {
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
}

function isLikelyTransactionSizeError(error: unknown) {
  const text = formatError(error).toLowerCase();
  return (
    text.includes('transaction too large') ||
    text.includes('too many account keys') ||
    text.includes('encoding overruns') ||
    text.includes('message too large')
  );
}

async function sendCommitWithFallback(params: {
  connection: ReturnType<typeof getConnection>;
  payer: ReturnType<typeof loadKeypair>;
  programId: PublicKey;
  lobbyStatePda: PublicKey;
  rankedConfigPda: PublicKey;
  leaderboardPda: PublicKey;
  extraTargets: CommitSpec[];
}): Promise<{ signatures: string[]; usedSingleTx: boolean }> {
  try {
    const signature = await sendInstruction({
      connection: params.connection,
      payer: params.payer,
      instruction: buildCommitInstruction(params),
    });
    return { signatures: [signature], usedSingleTx: true };
  } catch (error) {
    if (!isLikelyTransactionSizeError(error) || params.extraTargets.length <= 1) {
      throw error;
    }
  }

  const signatures = await sendCommitChunked(params);
  return { signatures, usedSingleTx: false };
}

async function sendCommitChunked(params: {
  connection: ReturnType<typeof getConnection>;
  payer: ReturnType<typeof loadKeypair>;
  programId: PublicKey;
  lobbyStatePda: PublicKey;
  rankedConfigPda: PublicKey;
  leaderboardPda: PublicKey;
  extraTargets: CommitSpec[];
}): Promise<string[]> {
  try {
    const signature = await sendInstruction({
      connection: params.connection,
      payer: params.payer,
      instruction: buildCommitInstruction(params),
    });
    return [signature];
  } catch (error) {
    if (!isLikelyTransactionSizeError(error) || params.extraTargets.length <= 1) {
      throw error;
    }
  }

  const midpoint = Math.ceil(params.extraTargets.length / 2);
  const left = params.extraTargets.slice(0, midpoint);
  const right = params.extraTargets.slice(midpoint);
  const leftSignatures = await sendCommitChunked({ ...params, extraTargets: left });
  const rightSignatures = await sendCommitChunked({ ...params, extraTargets: right });
  return [...leftSignatures, ...rightSignatures];
}

async function main() {
  const connection = getConnection();
  const payer = loadKeypair(requireEnv('SOLANA_PAYER_KEYPAIR'));
  const programId = resolveGuessrProgramId();
  const players = collectPlayerWallets(payer.publicKey);
  const specs = new Map<string, CommitSpec>();

  const [lobbyStatePda] = PublicKey.findProgramAddressSync([LOBBY_STATE_SEED], programId);
  const [rankedConfigPda] = PublicKey.findProgramAddressSync([RANKED_CONFIG_SEED], programId);
  const [leaderboardPda] = PublicKey.findProgramAddressSync([LEADERBOARD_SEED], programId);

  const lobbyInfo = await connection.getAccountInfo(lobbyStatePda, 'confirmed');
  const rankedInfo = await connection.getAccountInfo(rankedConfigPda, 'confirmed');
  const leaderboardInfo = await connection.getAccountInfo(leaderboardPda, 'confirmed');
  if (!lobbyInfo) {
    throw new Error(`Required PDA not found: lobby-state (${lobbyStatePda.toBase58()})`);
  }
  if (!rankedInfo) {
    throw new Error(`Required PDA not found: ranked-config (${rankedConfigPda.toBase58()})`);
  }
  if (!leaderboardInfo) {
    throw new Error(`Required PDA not found: leaderboard (${leaderboardPda.toBase58()})`);
  }

  for (const player of players) {
    const [playerStatusPda] = PublicKey.findProgramAddressSync(
      [PLAYER_STATUS_SEED, player.toBytes()],
      programId
    );
    const [playerLiveStatePda] = PublicKey.findProgramAddressSync(
      [PLAYER_LIVE_STATE_SEED, player.toBytes()],
      programId
    );
    const [playerProfilePda] = PublicKey.findProgramAddressSync(
      [PLAYER_PROFILE_SEED, player.toBytes()],
      programId
    );
    const [playerRewardsPda] = PublicKey.findProgramAddressSync(
      [PLAYER_REWARDS_SEED, player.toBytes()],
      programId
    );

    addSpec(specs, {
      label: `player-status:${player.toBase58()}`,
      pda: playerStatusPda,
    });
    addSpec(specs, {
      label: `player-live-state:${player.toBase58()}`,
      pda: playerLiveStatePda,
    });
    addSpec(specs, {
      label: `player-profile:${player.toBase58()}`,
      pda: playerProfilePda,
    });
    addSpec(specs, {
      label: `player-rewards:${player.toBase58()}`,
      pda: playerRewardsPda,
    });
  }

  const duelRoomIds = [
    ...parseCsv(process.env.DELEGATE_DUEL_ROOM_IDS),
    ...parseCsv(process.env.TEST_ROOM_ID),
    ...parseCsv(process.env.TEST_DUEL_ROOM_ID),
  ];

  for (const roomIdOrAddress of duelRoomIds) {
    const roomIdBytes = parseRoomIdBytes(roomIdOrAddress, programId);
    const [duelRoomPda] = PublicKey.findProgramAddressSync(
      [DUEL_ROOM_SEED, roomIdBytes],
      programId
    );
    addSpec(specs, {
      label: `duel-room:${roomIdOrAddress}`,
      pda: duelRoomPda,
    });
  }

  const rankedChallenges = [
    ...parseCsv(process.env.DELEGATE_RANKED_CHALLENGES),
    ...parseCsv(process.env.RANKED_CHALLENGE_ID),
  ];
  for (const player of players) {
    for (const challengeId of rankedChallenges) {
      const challengeHash = toFixedBytes32(challengeId);
      const [rankedRoomPda] = PublicKey.findProgramAddressSync(
        [RANKED_ROOM_SEED, player.toBytes(), challengeHash],
        programId
      );
      addSpec(specs, {
        label: `ranked-room:${player.toBase58()}:${challengeId}`,
        pda: rankedRoomPda,
      });
    }
  }

  const rewardClaims = parseRewardClaimInputs(process.env.DELEGATE_REWARD_CLAIMS);
  for (const claim of rewardClaims) {
    const [rewardClaimPda] = PublicKey.findProgramAddressSync(
      [REWARD_CLAIM_SEED, claim.player.toBytes(), claim.matchId, Uint8Array.from([claim.mode])],
      programId
    );
    addSpec(specs, {
      label: `reward-claim:${claim.player.toBase58()}:${claim.mode}`,
      pda: rewardClaimPda,
    });
  }

  const planned = Array.from(specs.values());
  const accountInfos = await Promise.all(
    planned.map(async spec => ({
      spec,
      account: await connection.getAccountInfo(spec.pda, 'confirmed'),
    }))
  );

  const extraTargets: CommitSpec[] = [];
  const skipped: string[] = [];
  for (const item of accountInfos) {
    if (item.account) {
      extraTargets.push(item.spec);
    } else {
      skipped.push(item.spec.label);
    }
  }

  const result = await sendCommitWithFallback({
    connection,
    payer,
    programId,
    lobbyStatePda,
    rankedConfigPda,
    leaderboardPda,
    extraTargets,
  });

  console.log('Program:', programId.toBase58());
  console.log('Committed global PDAs: 3');
  console.log('Committed additional PDAs:', extraTargets.length);
  console.log('Skipped missing PDAs:', skipped.length);
  if (result.usedSingleTx) {
    console.log('Commit completed in one transaction');
  } else {
    console.log(`Commit required ${result.signatures.length} transactions`);
  }
  for (const [index, signature] of result.signatures.entries()) {
    console.log(`Signature ${index + 1}:`, signature);
  }

  writeReport(
    '05_commit_guessr_state.log',
    `program=${programId.toBase58()} committedGlobal=3 committedExtra=${extraTargets.length} skipped=${skipped.length} singleTx=${result.usedSingleTx} signatures=${result.signatures.join(',')}`
  );
}

main().catch(error => {
  console.error(error);
  process.exit(1);
});
