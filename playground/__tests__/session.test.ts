import os from 'os';
import path from 'path';
import fs from 'fs';
import { sessionDir } from '@/lib/session';

describe('sessionDir', () => {
  it('returns path under /tmp/inkport-playground/<uuid>', () => {
    const dir = sessionDir('test-uuid-1234');
    // Normalize for cross-platform comparison
    const normalized = dir.replace(/\\/g, '/');
    expect(normalized).toBe('/tmp/inkport-playground/test-uuid-1234');
  });

  it('creates the directory', () => {
    const dir = sessionDir('test-uuid-mkdir');
    expect(fs.existsSync(dir)).toBe(true);
    fs.rmSync(dir, { recursive: true });
  });
});
