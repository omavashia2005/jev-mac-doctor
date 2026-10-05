export type OptionalNumber = number | null

export interface ProcessIdentity {
  id: string
  pid: number
  ppid?: number | null
  processStartTime?: number | null
  name: string
  executable?: string | null
  bundleIdentifier?: string | null
  responsiblePid?: number | null
  coalitionId?: number | null
  uid?: number | null
}

export interface ProcessSample {
  timestampNs: number
  identity: ProcessIdentity
  cpu: {
    percent?: OptionalNumber
    userTimeNs?: OptionalNumber
    systemTimeNs?: OptionalNumber
    runnableTimeNs?: OptionalNumber
    instructions?: OptionalNumber
    instructionsPerSecond?: OptionalNumber
    cycles?: OptionalNumber
    cyclesPerSecond?: OptionalNumber
    instructionsPerCycle?: OptionalNumber
    pCoreInstructions?: OptionalNumber
    pCoreCycles?: OptionalNumber
  }
  gpu: {
    accumulatedTimeNs?: OptionalNumber
    deltaTimeNs?: OptionalNumber
    utilizationPercent?: OptionalNumber
    source?: string | null
  }
  neural: {
    footprintBytes?: OptionalNumber
    intervalMaxFootprintBytes?: OptionalNumber
    lifetimeMaxFootprintBytes?: OptionalNumber
  }
  memory: {
    residentBytes?: OptionalNumber
    physicalFootprintBytes?: OptionalNumber
    wiredBytes?: OptionalNumber
    maxPhysicalFootprintBytes?: OptionalNumber
    pageins?: OptionalNumber
  }
  diskIo: {
    readBytes?: OptionalNumber
    writeBytes?: OptionalNumber
    readBytesPerSecond?: OptionalNumber
    writeBytesPerSecond?: OptionalNumber
    logicalWrites?: OptionalNumber
  }
  network: {
    rxBytes?: OptionalNumber
    txBytes?: OptionalNumber
    rxBytesPerSecond?: OptionalNumber
    txBytesPerSecond?: OptionalNumber
    available: boolean
  }
  scheduling: {
    interruptWakeups?: OptionalNumber
    interruptWakeupsPerSecond?: OptionalNumber
    packageIdleWakeups?: OptionalNumber
    packageIdleWakeupsPerSecond?: OptionalNumber
    runnableTimeNs?: OptionalNumber
    waitTimeNs?: OptionalNumber
  }
  qos: Record<string, OptionalNumber>
  energy: {
    energyNj?: OptionalNumber
    billedEnergyNj?: OptionalNumber
    servicedEnergyNj?: OptionalNumber
    energyImpact?: OptionalNumber
    pCoreEnergyNj?: OptionalNumber
  }
}

export interface SystemSample {
  timestampNs: number
  cpu: {
    utilizationPercent?: OptionalNumber
    userPercent?: OptionalNumber
    systemPercent?: OptionalNumber
    idlePercent?: OptionalNumber
    frequencyMHz?: OptionalNumber
    frequencyRatio?: OptionalNumber
    performanceCoreResidency?: Record<string, number> | null
    efficiencyCoreResidency?: Record<string, number> | null
    instructionsPerSecond?: OptionalNumber
    cyclesPerSecond?: OptionalNumber
    powerMw?: OptionalNumber
  }
  gpu: {
    activePercent?: OptionalNumber
    frequencyMHz?: OptionalNumber
    residency?: Record<string, number> | null
    powerMw?: OptionalNumber
  }
  ane: {
    activePercent?: OptionalNumber
    residency?: Record<string, number> | null
    powerMw?: OptionalNumber
    energyMj?: OptionalNumber
  }
  memory: {
    physicalBytes?: OptionalNumber
    usedBytes?: OptionalNumber
    freeBytes?: OptionalNumber
    activeBytes?: OptionalNumber
    inactiveBytes?: OptionalNumber
    wiredBytes?: OptionalNumber
    compressedBytes?: OptionalNumber
    swapUsedBytes?: OptionalNumber
    pageinsPerSecond?: OptionalNumber
    pageoutsPerSecond?: OptionalNumber
    memoryPressure?: string | null
  }
  disk: {
    readBytesPerSecond?: OptionalNumber
    writeBytesPerSecond?: OptionalNumber
    readOpsPerSecond?: OptionalNumber
    writeOpsPerSecond?: OptionalNumber
  }
  network: {
    rxBytesPerSecond?: OptionalNumber
    txBytesPerSecond?: OptionalNumber
    rxPacketsPerSecond?: OptionalNumber
    txPacketsPerSecond?: OptionalNumber
  }
  scheduler: {
    interruptWakeupsPerSecond?: OptionalNumber
    packageIdleWakeupsPerSecond?: OptionalNumber
    forcedIdlePercent?: OptionalNumber
  }
  thermal: {
    state?: string | null
    thermalPressure?: string | null
    cpuPowerLimitPercent?: OptionalNumber
    gpuPowerLimitPercent?: OptionalNumber
    throttled?: boolean | null
  }
}

export interface TelemetryFrame {
  timestampNs: number
  system: SystemSample
  processes: ProcessSample[]
}

export interface Capability {
  available: boolean
  source?: string | null
  reason?: string | null
  privateApi: boolean
  privileged: boolean
}

export interface DiagnosticError {
  timestampNs: number
  collector: string
  message: string
  recoverable: boolean
}

export type SlowdownStatus =
  | "collecting-post-window"
  | "building-state"
  | "querying-jev"
  | "complete"
  | "failed"

export interface ProcessWindow {
  identity: ProcessIdentity
  samples: ProcessSample[]
}

export interface JevSlowdownResult {
  slowdownSeverity: {
    score: number
    confidence?: number | null
    probabilities: Record<string, number>
    legend: Record<string, string>
  }
  primaryCause: {
    processId: string
    confidence?: number | null
    probabilities: Record<string, number>
  }
  processes: Array<{
    processId: string
    pid: number
    name: string
    contributionProbability?: number | null
    mechanism?: {
      choice: string
      confidence?: number | null
      probabilities: Record<string, number>
    } | null
  }>
  rawResponse: unknown
}

export interface SlowdownEvent {
  id: string
  clickedAtNs: number
  windowStartNs: number
  windowEndNs: number
  status: SlowdownStatus
  systemSamples: SystemSample[]
  processes: ProcessWindow[]
  capabilities: Record<string, Capability>
  jevRequest?: unknown
  jevResult?: JevSlowdownResult | null
  errors: DiagnosticError[]
}

export interface CollectorSnapshot {
  running: boolean
  sampleIntervalMs: number
  processesTracked: number
  samplesRetained: number
  lastSampleTimestampNs?: number | null
  latest?: TelemetryFrame | null
  capabilities: Record<string, Capability>
  errors: DiagnosticError[]
}

export interface CaptureStatus {
  id: string
  clickedAtNs: number
  windowEndNs: number
  status: SlowdownStatus
  remainingSeconds?: number | null
  message?: string | null
}

export interface EventSummary {
  id: string
  clickedAtNs: number
  status: SlowdownStatus
  severity?: number | null
  primaryProcess?: string | null
}

export interface TypeSafeCredentialStatus {
  configured: boolean
  source?: string | null
}
