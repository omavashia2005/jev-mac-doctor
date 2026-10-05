import { CopyIcon } from "lucide-react"
import { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from "@/components/ui/accordion"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import type { SlowdownEvent } from "@/types"

export function RawInspector({ event }: { event: SlowdownEvent | null }) {
  if (!event) {
    return <div className="py-16 text-center text-sm text-muted-foreground">Select a diagnostic event to inspect raw data.</div>
  }
  const request = event.jevRequest as { state?: unknown; questions?: unknown } | undefined
  const sections: Array<[string, string, unknown]> = [
    ["system", "Raw system telemetry", event.systemSamples],
    ["processes", "Raw process telemetry", event.processes],
    ["capabilities", "Capability state", event.capabilities],
    ["state", "Normalized / compacted Jev state", request?.state],
    ["request", "Exact Jev request", event.jevRequest],
    ["questions", "Jev question definitions", request?.questions],
    ["raw-response", "Raw Jev response", event.jevResult?.rawResponse],
    ["normalized-response", "Normalized Jev response", event.jevResult],
    ["errors", "Collector and analysis errors", event.errors],
  ]
  return (
    <Card>
      <CardHeader>
        <CardTitle>Technical inspector</CardTitle>
        <CardDescription>Exact retained inputs, outputs, capability state, and errors for this experiment.</CardDescription>
      </CardHeader>
      <CardContent>
        <Accordion type="multiple">
          {sections.map(([id, label, value]) => (
            <AccordionItem key={id} value={id}>
              <AccordionTrigger>{label}</AccordionTrigger>
              <AccordionContent>
                <div className="mb-2 flex justify-end">
                  <Button variant="outline" size="sm" onClick={() => copyJson(value)}>
                    <CopyIcon data-icon="inline-start" />
                    Copy JSON
                  </Button>
                </div>
                <pre className="max-h-[560px] overflow-auto rounded-lg bg-muted p-3 text-xs leading-relaxed">
                  {JSON.stringify(value ?? null, null, 2)}
                </pre>
              </AccordionContent>
            </AccordionItem>
          ))}
        </Accordion>
      </CardContent>
    </Card>
  )
}

function copyJson(value: unknown) {
  void navigator.clipboard.writeText(JSON.stringify(value ?? null, null, 2))
}
