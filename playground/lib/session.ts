import path from 'path';
import fs from 'fs';
import os from 'os';

export function sessionDir(sessionId: string): string {
  // os.tmpdir() returns an absolute path with a drive letter on Windows
  // (e.g. C:\Users\...\AppData\Local\Temp), which shell.ts can then convert
  // to a proper WSL mount path (/mnt/c/...). path.join('/tmp', ...) on Windows
  // produces a drive-letter-less path that maps to WSL's own /tmp, not Windows'.
  const dir = path.join(os.tmpdir(), 'inkport-playground', sessionId);
  fs.mkdirSync(dir, { recursive: true });
  return dir;
}
