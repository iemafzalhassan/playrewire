// TypeScript mirror of the bundle that tools/packcheck emits
// (apps/web/public/content/go.json). Keep in step with
// tools/packcheck/schema.go; the Vitest test parses a real packcheck bundle.
import { z } from 'zod';

export const StepSchema = z.enum(['see', 'type', 'drill', 'use']);
export type Step = z.infer<typeof StepSchema>;

/** One @city event the city stub prints. */
export const CityEventSchema = z.discriminatedUnion('op', [
  z.object({ op: z.literal('green'), i: z.number().int(), delay: z.number().int() }).strict(),
  z.object({ op: z.literal('lamp'), i: z.number().int(), on: z.boolean() }).strict(),
  z.object({ op: z.literal('say'), text: z.string() }).strict(),
]);
export type CityEvent = z.infer<typeof CityEventSchema>;

export const PathRefSchema = z.object({ pack: z.string(), chunk: z.string() }).strict();
export type PathRef = z.infer<typeof PathRefSchema>;

export const SeeSchema = z.object({ scene: z.string(), caption: z.string() }).strict();
export type See = z.infer<typeof SeeSchema>;

export const TipSchema = z.object({ match: z.string(), text: z.string() }).strict();
export type Tip = z.infer<typeof TipSchema>;

/** wholeFile: the code is a complete `package main` program, run unwrapped (lineOffset 0). */
export const TypeStepSchema = z
  .object({ ghost: z.string(), tips: z.array(TipSchema), wholeFile: z.boolean() })
  .strict();
export type TypeStep = z.infer<typeof TypeStepSchema>;

export const DrillSchema = z
  .object({
    prompt: z.string(),
    starter: z.string().optional(),
    solution: z.string(),
    expectStdout: z.string(),
    wholeFile: z.boolean(),
  })
  .strict();
export type Drill = z.infer<typeof DrillSchema>;

export const FixItSchema = z
  .object({
    broken: z.string(),
    solution: z.string(),
    expectStdout: z.string(),
    wholeFile: z.boolean(),
  })
  .strict();
export type FixIt = z.infer<typeof FixItSchema>;

export const ChunkSchema = z
  .object({
    id: z.string(),
    concept: z.string(),
    title: z.string(),
    see: SeeSchema,
    type: TypeStepSchema.optional(),
    /** Empty for see-only chunks; otherwise at least 5. */
    drills: z.array(DrillSchema),
    /** Empty when there are no drills; otherwise exactly 3. */
    hints: z.array(z.string()),
    fixIt: FixItSchema.optional(),
    /** Mission id within the same pack. */
    mission: z.string().optional(),
    pack: z.string(),
    steps: z.array(StepSchema),
  })
  .strict();
export type Chunk = z.infer<typeof ChunkSchema>;

export const LevelSchema = z
  .object({ template: z.string(), solution: z.string(), wholeFile: z.boolean() })
  .strict();
export type Level = z.infer<typeof LevelSchema>;

export const LevelsSchema = z
  .object({
    fillBlank: LevelSchema,
    halfFunction: LevelSchema,
    blankWithHints: LevelSchema,
  })
  .strict();
export type Levels = z.infer<typeof LevelsSchema>;

export const MissionSchema = z
  .object({
    id: z.string(),
    concept: z.string(),
    terminal: z.string(),
    title: z.string(),
    /** Go source for setup.go: package main, no main func, may be empty. */
    setup: z.string(),
    levels: LevelsSchema,
    expectEvents: z.array(CityEventSchema),
    pack: z.string(),
  })
  .strict();
export type Mission = z.infer<typeof MissionSchema>;

export const PackSchema = z
  .object({
    id: z.string(),
    title: z.string(),
    district: z.string(),
    chunks: z.array(ChunkSchema),
    missions: z.array(MissionSchema),
  })
  .strict();
export type Pack = z.infer<typeof PackSchema>;

export const BundleSchema = z
  .object({
    version: z.number().int(),
    language: z.literal('go'),
    /** Wrapper lines above user line 1 (user line 1 is file line 8). */
    wrapperLineOffset: z.number().int(),
    cityStub: z.string(),
    path: z.array(PathRefSchema),
    packs: z.array(PackSchema),
  })
  .strict();
export type Bundle = z.infer<typeof BundleSchema>;
