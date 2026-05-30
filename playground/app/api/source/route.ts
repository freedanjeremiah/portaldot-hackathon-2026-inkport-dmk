import { NextRequest, NextResponse } from 'next/server';
import fs from 'fs';
import path from 'path';
import { buildEnv } from '@/lib/env';

/**
 * GET /api/source?contract=ERC20
 *
 * Returns the ACTUAL local Solidity source for a repo contract, read from
 * $INKPORT_ROOT/contracts/<name>.sol — so the editor loads exactly what
 * compiles. The contract name is sanitized to alphanumerics only to prevent
 * path traversal (no slashes, dots, or `..` survive).
 */
export async function GET(request: NextRequest) {
  const raw = request.nextUrl.searchParams.get('contract') ?? '';
  // Strip anything that isn't [A-Za-z0-9]; defeats path traversal entirely.
  const name = raw.replace(/[^A-Za-z0-9]/g, '');

  if (!name) {
    return NextResponse.json({ error: 'Missing or invalid contract name' }, { status: 400 });
  }

  const env = buildEnv();
  const inkportRoot = env.INKPORT_ROOT as string;
  if (!inkportRoot) {
    return NextResponse.json({ error: 'INKPORT_ROOT not configured' }, { status: 500 });
  }

  const contractsDir = path.join(inkportRoot, 'contracts');
  const solFile = path.join(contractsDir, `${name}.sol`);

  // Defense-in-depth: ensure the resolved path is still inside contracts/.
  const resolved = path.resolve(solFile);
  if (!resolved.startsWith(path.resolve(contractsDir) + path.sep)) {
    return NextResponse.json({ error: 'Invalid contract path' }, { status: 400 });
  }

  let source: string;
  try {
    source = fs.readFileSync(resolved, 'utf8');
  } catch {
    return NextResponse.json({ error: `Contract "${name}" not found` }, { status: 404 });
  }

  return NextResponse.json({ name, source });
}
