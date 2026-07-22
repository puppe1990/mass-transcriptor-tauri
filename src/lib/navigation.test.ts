import { describe, expect, it } from "vitest";
import { targetAfterUpload, targetOpenBatch } from "./navigation";
import type { CreatedJob } from "./api";

function job(partial: Partial<CreatedJob> & Pick<CreatedJob, "id">): CreatedJob {
  return {
    uploadId: partial.uploadId ?? partial.id,
    batchId: partial.batchId ?? null,
    status: partial.status ?? "queued",
    originalFilename: partial.originalFilename ?? `f${partial.id}.wav`,
    ...partial,
  };
}

describe("targetAfterUpload", () => {
  it("opens single job detail for one file", () => {
    expect(targetAfterUpload([job({ id: 7 })])).toEqual({ route: "job", id: 7 });
  });

  it("opens batch when two audios share batchId (view group)", () => {
    expect(
      targetAfterUpload([
        job({ id: 1, batchId: 99 }),
        job({ id: 2, batchId: 99 }),
      ]),
    ).toEqual({ route: "batch", id: 99 });
  });

  it("falls back to jobs when multi-file has no batchId", () => {
    expect(targetAfterUpload([job({ id: 1 }), job({ id: 2 })])).toEqual({
      route: "jobs",
    });
  });
});

describe("targetOpenBatch", () => {
  it("opens batch for valid id (jobs list Ver/open)", () => {
    expect(targetOpenBatch(42)).toEqual({ route: "batch", id: 42 });
  });

  it("does not open batch when id is undefined (broken snake_case payload)", () => {
    expect(targetOpenBatch(undefined)).toEqual({ route: "jobs" });
    expect(targetOpenBatch(null)).toEqual({ route: "jobs" });
  });
});
