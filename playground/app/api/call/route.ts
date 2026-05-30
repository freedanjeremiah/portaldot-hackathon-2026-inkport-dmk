import { NextRequest, NextResponse } from 'next/server';
import { buildEnv } from '@/lib/env';
import { spawnCollect } from '@/lib/shell';

export async function POST(request: NextRequest) {
  const body = await request.json() as { metadata?: { name: string }; message?: string; args?: string[] };
  const { metadata, message, args } = body;

  if (!metadata?.name || !message) {
    return NextResponse.json({ error: 'Missing metadata.name or message' }, { status: 400 });
  }

  const name = metadata.name;
  const env = buildEnv();
  const inkportRoot = env.INKPORT_ROOT as string;

  const argFlags = (args ?? []).flatMap(a => ['--arg', String(a)]);

  const result = await spawnCollect(
    'inkport',
    ['call', name, message, ...argFlags],
    { cwd: inkportRoot, env }
  );

  if (result.code !== 0) {
    return NextResponse.json(
      { error: (result.stderr || result.stdout).trim() },
      { status: 400 }
    );
  }

  // inkport call prints: "call Name.msg(...) -> <value>"
  // followed by a JSON line: {"result": <value>}
  let parsed: unknown = null;
  try {
    const jsonLine = result.stdout.split('\n').find(l => l.trim().startsWith('{'));
    if (jsonLine) parsed = (JSON.parse(jsonLine) as { result: unknown }).result;
  } catch { /* ignore */ }

  if (parsed === null) {
    const match = /call\s+\S+\s+->\s+(.+)/.exec(result.stdout);
    parsed = match ? match[1].trim() : 'ok';
  }

  return NextResponse.json({ result: parsed, events: [] });
}
