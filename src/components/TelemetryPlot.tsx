import { useEffect, useLayoutEffect, useMemo, useRef } from "react"
import uPlot, { type AlignedData, type Options } from "uplot"
import "uplot/dist/uPlot.min.css"

export interface PlotSeries {
  label: string
  values: Array<number | null>
  color: string
}

interface TelemetryPlotProps {
  timestamps: number[]
  series: PlotSeries[]
  markerTimestamp?: number | null
  height?: number
  formatValue?: (value: number) => string
}

export function TelemetryPlot({
  timestamps,
  series,
  markerTimestamp,
  height = 150,
  formatValue = (value) => value.toLocaleString(undefined, { maximumFractionDigits: 1 }),
}: TelemetryPlotProps) {
  const host = useRef<HTMLDivElement>(null)
  const plot = useRef<uPlot>(null)
  const marker = useRef<number | null>(markerTimestamp ?? null)
  marker.current = markerTimestamp ?? null

  const first = timestamps[0] ?? 0
  const hasData = timestamps.length >= 2 && series.some((item) => item.values.some((value) => value != null))
  const data = useMemo(
    () => [
      timestamps.map((timestamp) => (timestamp - first) / 1e9),
      ...series.map((item) => item.values),
    ] as AlignedData,
    [first, series, timestamps],
  )

  useLayoutEffect(() => {
    if (!host.current) return
    const element = host.current
    const options: Options = {
      width: Math.max(element.clientWidth, 240),
      height,
      cursor: { drag: { x: false, y: false } },
      legend: { show: false },
      scales: { x: { time: false } },
      axes: [
        {
          stroke: "var(--muted-foreground)",
          grid: { stroke: "var(--border)", width: 1 },
          values: (_plot, values) => values.map((value) => `${value.toFixed(0)}s`),
        },
        {
          stroke: "var(--muted-foreground)",
          grid: { stroke: "var(--border)", width: 1 },
          size: 48,
        },
      ],
      series: [
        {},
        ...series.map((item) => ({
          label: item.label,
          stroke: item.color,
          width: 1.5,
          points: { show: false },
        })),
      ],
      hooks: {
        draw: [
          (chart) => {
            if (marker.current == null || first === 0) return
            const relative = (marker.current - first) / 1e9
            if (relative < chart.scales.x.min! || relative > chart.scales.x.max!) return
            const x = Math.round(chart.valToPos(relative, "x", true))
            const context = chart.ctx
            context.save()
            context.beginPath()
            context.setLineDash([4, 4])
            context.strokeStyle = "#ef4444"
            context.lineWidth = 1
            context.moveTo(x, chart.bbox.top)
            context.lineTo(x, chart.bbox.top + chart.bbox.height)
            context.stroke()
            context.restore()
          },
        ],
      },
    }
    plot.current = new uPlot(options, data, element)
    const observer = new ResizeObserver(([entry]) => {
      plot.current?.setSize({ width: Math.max(entry.contentRect.width, 240), height })
    })
    observer.observe(element)
    return () => {
      observer.disconnect()
      plot.current?.destroy()
      plot.current = null
    }
    // Series definitions are stable per mounted chart; values update below.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [hasData, height, series.length])

  useEffect(() => {
    plot.current?.setData(data)
    plot.current?.redraw()
  }, [data])

  if (!hasData) {
    return (
      <div className="flex h-36 items-center justify-center text-xs text-muted-foreground">
        Metric unavailable or waiting for a second sample
      </div>
    )
  }

  return (
    <div className="flex flex-col gap-2">
      <div ref={host} className="w-full overflow-hidden" />
      <div className="flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted-foreground">
        {series.map((item) => {
          const latest = latestValue(item.values)
          return (
            <span key={item.label}>
              {item.label}: <span className="font-medium text-foreground">{latest == null ? "—" : formatValue(latest)}</span>
            </span>
          )
        })}
      </div>
    </div>
  )
}

function latestValue(values: Array<number | null>) {
  for (let index = values.length - 1; index >= 0; index -= 1) {
    if (values[index] != null) return values[index]
  }
  return null
}
