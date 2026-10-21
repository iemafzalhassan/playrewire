import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { BundleSchema } from './types';

// Built from tools/packcheck/testdata/good; a Go test fails if it goes stale.
const fixture = new URL('../../../../tools/packcheck/testdata/good.bundle.json', import.meta.url);
const bundle = () => JSON.parse(readFileSync(fixture, 'utf8'));

describe('BundleSchema', () => {
  it('bundle schema accepts packcheck output', () => {
    const parsed = BundleSchema.parse(bundle());
    expect(parsed.wrapperLineOffset).toBe(7);
    expect(parsed.version).toBe(1);
    const chunks = parsed.packs.flatMap((p) => p.chunks);
    expect(chunks.find((c) => c.id === 'hello')?.steps).toEqual(['see']);
    expect(chunks.find((c) => c.id === 'print')?.steps).toEqual(['see', 'type', 'drill', 'use']);
    expect(parsed.packs[0].missions[0].expectEvents[0]).toEqual({ op: 'lamp', i: 0, on: true });
  });

  it('rejects an unknown event op', () => {
    const b = bundle();
    b.packs[0].missions[0].expectEvents[0] = { op: 'explode' };
    expect(BundleSchema.safeParse(b).success).toBe(false);
  });

  it('rejects a missing camelCase key', () => {
    const b = bundle();
    delete b.packs[0].chunks[0].steps;
    expect(BundleSchema.safeParse(b).success).toBe(false);
  });
});
