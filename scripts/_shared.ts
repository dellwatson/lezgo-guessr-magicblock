import { Buffer } from 'buffer';
import { createHash } from 'crypto';
import { readFileSync, writeFileSync, existsSync, mkdirSync } from 'fs';
import path from 'path';
import bs58 from 'bs58';
import {
  Connection,
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  Transaction,
  TransactionInstruction,
  sendAndConfirmTransaction,
} from '@solana/web3.js';

export function loadKeypair(keypairPath: string) {
  const raw = JSON.parse(readFileSync(keypairPath, 'utf8')) as number[];
  return Keypair.fromSecretKey(Uint8Array.from(raw));
}

export function loadKeypairFromPrivateKeyEnv(envName: string) {
  const value = process.env[envName];
  if (!value) {
    throw new Error(`Missing env var: ${envName}`);
  }
  const secret = bs58.decode(value);
  return Keypair.fromSecretKey(secret);
}

export function loadUserKeypairFromEnv(prefix: string) {
  const pvtKeyEnv = `${prefix}_PVTKEY`;
  const keypairEnv = `${prefix}_KEYPAIR`;
  if (process.env[pvtKeyEnv]) {
    return loadKeypairFromPrivateKeyEnv(pvtKeyEnv);
  }
  const keypairPath = process.env[keypairEnv];
  if (keypairPath) {
    return loadKeypair(keypairPath);
  }
  throw new Error(`Missing env vars: ${pvtKeyEnv} or ${keypairEnv}`);
}

function getReportsDir() {
  const dir = path.resolve(__dirname, '..', 'reports');
  if (!existsSync(dir)) {
    mkdirSync(dir, { recursive: true });
  }
  return dir;
}

export function writeReport(name: string, content: string) {
  const dir = getReportsDir();
  const file = path.join(dir, name);
  writeFileSync(file, `${content}\n`, { flag: 'a' });
}

export function requireEnv(name: string) {
  const value = process.env[name];
  if (!value) {
    throw new Error(`Missing env var: ${name}`);
  }
  return value;
}

export function resolveGuessrProgramId() {
  const value = process.env.GUESSR_PROGRAM_ID || process.env.MULTIPLAYER_PROGRAM_ID;

  if (!value) {
    throw new Error('Missing env var: GUESSR_PROGRAM_ID (fallback: MULTIPLAYER_PROGRAM_ID)');
  }

  return parsePublicKey(value);
}

export function getConnection() {
  const rpcUrl = process.env.SOLANA_RPC_URL || 'https://api.devnet.solana.com';
  return new Connection(rpcUrl, 'confirmed');
}

export function anchorDiscriminator(method: string) {
  const hash = createHash('sha256').update(`global:${method}`).digest();
  return Uint8Array.from(hash.subarray(0, 8));
}

export function encodeI64(value: number) {
  const buffer = Buffer.alloc(8);
  buffer.writeBigInt64LE(BigInt(value), 0);
  return Uint8Array.from(buffer);
}

export function encodeU64(value: number) {
  const buffer = Buffer.alloc(8);
  buffer.writeBigUInt64LE(BigInt(value), 0);
  return Uint8Array.from(buffer);
}

export function toFixedBytes32(value: string) {
  const bytes = new Uint8Array(32);
  const source = new TextEncoder().encode(value);
  bytes.set(source.subarray(0, 32));
  return bytes;
}

export function concatBinary(parts: Uint8Array[]) {
  const total = parts.reduce((sum, part) => sum + part.length, 0);
  const merged = new Uint8Array(total);
  let offset = 0;

  for (const part of parts) {
    merged.set(part, offset);
    offset += part.length;
  }

  return Buffer.from(merged);
}

export async function sendInstructions(params: {
  connection?: Connection;
  payer: Keypair;
  instructions: TransactionInstruction[];
  signers?: Keypair[];
}) {
  const connection = params.connection ?? getConnection();
  const tx = new Transaction().add(...params.instructions);
  const signerMap = new Map<string, Keypair>();
  for (const signer of [params.payer, ...(params.signers ?? [])]) {
    signerMap.set(signer.publicKey.toBase58(), signer);
  }

  return sendAndConfirmTransaction(connection, tx, Array.from(signerMap.values()), {
    commitment: 'confirmed',
  });
}

export async function sendInstruction(params: {
  connection?: Connection;
  payer: Keypair;
  instruction: TransactionInstruction;
  signers?: Keypair[];
}) {
  return sendInstructions({
    connection: params.connection,
    payer: params.payer,
    instructions: [params.instruction],
    signers: params.signers,
  });
}

export async function ensureMinimumBalance(params: {
  connection: Connection;
  wallet: Keypair;
  minSol: number;
}) {
  const minLamports = Math.floor(params.minSol * LAMPORTS_PER_SOL);
  const current = await params.connection.getBalance(params.wallet.publicKey, 'confirmed');
  if (current >= minLamports) {
    return;
  }

  const shortfall = minLamports - current;
  const signature = await params.connection.requestAirdrop(params.wallet.publicKey, shortfall);
  await params.connection.confirmTransaction(signature, 'confirmed');
}

export function parsePublicKey(value: string) {
  return new PublicKey(value);
}
