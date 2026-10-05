import { ActivityIcon, GaugeIcon, HardDriveIcon, MemoryStickIcon, NetworkIcon, ZapIcon } from "lucide-react"
import { TelemetryPlot } from "@/components/TelemetryPlot"
import { ProcessTable } from "@/components/ProcessTable"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { formatBytes, formatPercent, formatRate, formatTimestamp } from "@/lib/format"
import type { CaptureStatus, CollectorSnapshot, TelemetryFrame } from "@/types"

interface LiveViewProps {
  frames: TelemetryFrame[]
  snapshot: CollectorSnapshot | null
  capture: CaptureStatus | null
  onCapture: () => void
  onIntervalChange: (milliseconds: number) => void
}

const chartCards = [
  {
    title: "CPU",
    description: "Host utilization",
    icon: ActivityIcon,
    formatValue: formatPercent,
    series: (frames: TelemetryFrame[]) => [
      { label: "Total", values: frames.map((frame) => frame.system.cpu.utilizationPercent ?? null), color: "#d4d4d8" },
      { label: "System", values: frames.map((frame) => frame.system.cpu.systemPercent ?? null), color: "#71717a" },
    ],
  },
  {
    title: "GPU",
    description: "System activity",
    icon: GaugeIcon,
    formatValue: formatPercent,
    series: (frames: TelemetryFrame[]) => [
      { label: "Active", values: frames.map((frame) => frame.system.gpu.activePercent ?? null), color: "#d4d4d8" },
    ],
  },
  {
    title: "Memory",
    description: "Used and compressed bytes",
    icon: MemoryStickIcon,
    formatValue: formatBytes,
    series: (frames: TelemetryFrame[]) => [
      { label: "Used", values: frames.map((frame) => frame.system.memory.usedBytes ?? null), color: "#d4d4d8" },
      { label: "Compressed", values: frames.map((frame) => frame.system.memory.compressedBytes ?? null), color: "#71717a" },
    ],
  },
  {
    title: "Disk",
    description: "Bytes per second",
    icon: HardDriveIcon,
    formatValue: formatRate,
    series: (frames: TelemetryFrame[]) => [
      { label: "Read", values: frames.map((frame) => frame.system.disk.readBytesPerSecond ?? null), color: "#d4d4d8" },
      { label: "Write", values: frames.map((frame) => frame.system.disk.writeBytesPerSecond ?? null), color: "#71717a" },
    ],
  },
  {
    title: "Network",
    description: "Interface bytes per second",
    icon: NetworkIcon,
    formatValue: formatRate,
    series: (frames: TelemetryFrame[]) => [
      { label: "Receive", values: frames.map((frame) => frame.system.network.rxBytesPerSecond ?? null), color: "#d4d4d8" },
      { label: "Transmit", values: frames.map((frame) => frame.system.network.txBytesPerSecond ?? null), color: "#71717a" },
    ],
  },
  {
    title: "Thermal / power",
    description: "CPU, GPU, and ANE power",
    icon: ZapIcon,
    formatValue: (value: number) => `${value.toFixed(1)} mW`,
    series: (frames: TelemetryFrame[]) => [
      { label: "CPU mW", values: frames.map((frame) => frame.system.cpu.powerMw ?? null), color: "#d4d4d8" },
      { label: "GPU mW", values: frames.map((frame) => frame.system.gpu.powerMw ?? null), color: "#a1a1aa" },
      { label: "ANE mW", values: frames.map((frame) => frame.system.ane.powerMw ?? null), color: "#52525b" },
    ],
  },
]

export function LiveView({ frames, snapshot, capture, onCapture, onIntervalChange }: LiveViewProps) {
  const latest = snapshot?.latest ?? frames.at(-1)
  const capturing = capture != null && !["complete", "failed"].includes(capture.status)
  const timestamps = frames.map((frame) => frame.timestampNs)

  return (
    <div className="flex flex-col gap-4">
      <div className="grid gap-3 md:grid-cols-4">
        <StatusMetric label="Collector" value={snapshot?.running ? "RUNNING" : "STARTING"} active={snapshot?.running} />
        <StatusMetric label="Processes tracked" value={String(snapshot?.processesTracked ?? 0)} />
        <StatusMetric label="Samples retained" value={String(snapshot?.samplesRetained ?? 0)} />
        <StatusMetric label="Last sample" value={formatTimestamp(snapshot?.lastSampleTimestampNs)} />
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Slowdown capture</CardTitle>
          <CardDescription>Marks now, retains the previous 10 seconds, then observes the next 10 seconds.</CardDescription>
          <CardAction className="flex items-center gap-2">
            <Select
              value={String(snapshot?.sampleIntervalMs ?? 1000)}
              onValueChange={(value) => onIntervalChange(Number(value))}
            >
              <SelectTrigger aria-label="Sample interval">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectGroup>
                  <SelectItem value="500">0.5 second</SelectItem>
                  <SelectItem value="1000">1 second</SelectItem>
                  <SelectItem value="2000">2 seconds</SelectItem>
                  <SelectItem value="5000">5 seconds</SelectItem>
                </SelectGroup>
              </SelectContent>
            </Select>
            <Button variant="destructive" size="lg" disabled={!snapshot?.running || capturing} onClick={onCapture}>
              MAC FEELS SLOW
            </Button>
          </CardAction>
        </CardHeader>
        {capture && (
          <CardContent>
            <Alert>
              <ActivityIcon />
              <AlertTitle>Slowdown marked at {formatTimestamp(capture.clickedAtNs)}</AlertTitle>
              <AlertDescription>
                {capture.status === "collecting-post-window"
                  ? `Collecting post-event telemetry… ${capture.remainingSeconds ?? 0}`
                  : capture.status === "building-state"
                    ? "Building the structured Jev state…"
                    : capture.status === "querying-jev"
                      ? "Querying Jev with Score, Choice, and Noul…"
                      : capture.status === "complete"
                        ? "Analysis complete."
                        : capture.message ?? "Capture failed; retained telemetry remains available."}
              </AlertDescription>
            </Alert>
          </CardContent>
        )}
      </Card>

      <div className="grid gap-3 xl:grid-cols-2">
        {chartCards.map((chart) => (
          <Card key={chart.title} size="sm">
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <chart.icon />
                {chart.title}
              </CardTitle>
              <CardDescription>{chart.description}</CardDescription>
            </CardHeader>
            <CardContent>
              <TelemetryPlot
                timestamps={timestamps}
                series={chart.series(frames)}
                markerTimestamp={capture?.clickedAtNs}
                formatValue={chart.formatValue}
              />
            </CardContent>
          </Card>
        ))}
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Processes</CardTitle>
          <CardDescription>Latest real sample; unavailable telemetry is shown as —.</CardDescription>
          <CardAction>
            <Badge variant="outline">Top 100 by CPU</Badge>
          </CardAction>
        </CardHeader>
        <CardContent>
          <ProcessTable processes={latest?.processes ?? []} />
        </CardContent>
      </Card>
    </div>
  )
}

function StatusMetric({ label, value, active }: { label: string; value: string; active?: boolean }) {
  return (
    <Card size="sm">
      <CardHeader>
        <CardDescription>{label}</CardDescription>
        <CardTitle>{value}</CardTitle>
        {active != null && (
          <CardAction>
            <Badge variant={active ? "secondary" : "outline"}>{active ? "Live" : "Idle"}</Badge>
          </CardAction>
        )}
      </CardHeader>
    </Card>
  )
}
