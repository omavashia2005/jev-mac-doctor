import { useMemo, useState } from "react"
import {
  createColumnHelper,
  flexRender,
  getCoreRowModel,
  getSortedRowModel,
  useReactTable,
  type SortingState,
} from "@tanstack/react-table"
import { ArrowUpDownIcon } from "lucide-react"
import { Button } from "@/components/ui/button"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import { formatBytes, formatPercent, formatRate } from "@/lib/format"
import type { ProcessSample } from "@/types"

const column = createColumnHelper<ProcessSample>()

export function ProcessTable({ processes }: { processes: ProcessSample[] }) {
  const [sorting, setSorting] = useState<SortingState>([{ id: "cpu", desc: true }])
  const columns = useMemo(
    () => [
      column.accessor((row) => row.identity.name, {
        id: "process",
        header: "Process",
        cell: ({ row, getValue }) => (
          <div className="min-w-36">
            <div className="truncate font-medium">{getValue()}</div>
            <div className="truncate text-xs text-muted-foreground">{row.original.identity.executable ?? "—"}</div>
          </div>
        ),
      }),
      column.accessor((row) => row.identity.pid, { id: "pid", header: "PID" }),
      column.accessor((row) => row.cpu.percent ?? -1, {
        id: "cpu",
        header: "CPU",
        cell: ({ row }) => formatPercent(row.original.cpu.percent),
      }),
      column.accessor((row) => row.gpu.utilizationPercent ?? -1, {
        id: "gpu",
        header: "GPU",
        cell: ({ row }) => formatPercent(row.original.gpu.utilizationPercent),
      }),
      column.accessor((row) => row.memory.physicalFootprintBytes ?? -1, {
        id: "memory",
        header: "Memory",
        cell: ({ row }) => formatBytes(row.original.memory.physicalFootprintBytes),
      }),
      column.accessor(
        (row) => (row.diskIo.readBytesPerSecond ?? 0) + (row.diskIo.writeBytesPerSecond ?? 0),
        {
          id: "disk",
          header: "Disk R/W",
          cell: ({ row }) =>
            `${formatRate(row.original.diskIo.readBytesPerSecond)} / ${formatRate(row.original.diskIo.writeBytesPerSecond)}`,
        },
      ),
      column.accessor(
        (row) => (row.network.rxBytesPerSecond ?? 0) + (row.network.txBytesPerSecond ?? 0),
        {
          id: "network",
          header: "Network R/W",
          cell: ({ row }) =>
            row.original.network.available
              ? `${formatRate(row.original.network.rxBytesPerSecond)} / ${formatRate(row.original.network.txBytesPerSecond)}`
              : "—",
        },
      ),
      column.accessor((row) => row.scheduling.interruptWakeupsPerSecond ?? -1, {
        id: "wakeups",
        header: "Wakeups/s",
        cell: ({ row }) => row.original.scheduling.interruptWakeupsPerSecond?.toFixed(1) ?? "—",
      }),
      column.accessor((row) => row.energy.energyNj ?? -1, {
        id: "energy",
        header: "Energy",
        cell: ({ row }) =>
          row.original.energy.energyNj == null ? "—" : `${(row.original.energy.energyNj / 1e6).toFixed(1)} mJ`,
      }),
      column.accessor(
        (row) => Object.values(row.qos).reduce<number>((sum, value) => sum + (value ?? 0), 0),
        {
          id: "qos",
          header: "QoS CPU",
          cell: ({ getValue }) => (getValue() ? `${(getValue() / 1e9).toFixed(1)} s` : "—"),
        },
      ),
      column.accessor((row) => row.neural.footprintBytes ?? -1, {
        id: "neural",
        header: "Neural footprint",
        cell: ({ row }) => formatBytes(row.original.neural.footprintBytes),
      }),
    ],
    [],
  )
  const table = useReactTable({
    data: processes,
    columns,
    state: { sorting },
    onSortingChange: setSorting,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
  })

  return (
    <div className="max-h-[430px] overflow-auto rounded-lg border">
      <Table>
        <TableHeader className="sticky top-0 bg-card">
          {table.getHeaderGroups().map((group) => (
            <TableRow key={group.id}>
              {group.headers.map((header) => (
                <TableHead key={header.id}>
                  {header.isPlaceholder ? null : header.column.getCanSort() ? (
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={header.column.getToggleSortingHandler()}
                    >
                      {flexRender(header.column.columnDef.header, header.getContext())}
                      <ArrowUpDownIcon data-icon="inline-end" />
                    </Button>
                  ) : (
                    flexRender(header.column.columnDef.header, header.getContext())
                  )}
                </TableHead>
              ))}
            </TableRow>
          ))}
        </TableHeader>
        <TableBody>
          {table.getRowModel().rows.map((row) => (
            <TableRow key={row.original.identity.id}>
              {row.getVisibleCells().map((cell) => (
                <TableCell key={cell.id} className="whitespace-nowrap">
                  {flexRender(cell.column.columnDef.cell, cell.getContext())}
                </TableCell>
              ))}
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  )
}
