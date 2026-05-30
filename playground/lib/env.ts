import path from 'path';

function wslPath(p: string): string {
  if (process.platform !== 'win32') return p;
  const drive = p.match(/^([A-Za-z]):[\\\/](.*)/);
  if (drive) return `/mnt/${drive[1].toLowerCase()}/${drive[2].replace(/\\/g, '/')}`;
  return p.replace(/\\/g, '/');
}

export function buildEnv(): NodeJS.ProcessEnv {
  const inkportRoot = process.env.INKPORT_ROOT ?? '';
  const cargoHome = process.env.CARGO_HOME ?? path.join(process.env.HOME ?? '', '.cargo');
  const inkportVenv = process.env.INKPORT_VENV ?? '';

  // PATH entries must be Linux-format for WSL subprocesses.
  const extraPath = [
    inkportVenv ? wslPath(path.join(inkportVenv, 'bin')) : '',
    cargoHome ? wslPath(path.join(cargoHome, 'bin')) : '',
  ].filter(Boolean).join(':');

  return {
    ...process.env,
    PATH: `${extraPath}:${process.env.PATH ?? ''}`,
    INKPORT_ROOT: inkportRoot,
    CARGO_HOME: cargoHome,
    INKPORT_VENV: inkportVenv,
  };
}
