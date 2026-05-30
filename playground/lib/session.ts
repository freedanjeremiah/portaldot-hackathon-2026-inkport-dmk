import path from 'path';
import fs from 'fs';

export function sessionDir(sessionId: string): string {
  const dir = path.join('/tmp', 'inkport-playground', sessionId);
  fs.mkdirSync(dir, { recursive: true });
  return dir;
}
