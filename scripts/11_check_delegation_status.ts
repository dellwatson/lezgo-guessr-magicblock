import { existsSync, readFileSync } from 'fs';
import path from 'path';
import { writeReport } from './_shared';

interface DelegationRecord {
  authority?: string;
  owner?: string;
  delegationSlot?: number;
  lamports?: number;
}

interface DelegationStatusResult {
  isDelegated?: boolean;
  fqdn?: string;
  delegationRecord?: DelegationRecord;
}

interface DelegationStatusRpcResponse {
  jsonrpc: string;
  id: number;
  result?: DelegationStatusResult;
  error?: { code: number; message: string; data?: unknown };
}

function parseCsv(value: string | undefined) {
  if (!value) return [] as string[];
  return value
    .split(',')
    .map(entry => entry.trim())
    .filter(Boolean);
}

function collectPdasFromEnv() {
  return parseCsv(process.env.DELEGATION_STATUS_PDAS);
}

function collectPdasFromReport() {
  const reportPath = path.resolve(__dirname, '..', 'reports', '04_delegate_guessr_state_pdas.log');
  if (!existsSync(reportPath)) {
    return [] as string[];
  }

  const content = readFileSync(reportPath, 'utf8');
  const result: string[] = [];

  for (const line of content.split('\n')) {
    const trimmed = line.trim();
    if (!trimmed) continue;
    const match = trimmed.match(/pda=([A-Za-z0-9]+)/);
    if (match) {
      result.push(match[1]);
    }
  }

  return result;
}

async function queryDelegationStatus(
  routerUrl: string,
  account: string
): Promise<DelegationStatusResult | undefined> {
  const body = {
    jsonrpc: '2.0',
    id: 1,
    method: 'getDelegationStatus',
    params: [account],
  };

  const response = await fetch(routerUrl, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });

  if (!response.ok) {
    throw new Error(
      `Router request failed for ${account}: ${response.status} ${response.statusText}`
    );
  }

  const json = (await response.json()) as DelegationStatusRpcResponse;
  if (json.error) {
    throw new Error(`Router error for ${account}: ${json.error.code} ${json.error.message}`);
  }

  return json.result;
}

async function main() {
  const routerUrl = process.env.MAGIC_ROUTER_URL || 'https://devnet-router.magicblock.app';

  const pdas = Array.from(new Set([...collectPdasFromEnv(), ...collectPdasFromReport()]));

  if (pdas.length === 0) {
    throw new Error(
      'No PDAs to query. Set DELEGATION_STATUS_PDAS env or run 04_delegate_guessr_state.ts first to populate reports.'
    );
  }

  console.log('Magic Router URL:', routerUrl);
  console.log('PDAs to query:', pdas.length);

  for (const pda of pdas) {
    try {
      const status = await queryDelegationStatus(routerUrl, pda);
      const delegated = !!status?.isDelegated;
      const fqdn = status?.fqdn ?? '';
      const authority = status?.delegationRecord?.authority ?? '';

      console.log(`PDA ${pda}: delegated=${delegated} fqdn=${fqdn} authority=${authority}`);

      writeReport(
        '11_delegation_status.log',
        `pda=${pda} delegated=${delegated} fqdn=${fqdn} authority=${authority}`
      );
    } catch (error) {
      console.error(`Error querying PDA ${pda}:`, error);
      writeReport(
        '11_delegation_status.log',
        `pda=${pda} error=${error instanceof Error ? error.message : String(error)}`
      );
    }
  }
}

main().catch(error => {
  console.error(error);
  process.exit(1);
});
