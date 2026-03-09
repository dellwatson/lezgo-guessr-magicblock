import { PublicKey, SystemProgram, TransactionInstruction } from '@solana/web3.js';
import {
  anchorDiscriminator,
  concatBinary,
  getConnection,
  loadKeypair,
  loadUserKeypairFromEnv,
  requireEnv,
  resolveGuessrProgramId,
  sendInstructions,
  toFixedBytes32,
  writeReport,
} from './_shared';

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
const DELEGATE_BUFFER_TAG = new TextEncoder().encode('buffer');
const DELEGATION_RECORD_TAG = new TextEncoder().encode('delegation');
const DELEGATION_METADATA_TAG = new TextEncoder().encode('delegation-metadata');
const DEFAULT_MAGICBLOCK_VALIDATOR = 'MUS3hc9TCw4cGC12vHNoYcCGzJG1txjgQLZWVoenHNd';
const DEFAULT_DELEGATION_PROGRAM_ID = 'DELeGGvXpWV2fqJUhqcF5ZSYMS4JTLjteaAMARRSaeSh';

const DELEGATE_TARGET_LOBBY_STATE = 0;
const DELEGATE_TARGET_RANKED_CONFIG = 1;
const DELEGATE_TARGET_PLAYER_STATUS = 2;
const DELEGATE_TARGET_PLAYER_LIVE_STATE = 3;
const DELEGATE_TARGET_PLAYER_PROFILE = 4;
const DELEGATE_TARGET_DUEL_ROOM = 5;
const DELEGATE_TARGET_RANKED_ROOM = 6;
const DELEGATE_TARGET_REWARD_CLAIM = 7;
const DELEGATE_TARGET_LEADERBOARD = 8;
const DELEGATE_TARGET_PLAYER_REWARDS = 9;

const MATCH_MODE_DUEL = 0;
const MATCH_MODE_RANKED_SOLO = 1;

type DelegationTarget =
  | typeof DELEGATE_TARGET_LOBBY_STATE
  | typeof DELEGATE_TARGET_RANKED_CONFIG
  | typeof DELEGATE_TARGET_PLAYER_STATUS
  | typeof DELEGATE_TARGET_PLAYER_LIVE_STATE
  | typeof DELEGATE_TARGET_PLAYER_PROFILE
  | typeof DELEGATE_TARGET_DUEL_ROOM
  | typeof DELEGATE_TARGET_RANKED_ROOM
  | typeof DELEGATE_TARGET_REWARD_CLAIM
  | typeof DELEGATE_TARGET_LEADERBOARD
  | typeof DELEGATE_TARGET_PLAYER_REWARDS;

