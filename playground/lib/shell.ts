import { spawn } from 'child_process';

export interface SpawnResult {
  stdout: string;
  stderr: string;
  code: number;
}

export function spawnCollect(
  cmd: string,
  args: string[],
  opts: { cwd?: string; env?: NodeJS.ProcessEnv }
): Promise<SpawnResult> {
  return new Promise((resolve) => {
    const proc = spawn(cmd, args, {
      cwd: opts.cwd,
      env: opts.env ?? process.env,
      shell: false,
    });
    let stdout = '';
    let stderr = '';
    proc.stdout.on('data', (d: Buffer) => { stdout += d.toString(); });
    proc.stderr.on('data', (d: Buffer) => { stderr += d.toString(); });
    proc.on('error', (err: Error) => resolve({ stdout, stderr: err.message, code: 1 }));
    proc.on('close', (code: number | null) => resolve({ stdout, stderr, code: code ?? 1 }));
  });
}

function classifyLine(line: string): string {
  const t = line.trim();
  if (/^✓|Finished/.test(t)) return 'lg-ok';
  if (/^✗|^error[^:]*:/i.test(t)) return 'lg-err';
  if (/^warning/i.test(t)) return 'lg-warn';
  if (/^\$/.test(t)) return 'lg-cmd';
  return 'lg-dim';
}

export function spawnStream(
  cmd: string,
  args: string[],
  opts: { cwd?: string; env?: NodeJS.ProcessEnv }
): ReadableStream<Uint8Array> {
  const encoder = new TextEncoder();

  return new ReadableStream<Uint8Array>({
    start(controller) {
      const proc = spawn(cmd, args, {
        cwd: opts.cwd,
        env: opts.env ?? process.env,
        shell: false,
      });

      function emitLine(line: string) {
        const cls = classifyLine(line);
        const event = JSON.stringify({ type: 'log', cls, text: line });
        controller.enqueue(encoder.encode(`data: ${event}\n\n`));
      }

      let stderrBuf = '';

      proc.stdout.on('data', (chunk: Buffer) => {
        chunk.toString().split('\n').forEach(l => { if (l) emitLine(l); });
      });
      proc.stderr.on('data', (chunk: Buffer) => {
        stderrBuf += chunk.toString();
        chunk.toString().split('\n').forEach(l => { if (l) emitLine(l); });
      });
      proc.on('error', (err: Error) => {
        controller.enqueue(encoder.encode(`data: ${JSON.stringify({ type: 'error', log: err.message })}\n\n`));
        controller.close();
      });
      proc.on('close', (code: number | null) => {
        if ((code ?? 1) === 0) {
          controller.enqueue(encoder.encode(`data: ${JSON.stringify({ type: 'done' })}\n\n`));
        } else {
          controller.enqueue(encoder.encode(`data: ${JSON.stringify({ type: 'error', log: stderrBuf })}\n\n`));
        }
        controller.close();
      });
    },
  });
}
