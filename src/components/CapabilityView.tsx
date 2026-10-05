import { CheckCircle2Icon, CircleHelpIcon, ShieldCheckIcon, XCircleIcon } from "lucide-react"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Badge } from "@/components/ui/badge"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { titleCase } from "@/lib/format"
import type { Capability } from "@/types"

export function CapabilityView({ capabilities }: { capabilities: Record<string, Capability> }) {
  const entries = Object.entries(capabilities).sort(([left], [right]) => left.localeCompare(right))
  return (
    <div className="flex flex-col gap-4">
      <Alert>
        <ShieldCheckIcon />
        <AlertTitle>Read-only telemetry boundary</AlertTitle>
        <AlertDescription>
          Collectors only query libproc, Mach host statistics, read-only sysctls, interface counters, and IORegistry
          properties. Missing access remains unavailable; the app never mutates system or process state.
        </AlertDescription>
      </Alert>
      <Card>
        <CardHeader>
          <CardTitle>Telemetry sources</CardTitle>
          <CardDescription>Detected at runtime on this Mac. Unavailable does not mean zero.</CardDescription>
        </CardHeader>
        <CardContent className="grid gap-2 lg:grid-cols-2">
          {entries.map(([name, capability]) => (
            <div key={name} className="flex items-start gap-3 rounded-lg border p-3">
              {capability.available ? (
                <CheckCircle2Icon className="mt-0.5 text-foreground" />
              ) : capability.reason ? (
                <XCircleIcon className="mt-0.5 text-muted-foreground" />
              ) : (
                <CircleHelpIcon className="mt-0.5 text-muted-foreground" />
              )}
              <div className="min-w-0 flex-1">
                <div className="font-medium">{titleCase(name)}</div>
                <div className="text-xs text-muted-foreground">{capability.source ?? "No source"}</div>
                {capability.reason && <div className="mt-1 text-xs text-muted-foreground">{capability.reason}</div>}
                <div className="mt-2 flex gap-1">
                  {capability.privateApi && <Badge variant="outline">Private API</Badge>}
                  {capability.privileged && <Badge variant="outline">Privilege required</Badge>}
                  <Badge variant={capability.available ? "secondary" : "outline"}>
                    {capability.available ? "Available" : "Unavailable"}
                  </Badge>
                </div>
              </div>
            </div>
          ))}
        </CardContent>
      </Card>
    </div>
  )
}
