import { NextRequest, NextResponse } from 'next/server';
import fs from 'fs';
import path from 'path';
import { buildEnv } from '@/lib/env';
import { sessionDir } from '@/lib/session';
import { spawnCollect } from '@/lib/shell';

function parseName(solidity: string): string {
  const stripped = solidity
    .replace(/\/\*[\s\S]*?\*\//g, ' ')  // block comments
    .replace(/\/\/[^\n]*/g, '');         // line comments
  const m = /contract\s+([A-Za-z_]\w*)/.exec(stripped);
  return m ? m[1] : 'Contract';
}

export async function POST(request: NextRequest) {
  let body: { solidity?: string; sessionId?: string };
  try { body = await request.json(); } catch {
    return NextResponse.json({ error: 'Invalid JSON body' }, { status: 400 });
  }
  const { solidity, sessionId } = body;

  if (!solidity || !sessionId) {
    return NextResponse.json({ error: 'Missing solidity or sessionId' }, { status: 400 });
  }

  const env = buildEnv();
  const inkportRoot = env.INKPORT_ROOT as string;
  const name = parseName(solidity);
  const tmpdir = sessionDir(sessionId);
  const solFile = path.join(tmpdir, `${name}.sol`);
  const buildDir = path.join(tmpdir, 'build', name);

  fs.mkdirSync(path.join(buildDir, 'src'), { recursive: true });
  fs.writeFileSync(solFile, solidity, 'utf8');

  const translatorBin = path.join(
    inkportRoot, 'translator', 'target', 'release', 'inkport-translate'
  );

  const result = await spawnCollect(
    translatorBin,
    [solFile, '--target', 'seal', '--out', buildDir],
    { env }
  );

  if (result.code !== 0) {
    return NextResponse.json(
      { error: (result.stderr || result.stdout).trim() },
      { status: 400 }
    );
  }

  const rustPath = path.join(buildDir, 'src', 'lib.rs');
  const metaPath = path.join(buildDir, 'metadata.json');
  const rust = fs.readFileSync(rustPath, 'utf8');
  const metadata = JSON.parse(fs.readFileSync(metaPath, 'utf8'));

  return NextResponse.json({ rust, metadata });
}
