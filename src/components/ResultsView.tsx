import { useState } from "react"
import { AlertTriangleIcon, Clock3Icon } from "lucide-react"
import { TelemetryPlot, type PlotSeries } from "@/components/TelemetryPlot"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Badge } from "@/components/ui/badge"
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import { Progress } from "@/components/ui/progress"
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { formatTimestamp, titleCase } from "@/lib/format"
import type { ProcessWindow, SlowdownEvent } from "@/types"

export function ResultsView({ event }: { event: SlowdownEvent | null }) {
  const result = event?.jevResult
  const [selectedProcessId, setSelectedProcessId] = useState(
    result?.processes[0]?.processId ?? event?.processes[0]?.identity.id ?? "",
  )
  if (!event) {
    return (
      <Alert>
        <Clock3Icon />
        <AlertTitle>No diagnostic selected</AlertTitle>
        <AlertDescription>Capture a slowdown or open a completed event from history.</AlertDescription>
      </Alert>
    )
  }
  const processNames = Object.fromEntries(event.processes.map((process) => [process.identity.id, process.identity.name]))
  processNames.system_or_unknown = "system / unknown"
  const selectedWindow = event.processes.find((process) => process.identity.id === selectedProcessId)
  const selectedResult = result?.processes.find((process) => process.processId === selectedProcessId)

  return (
    <div className="flex flex-col gap-4">
      <div className="grid gap-3 md:grid-cols-4">
        <Metadata label="Marked" value={formatTimestamp(event.clickedAtNs)} />
        <Metadata label="Window" value={`${formatTimestamp(event.windowStartNs)} – ${formatTimestamp(event.windowEndNs)}`} />
        <Metadata label="Processes" value={String(event.processes.length)} />
        <Metadata label="System samples" value={String(event.systemSamples.length)} />
      </div>

      <TimelineMarker />

      {!result ? (
        <Alert variant="destructive">
          <AlertTriangleIcon />
          <AlertTitle>Jev analysis unavailable</AlertTitle>
          <AlertDescription>
            {event.errors.at(-1)?.message ?? "This event has telemetry but no normalized Jev result."}
          </AlertDescription>
        </Alert>
      ) : (
        <>
          <div className="grid gap-3 xl:grid-cols-2">
            <Card>
              <CardHeader>
                <CardTitle>Slowdown severity</CardTitle>
                <CardDescription>Score is a probability-weighted position, not an exact percent slower.</CardDescription>
                <CardAction>
                  <Badge variant="secondary">{result.slowdownSeverity.score.toFixed(2)} / 9</Badge>
                </CardAction>
              </CardHeader>
              <CardContent>
                <ProbabilityBars
                  probabilities={result.slowdownSeverity.probabilities}
                  label={(key) => `${key} · ${result.slowdownSeverity.legend[key] ?? "Unknown"}`}
                />
              </CardContent>
            </Card>
            <Card>
              <CardHeader>
                <CardTitle>Primary cause</CardTitle>
                <CardDescription>Competing explanations; the Choice distribution sums to 100%.</CardDescription>
                <CardAction>
                  <Badge variant="outline">
                    confidence {result.primaryCause.confidence?.toFixed(2) ?? "—"}
                  </Badge>
                </CardAction>
              </CardHeader>
              <CardContent>
                <ProbabilityBars
                  probabilities={result.primaryCause.probabilities}
                  label={(key) => processNames[key] ?? key}
                />
              </CardContent>
            </Card>
          </div>

          <div className="grid gap-3 xl:grid-cols-2">
            <Card>
              <CardHeader>
                <CardTitle>Independent contribution</CardTitle>
                <CardDescription>Each Noul is P(yes); these values do not need to sum to 100%.</CardDescription>
              </CardHeader>
              <CardContent>
                <ProbabilityBars
                  probabilities={Object.fromEntries(
                    result.processes
                      .filter((process) => process.contributionProbability != null)
                      .map((process) => [process.processId, process.contributionProbability!]),
                  )}
                  label={(key) => processNames[key] ?? key}
                />
              </CardContent>
            </Card>
            <Card>
              <CardHeader>
                <CardTitle>Mechanism</CardTitle>
                <CardDescription>Conditional Choice for the selected process, based only on observed telemetry.</CardDescription>
                <CardAction>
                  <ProcessSelect
                    event={event}
                    value={selectedProcessId}
                    onValueChange={setSelectedProcessId}
                  />
                </CardAction>
              </CardHeader>
              <CardContent>
                {selectedResult?.mechanism ? (
                  <ProbabilityBars
                    probabilities={selectedResult.mechanism.probabilities}
                    label={titleCase}
                  />
                ) : (
                  <div className="py-10 text-center text-sm text-muted-foreground">No Jev mechanism result for this process.</div>
                )}
              </CardContent>
            </Card>
          </div>
        </>
      )}

      <Card>
        <CardHeader>
          <CardTitle>Process timeline</CardTitle>
          <CardDescription>Every chart shares the same 20-second window and slowdown marker.</CardDescription>
          <CardAction>
            <ProcessSelect event={event} value={selectedProcessId} onValueChange={setSelectedProcessId} />
          </CardAction>
        </CardHeader>
        <CardContent>
          {selectedWindow ? (
            <ProcessTimeline process={selectedWindow} markerTimestamp={event.clickedAtNs} />
          ) : (
            <div className="py-10 text-center text-sm text-muted-foreground">Select a process with retained samples.</div>
          )}
        </CardContent>
      </Card>
    </div>
  )
}

