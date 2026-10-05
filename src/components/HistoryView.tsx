import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { formatTimestamp } from "@/lib/format"
import type { EventSummary } from "@/types"

export function HistoryView({
  events,
  onOpen,
}: {
  events: EventSummary[]
  onOpen: (id: string) => void
}) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Local event history</CardTitle>
        <CardDescription>Diagnostic events are JSON files in this app’s local data directory; secrets are never stored.</CardDescription>
      </CardHeader>
      <CardContent>
        {events.length === 0 ? (
          <div className="py-16 text-center text-sm text-muted-foreground">No completed captures yet.</div>
        ) : (
          <div className="rounded-lg border">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Time</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead>Severity</TableHead>
                  <TableHead>Primary process</TableHead>
                  <TableHead className="text-right">Open</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {events.map((event) => (
                  <TableRow key={event.id}>
                    <TableCell>{formatTimestamp(event.clickedAtNs)}</TableCell>
                    <TableCell><Badge variant={event.status === "complete" ? "secondary" : "outline"}>{event.status}</Badge></TableCell>
                    <TableCell>{event.severity?.toFixed(2) ?? "—"}</TableCell>
                    <TableCell>{event.primaryProcess ?? "—"}</TableCell>
                    <TableCell className="text-right">
                      <Button variant="outline" size="sm" onClick={() => onOpen(event.id)}>Open</Button>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        )}
      </CardContent>
    </Card>
  )
}
