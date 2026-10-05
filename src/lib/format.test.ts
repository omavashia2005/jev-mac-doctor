import { describe, expect, it } from "vitest"
import { formatBytes, formatPercent, formatRate, titleCase } from "./format"

describe("telemetry formatting", () => {
  it("shows unavailable values as an em dash instead of zero", () => {
    expect(formatBytes(null)).toBe("—")
    expect(formatPercent(undefined)).toBe("—")
    expect(formatRate(null)).toBe("—")
  })

  it("keeps actual zero distinguishable from unavailable", () => {
    expect(formatBytes(0)).toBe("0 B")
    expect(formatPercent(0)).toBe("0.0%")
  })

  it("labels mechanism keys", () => {
    expect(titleCase("scheduler_pressure")).toBe("Scheduler Pressure")
  })
})
