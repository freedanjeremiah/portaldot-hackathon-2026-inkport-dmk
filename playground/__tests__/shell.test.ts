import { spawnCollect } from '@/lib/shell';

describe('spawnCollect', () => {
  it('captures stdout and returns exit 0', async () => {
    const result = await spawnCollect(
      'node', ['-e', "process.stdout.write('hello world')"], {}
    );
    expect(result.stdout.trim()).toBe('hello world');
    expect(result.code).toBe(0);
  });

  it('captures stderr and returns non-zero exit on failure', async () => {
    const result = await spawnCollect(
      'node', ['-e', "process.stderr.write('err'); process.exit(1)"], {}
    );
    expect(result.stderr.trim()).toBe('err');
    expect(result.code).toBe(1);
  });
});
