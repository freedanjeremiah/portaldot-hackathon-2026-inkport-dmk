import path from 'path';

export function buildEnv(): NodeJS.ProcessEnv {
  const inkportRoot = process.env.INKPORT_ROOT ?? '';
  const cargoHome = process.env.CARGO_HOME ?? path.join(process.env.HOME ?? '', '.cargo');
  const inkportVenv = process.env.INKPORT_VENV ?? '';

  const extraPath = [
    inkportVenv ? path.join(inkportVenv, 'bin') : '',
    cargoHome ? path.join(cargoHome, 'bin') : '',
  ].filter(Boolean).join(':');

  return {
    ...process.env,
    PATH: `${extraPath}:${process.env.PATH ?? ''}`,
    INKPORT_ROOT: inkportRoot,
    CARGO_HOME: cargoHome,
    INKPORT_VENV: inkportVenv,
  };
}