type DelegationSpec = {
  label: string;
  target: DelegationTarget;
  pda: PublicKey;
  player?: PublicKey;
  roomOrMatchId?: Uint8Array;
  mode?: number;
  required?: boolean;
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

function resolveMagicblockValidator() {
  const raw = process.env.MAGICBLOCK_VALIDATOR;
  if (raw && raw.trim().length > 0) {
    const parsed = tryParsePublicKey(raw.trim());
    if (!parsed) {
      throw new Error(`Invalid MAGICBLOCK_VALIDATOR: ${raw}`);
    }
    return parsed;
  }

  return new PublicKey(DEFAULT_MAGICBLOCK_VALIDATOR);
}

function resolveDelegationProgramId() {
  const raw = process.env.DELEGATION_PROGRAM_ID;
  if (raw && raw.trim().length > 0) {
    const parsed = tryParsePublicKey(raw.trim());
    if (!parsed) {
      throw new Error(`Invalid DELEGATION_PROGRAM_ID: ${raw}`);
    }
    return parsed;
  }

  return new PublicKey(DEFAULT_DELEGATION_PROGRAM_ID);
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

function addSpec(store: Map<string, DelegationSpec>, spec: DelegationSpec) {
  const key = `${spec.target}:${spec.pda.toBase58()}`;
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

function buildDelegateInstruction(params: {
  programId: PublicKey;
  payer: PublicKey;
  spec: DelegationSpec;
  validator: PublicKey;
  delegationProgramId: PublicKey;
}) {
  const player = params.spec.player ?? PublicKey.default;
  const roomOrMatchId = params.spec.roomOrMatchId ?? new Uint8Array(32);
  const mode = params.spec.mode ?? 0;
  const [bufferPda] = PublicKey.findProgramAddressSync(
    [DELEGATE_BUFFER_TAG, params.spec.pda.toBytes()],
    params.programId
  );
  const [delegationRecordPda] = PublicKey.findProgramAddressSync(
    [DELEGATION_RECORD_TAG, params.spec.pda.toBytes()],
    params.delegationProgramId
  );
  const [delegationMetadataPda] = PublicKey.findProgramAddressSync(
    [DELEGATION_METADATA_TAG, params.spec.pda.toBytes()],
    params.delegationProgramId
  );

  return new TransactionInstruction({
    programId: params.programId,
    keys: [
      { pubkey: params.payer, isSigner: true, isWritable: true },
      { pubkey: bufferPda, isSigner: false, isWritable: true },
      { pubkey: delegationRecordPda, isSigner: false, isWritable: true },
      { pubkey: delegationMetadataPda, isSigner: false, isWritable: true },
      { pubkey: params.spec.pda, isSigner: false, isWritable: true },
      { pubkey: params.programId, isSigner: false, isWritable: false },
      { pubkey: params.delegationProgramId, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      { pubkey: params.validator, isSigner: false, isWritable: false },
    ],
    data: concatBinary([
      anchorDiscriminator('delegate_guessr_state'),
      Uint8Array.from([params.spec.target]),
      player.toBytes(),
      roomOrMatchId,
      Uint8Array.from([mode]),
    ]),
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

async function sendWithFallback(
  instructions: TransactionInstruction[],
  payer: ReturnType<typeof loadKeypair>
): Promise<{ signatures: string[]; usedSingleTx: boolean }> {
  if (instructions.length === 0) {
    return { signatures: [], usedSingleTx: true };
  }

  try {
    const signature = await sendInstructions({ payer, instructions });
    return { signatures: [signature], usedSingleTx: true };
  } catch (error) {
    if (!isLikelyTransactionSizeError(error) || instructions.length === 1) {
      throw error;
    }
  }

  const signatures = await sendChunked(instructions, payer);
  return { signatures, usedSingleTx: false };
}

async function sendChunked(
  instructions: TransactionInstruction[],
  payer: ReturnType<typeof loadKeypair>
): Promise<string[]> {
  if (instructions.length === 0) {
    return [];
  }

  try {
    const signature = await sendInstructions({ payer, instructions });
    return [signature];
  } catch (error) {
    if (!isLikelyTransactionSizeError(error) || instructions.length === 1) {
      throw error;
    }
  }

  const midpoint = Math.ceil(instructions.length / 2);
  const left = instructions.slice(0, midpoint);
  const right = instructions.slice(midpoint);
  const leftSignatures = await sendChunked(left, payer);
  const rightSignatures = await sendChunked(right, payer);

  return [...leftSignatures, ...rightSignatures];
}

async function main() {
  const connection = getConnection();
  const payer = loadKeypair(requireEnv('SOLANA_PAYER_KEYPAIR'));
  const programId = resolveGuessrProgramId();
  const validator = resolveMagicblockValidator();
  const delegationProgramId = resolveDelegationProgramId();
  const players = collectPlayerWallets(payer.publicKey);
  const specs = new Map<string, DelegationSpec>();

  const [lobbyStatePda] = PublicKey.findProgramAddressSync([LOBBY_STATE_SEED], programId);
  const [rankedConfigPda] = PublicKey.findProgramAddressSync([RANKED_CONFIG_SEED], programId);
  const [leaderboardPda] = PublicKey.findProgramAddressSync([LEADERBOARD_SEED], programId);

  addSpec(specs, {
    label: 'lobby-state',
    target: DELEGATE_TARGET_LOBBY_STATE,
    pda: lobbyStatePda,
    required: true,
  });
  addSpec(specs, {
    label: 'ranked-config',
    target: DELEGATE_TARGET_RANKED_CONFIG,
    pda: rankedConfigPda,
    required: true,
  });
  addSpec(specs, {
    label: 'leaderboard',
    target: DELEGATE_TARGET_LEADERBOARD,
    pda: leaderboardPda,
    required: true,
  });

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
      target: DELEGATE_TARGET_PLAYER_STATUS,
      pda: playerStatusPda,
      player,
    });
    addSpec(specs, {
      label: `player-live-state:${player.toBase58()}`,
      target: DELEGATE_TARGET_PLAYER_LIVE_STATE,
      pda: playerLiveStatePda,
      player,
    });
    addSpec(specs, {
      label: `player-profile:${player.toBase58()}`,
      target: DELEGATE_TARGET_PLAYER_PROFILE,
      pda: playerProfilePda,
      player,
    });
    addSpec(specs, {
      label: `player-rewards:${player.toBase58()}`,
      target: DELEGATE_TARGET_PLAYER_REWARDS,
      pda: playerRewardsPda,
      player,
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
      target: DELEGATE_TARGET_DUEL_ROOM,
      pda: duelRoomPda,
      roomOrMatchId: roomIdBytes,
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
        target: DELEGATE_TARGET_RANKED_ROOM,
        pda: rankedRoomPda,
        player,
        roomOrMatchId: challengeHash,
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
      target: DELEGATE_TARGET_REWARD_CLAIM,
      pda: rewardClaimPda,
      player: claim.player,
      roomOrMatchId: claim.matchId,
      mode: claim.mode,
    });
  }

  const planned = Array.from(specs.values());
  const accountInfos = await Promise.all(
    planned.map(async spec => ({
      spec,
      account: await connection.getAccountInfo(spec.pda, 'confirmed'),
    }))
  );

  const toDelegate: DelegationSpec[] = [];
  const skipped: string[] = [];
  const skippedSpecs: DelegationSpec[] = [];

  for (const item of accountInfos) {
    if (item.account) {
      toDelegate.push(item.spec);
      continue;
    }

    if (item.spec.required) {
      throw new Error(`Required PDA not found: ${item.spec.label} (${item.spec.pda.toBase58()})`);
    }

    skipped.push(item.spec.label);
    skippedSpecs.push(item.spec);
  }

  if (toDelegate.length === 0) {
    throw new Error('No existing PDAs found to delegate.');
  }

  const instructions = toDelegate.map(spec =>
    buildDelegateInstruction({
      programId,
      payer: payer.publicKey,
      spec,
      validator,
      delegationProgramId,
    })
  );

  const result = await sendWithFallback(instructions, payer);

  console.log('Program:', programId.toBase58());
  console.log('Delegated PDAs:', toDelegate.length);
  console.log('Skipped missing PDAs:', skipped.length);
  if (result.usedSingleTx) {
    console.log('Delegation completed in one transaction');
  } else {
    console.log(`Delegation required ${result.signatures.length} transactions`);
  }
  for (const [index, signature] of result.signatures.entries()) {
    console.log(`Signature ${index + 1}:`, signature);
  }

  writeReport(
    '04_delegate_guessr_state.log',
    `program=${programId.toBase58()} delegated=${toDelegate.length} skipped=${skipped.length} singleTx=${result.usedSingleTx} signatures=${result.signatures.join(',')}`
  );

  for (const spec of toDelegate) {
    const playerBase58 = spec.player ? spec.player.toBase58() : '';
    writeReport(
      '04_delegate_guessr_state_pdas.log',
      `delegated label=${spec.label} target=${spec.target} pda=${spec.pda.toBase58()} player=${playerBase58}`
    );
  }

  for (const spec of skippedSpecs) {
    const playerBase58 = spec.player ? spec.player.toBase58() : '';
    writeReport(
      '04_delegate_guessr_state_pdas.log',
      `skipped label=${spec.label} target=${spec.target} pda=${spec.pda.toBase58()} player=${playerBase58}`
    );
  }
}

main().catch(error => {
  console.error(error);
  process.exit(1);
});