function Metadata({ label, value }: { label: string; value: string }) {
  return (
    <Card size="sm">
      <CardHeader>
        <CardDescription>{label}</CardDescription>
        <CardTitle className="text-sm">{value}</CardTitle>
      </CardHeader>
    </Card>
  )
}

function TimelineMarker() {
  return (
    <Card size="sm">
      <CardContent>
        <div className="grid grid-cols-[1fr_auto_1fr] items-center gap-3">
          <div>
            <div className="text-xs text-muted-foreground">T − 10 sec</div>
            <div className="h-px bg-border" />
          </div>
          <div className="flex flex-col items-center gap-1">
            <div className="text-xs font-medium">slowdown click</div>
            <div className="h-5 w-px bg-destructive" />
          </div>
          <div>
            <div className="text-right text-xs text-muted-foreground">T + 10 sec</div>
            <div className="h-px bg-border" />
          </div>
        </div>
      </CardContent>
    </Card>
  )
}

function ProbabilityBars({
  probabilities,
  label,
}: {
  probabilities: Record<string, number>
  label: (key: string) => string
}) {
  return (
    <div className="flex flex-col gap-3">
      {Object.entries(probabilities)
        .sort((left, right) => right[1] - left[1])
        .map(([key, probability]) => (
          <div key={key} className="grid grid-cols-[minmax(9rem,1fr)_3fr_3.5rem] items-center gap-3">
            <div className="truncate text-sm" title={label(key)}>{label(key)}</div>
            <Progress value={probability * 100} aria-label={`${label(key)} ${(probability * 100).toFixed(1)} percent`} />
            <div className="text-right font-mono text-xs tabular-nums">{(probability * 100).toFixed(1)}%</div>
          </div>
        ))}
    </div>
  )
}

function ProcessSelect({
  event,
  value,
  onValueChange,
}: {
  event: SlowdownEvent
  value: string
  onValueChange: (value: string) => void
}) {
  return (
    <Select value={value} onValueChange={onValueChange}>
      <SelectTrigger aria-label="Selected process" className="max-w-56">
        <SelectValue placeholder="Select process" />
      </SelectTrigger>
      <SelectContent>
        <SelectGroup>
          {event.processes.map((process) => (
            <SelectItem key={process.identity.id} value={process.identity.id}>
              {process.identity.name} · {process.identity.pid}
            </SelectItem>
          ))}
        </SelectGroup>
      </SelectContent>
    </Select>
  )
}

function ProcessTimeline({ process, markerTimestamp }: { process: ProcessWindow; markerTimestamp: number }) {
  const timestamps = process.samples.map((sample) => sample.timestampNs)
  const charts: Array<{ title: string; series: PlotSeries[] }> = [
    { title: "CPU", series: [{ label: "%", values: process.samples.map((s) => s.cpu.percent ?? null), color: "#d4d4d8" }] },
    { title: "GPU time", series: [{ label: "%", values: process.samples.map((s) => s.gpu.utilizationPercent ?? null), color: "#d4d4d8" }] },
    { title: "Memory", series: [{ label: "bytes", values: process.samples.map((s) => s.memory.physicalFootprintBytes ?? null), color: "#d4d4d8" }] },
    {
      title: "Disk",
      series: [
        { label: "read/s", values: process.samples.map((s) => s.diskIo.readBytesPerSecond ?? null), color: "#d4d4d8" },
        { label: "write/s", values: process.samples.map((s) => s.diskIo.writeBytesPerSecond ?? null), color: "#71717a" },
      ],
    },
    {
      title: "Network",
      series: [
        { label: "rx/s", values: process.samples.map((s) => s.network.rxBytesPerSecond ?? null), color: "#d4d4d8" },
        { label: "tx/s", values: process.samples.map((s) => s.network.txBytesPerSecond ?? null), color: "#71717a" },
      ],
    },
    {
      title: "Wakeups",
      series: [
        { label: "interrupt/s", values: process.samples.map((s) => s.scheduling.interruptWakeupsPerSecond ?? null), color: "#d4d4d8" },
        { label: "package/s", values: process.samples.map((s) => s.scheduling.packageIdleWakeupsPerSecond ?? null), color: "#71717a" },
      ],
    },
    {
      title: "Runnable / wait",
      series: [
        { label: "runnable ns", values: process.samples.map((s) => s.scheduling.runnableTimeNs ?? null), color: "#d4d4d8" },
        { label: "wait ns", values: process.samples.map((s) => s.scheduling.waitTimeNs ?? null), color: "#71717a" },
      ],
    },
    { title: "Energy", series: [{ label: "nJ", values: process.samples.map((s) => s.energy.energyNj ?? null), color: "#d4d4d8" }] },
    { title: "Neural footprint", series: [{ label: "bytes", values: process.samples.map((s) => s.neural.footprintBytes ?? null), color: "#d4d4d8" }] },
  ]
  return (
    <div className="grid gap-3 xl:grid-cols-2">
      {charts.map((chart) => (
        <div key={chart.title} className="rounded-lg border p-3">
          <div className="mb-2 text-sm font-medium">{chart.title}</div>
          <TelemetryPlot timestamps={timestamps} series={chart.series} markerTimestamp={markerTimestamp} height={130} />
        </div>
      ))}
    </div>
  )
}
