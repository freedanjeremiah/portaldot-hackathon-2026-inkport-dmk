import { spawn } from 'child_process';

export interface SpawnLiveResult {
  code: number;
  stderr: string;
}

export interface SpawnResult {
  stdout: string;
  stderr: string;
  code: number;
}

// On Windows, route all subprocess calls through WSL bash so Linux binaries work.
// Windows path.join() produces backslash paths and drive letters (C:\foo\bar).
// WSL mounts Windows drives at /mnt/<drive>/, so we must convert:
//   C:\foo\bar   →  /mnt/c/foo/bar
//   D:\foo\bar   →  /mnt/d/foo/bar
//   \foo\bar     →  /foo/bar  (UNC-relative, rare)
function toLinux(p: string): string {
  if (process.platform !== 'win32') return p;
  const drive = p.match(/^([A-Za-z]):[\\\/](.*)/);
  if (drive) return `/mnt/${drive[1].toLowerCase()}/${drive[2].replace(/\\/g, '/')}`;
  return p.replace(/\\/g, '/');
}

function buildWslArgs(cmd: string, args: string[], cwd?: string): { cmd: string; args: string[]; cwd?: string } {
  if (process.platform !== 'win32') return { cmd, args, cwd };
  const normCmd = toLinux(cmd);
  const normArgs = args.map(toLinux);
  const normCwd = cwd ? toLinux(cwd) : undefined;
  const quote = (s: string) => `'${s.replace(/'/g, "'\\''")}'`;
  const parts = [normCmd, ...normArgs].map(quote).join(' ');

  // Embed PATH setup in the bash script itself — WSL env inheritance from a
  // Windows Node.js parent is unreliable. Source cargo's env script so `cargo`
  // is findable, and prepend the venv's bin so `inkport`/`python3` are found.
  const venv = process.env.INKPORT_VENV ? toLinux(process.env.INKPORT_VENV) : '';
  const pathPrefix = venv ? `${venv}/bin:` : '';
  const envSetup = `source "$HOME/.cargo/env" 2>/dev/null; export PATH="${pathPrefix}$HOME/.cargo/bin:$PATH"; `;

  const body = normCwd ? `cd ${quote(normCwd)} && ${parts}` : parts;
  const script = envSetup + body;
  return { cmd: 'wsl', args: ['bash', '-c', script], cwd: undefined };
}

export function spawnCollect(
  cmd: string,
  args: string[],
  opts: { cwd?: string; env?: NodeJS.ProcessEnv }
): Promise<SpawnResult> {
  const { cmd: realCmd, args: realArgs, cwd } = buildWslArgs(cmd, args, opts.cwd);
  return new Promise((resolve) => {
    const proc = spawn(realCmd, realArgs, {
      cwd,
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

export function spawnLive(
  cmd: string,
  args: string[],
  opts: { cwd?: string; env?: NodeJS.ProcessEnv; onLine?: (line: string, isStderr: boolean) => void }
): Promise<SpawnLiveResult> {
  const { cmd: realCmd, args: realArgs, cwd } = buildWslArgs(cmd, args, opts.cwd);
  return new Promise((resolve) => {
    const proc = spawn(realCmd, realArgs, {
      cwd,
      env: opts.env ?? process.env,
      shell: false,
    });
    let stderr = '';
    let outBuf = '';
    let errBuf = '';

    function flush(buf: string, isStderr: boolean): string {
      const nl = buf.lastIndexOf('\n');
      if (nl === -1) return buf;
      buf.slice(0, nl).split('\n').forEach(line => {
        if (line && opts.onLine) opts.onLine(line, isStderr);
      });
      return buf.slice(nl + 1);
    }

    proc.stdout.on('data', (d: Buffer) => { outBuf = flush(outBuf + d.toString(), false); });
    proc.stderr.on('data', (d: Buffer) => {
      const s = d.toString();
      stderr += s;
      errBuf = flush(errBuf + s, true);
    });
    proc.on('error', (err: Error) => resolve({ code: 1, stderr: err.message }));
    proc.on('close', (code: number | null) => {
      if (outBuf && opts.onLine) opts.onLine(outBuf, false);
      if (errBuf && opts.onLine) opts.onLine(errBuf, true);
      resolve({ code: code ?? 1, stderr });
    });
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
  const { cmd: realCmd, args: realArgs, cwd } = buildWslArgs(cmd, args, opts.cwd);

  return new ReadableStream<Uint8Array>({
    start(controller) {
      const proc = spawn(realCmd, realArgs, {
        cwd,
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
