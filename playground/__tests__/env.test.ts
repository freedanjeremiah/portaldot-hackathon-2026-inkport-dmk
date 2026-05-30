import path from 'path';

// Must set before importing to avoid module-level evaluation issues
process.env.INKPORT_ROOT = '/fake/inkport';
process.env.CARGO_HOME = '/fake/.cargo';
process.env.INKPORT_VENV = '/fake/.venv';

import { buildEnv } from '@/lib/env';

describe('buildEnv', () => {
  it('includes venv/bin and cargo/bin in PATH', () => {
    const env = buildEnv();
    // On Windows, path.join uses backslashes; normalize for comparison
    const venvBin = path.join('/fake', '.venv', 'bin').replace(/\\/g, '/');
    const cargoBin = path.join('/fake', '.cargo', 'bin').replace(/\\/g, '/');
    const normalizedPath = (env.PATH ?? '').replace(/\\/g, '/');
    expect(normalizedPath).toContain(venvBin);
    expect(normalizedPath).toContain(cargoBin);
  });

  it('exposes INKPORT_ROOT', () => {
    const env = buildEnv();
    expect(env.INKPORT_ROOT).toBe('/fake/inkport');
  });
});
