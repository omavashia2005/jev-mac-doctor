import { useEffect, useState, type FormEvent } from "react"
import { invoke } from "@tauri-apps/api/core"
import { listen } from "@tauri-apps/api/event"
import { ActivityIcon, DatabaseIcon, HistoryIcon, KeyRoundIcon, LoaderCircleIcon, MoonIcon, RadioTowerIcon, SunIcon } from "lucide-react"
import { CapabilityView } from "@/components/CapabilityView"
import { HistoryView } from "@/components/HistoryView"
import { LiveView } from "@/components/LiveView"
import { RawInspector } from "@/components/RawInspector"
import { ResultsView } from "@/components/ResultsView"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog"
import { Field, FieldDescription, FieldError, FieldGroup, FieldLabel } from "@/components/ui/field"
import { Input } from "@/components/ui/input"
import { Separator } from "@/components/ui/separator"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip"
import type {
  Capability,
  CaptureStatus,
  CollectorSnapshot,
  DiagnosticError,
  EventSummary,
  SlowdownEvent,
  TelemetryFrame,
  TypeSafeCredentialStatus,
} from "@/types"
import "./index.css"

type Screen = "live" | "capabilities" | "results" | "history" | "raw"

function App() {
  const [screen, setScreen] = useState<Screen>("live")
  const [dark, setDark] = useState(true)
  const [snapshot, setSnapshot] = useState<CollectorSnapshot | null>(null)
  const [frames, setFrames] = useState<TelemetryFrame[]>([])
  const [capture, setCapture] = useState<CaptureStatus | null>(null)
  const [selectedEvent, setSelectedEvent] = useState<SlowdownEvent | null>(null)
  const [history, setHistory] = useState<EventSummary[]>([])
  const [credentialStatus, setCredentialStatus] = useState<TypeSafeCredentialStatus | null>(null)

  useEffect(() => {
    document.documentElement.classList.toggle("dark", dark)
  }, [dark])

  useEffect(() => {
    let active = true
    const appendFrame = (frame: TelemetryFrame) => {
      setFrames((current) => [...current.filter((item) => item.timestampNs !== frame.timestampNs), frame].slice(-61))
    }
    const applySnapshot = (nextSnapshot: CollectorSnapshot) => {
      setSnapshot(nextSnapshot)
      if (nextSnapshot.latest) appendFrame(nextSnapshot.latest)
    }
    const refresh = async () => {
      try {
        const [nextSnapshot, events, credentials] = await Promise.all([
          invoke<CollectorSnapshot>("get_snapshot"),
          invoke<EventSummary[]>("list_events"),
          invoke<TypeSafeCredentialStatus>("get_typesafe_credential_status"),
        ])
        if (!active) return
        applySnapshot(nextSnapshot)
        setHistory(events)
        setCredentialStatus(credentials)
      } catch {
        // Vite browser preview has no Tauri IPC; the packaged app does.
      }
    }
    void refresh()
    const snapshotTimer = window.setInterval(() => {
      void invoke<CollectorSnapshot>("get_snapshot").then(applySnapshot).catch(() => undefined)
    }, 5_000)
    const unlisteners = Promise.all([
      listen<TelemetryFrame>("telemetry://sample", ({ payload }) => {
        appendFrame(payload)
        setSnapshot((current) =>
          current
            ? { ...current, running: true, latest: payload, lastSampleTimestampNs: payload.timestampNs }
            : current,
        )
      }),
      listen<Record<string, Capability>>("telemetry://capabilities", ({ payload }) => {
        setSnapshot((current) => (current ? { ...current, capabilities: payload } : current))
      }),
      listen<CaptureStatus>("slowdown://capture-status", ({ payload }) => setCapture(payload)),
      listen<SlowdownEvent>("slowdown://completed", ({ payload }) => {
        setSelectedEvent(payload)
        setScreen("results")
        void invoke<EventSummary[]>("list_events").then(setHistory).catch(() => undefined)
      }),
      listen<DiagnosticError>("collector://error", ({ payload }) => {
        setSnapshot((current) =>
          current ? { ...current, errors: [...current.errors, payload].slice(-100) } : current,
        )
      }),
    ])
    return () => {
      active = false
      window.clearInterval(snapshotTimer)
      void unlisteners.then((items) => items.forEach((unlisten) => unlisten()))
    }
  }, [])

  const markSlowdown = async () => {
    try {
      const started = await invoke<{ id: string; clickedAtNs: number; windowEndNs: number }>("mark_slowdown")
      setCapture({ ...started, status: "collecting-post-window", remainingSeconds: 10 })
    } catch (error) {
      setCapture({
        id: "failed",
        clickedAtNs: Date.now() * 1e6,
        windowEndNs: Date.now() * 1e6,
        status: "failed",
        message: String(error),
      })
    }
  }

  const updateInterval = async (milliseconds: number) => {
    await invoke("set_sample_interval", { milliseconds })
    setSnapshot((current) => (current ? { ...current, sampleIntervalMs: milliseconds } : current))
  }

  const openEvent = async (id: string) => {
    const event = await invoke<SlowdownEvent>("get_event", { id })
    setSelectedEvent(event)
    setScreen("results")
  }

  return (
    <TooltipProvider>
      <div className="min-h-screen bg-background">
        <header className="sticky top-0 z-10 border-b bg-background/95 backdrop-blur">
          <div className="mx-auto flex max-w-[1600px] items-center gap-3 px-5 py-3">
            <div className="flex items-center gap-2">
              <div className="flex size-8 items-center justify-center rounded-lg bg-primary text-primary-foreground">
                <ActivityIcon />
              </div>
              <div>
                <div className="font-heading text-sm font-semibold">Jev Mac Doctor</div>
                <div className="text-xs text-muted-foreground">Read-only slowdown diagnostics</div>
              </div>
            </div>
            <Separator orientation="vertical" className="mx-1 h-7" />
            <Badge variant={snapshot?.running ? "secondary" : "outline"}>
              <span className="mr-1 inline-block size-1.5 rounded-full bg-current" />
              {snapshot?.running ? "Collector running" : "Collector starting"}
            </Badge>
            <div className="ml-auto flex items-center gap-2">
              <TypeSafeKeyDialog status={credentialStatus} onSaved={setCredentialStatus} />
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button variant="ghost" size="icon" onClick={() => setDark((value) => !value)}>
                    {dark ? <SunIcon /> : <MoonIcon />}
                    <span className="sr-only">Toggle color scheme</span>
                  </Button>
                </TooltipTrigger>
                <TooltipContent>Toggle color scheme</TooltipContent>
              </Tooltip>
            </div>
          </div>
        </header>

        <main className="mx-auto max-w-[1600px] px-5 py-4">
          <Tabs value={screen} onValueChange={(value) => setScreen(value as Screen)}>
            <TabsList>
              <TabsTrigger value="live"><RadioTowerIcon />Live</TabsTrigger>
              <TabsTrigger value="capabilities"><ActivityIcon />Capabilities</TabsTrigger>
              <TabsTrigger value="results"><ActivityIcon />Results</TabsTrigger>
              <TabsTrigger value="history"><HistoryIcon />History</TabsTrigger>
              <TabsTrigger value="raw"><DatabaseIcon />Raw data</TabsTrigger>
            </TabsList>
            <TabsContent value="live" className="mt-4">
              <LiveView
                frames={frames}
                snapshot={snapshot}
                capture={capture}
                onCapture={() => void markSlowdown()}
                onIntervalChange={(milliseconds) => void updateInterval(milliseconds)}
              />
            </TabsContent>
            <TabsContent value="capabilities" className="mt-4">
              <CapabilityView capabilities={snapshot?.capabilities ?? {}} />
            </TabsContent>
            <TabsContent value="results" className="mt-4">
              <ResultsView key={selectedEvent?.id ?? "empty"} event={selectedEvent} />
            </TabsContent>
            <TabsContent value="history" className="mt-4">
              <HistoryView events={history} onOpen={(id) => void openEvent(id)} />
            </TabsContent>
            <TabsContent value="raw" className="mt-4">
              <RawInspector event={selectedEvent} />
            </TabsContent>
          </Tabs>
        </main>
      </div>
    </TooltipProvider>
  )
}

function TypeSafeKeyDialog({
  status,
  onSaved,
}: {
  status: TypeSafeCredentialStatus | null
  onSaved: (status: TypeSafeCredentialStatus) => void
}) {
  const [open, setOpen] = useState(false)
  const [apiKey, setApiKey] = useState("")
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const save = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    if (!apiKey.trim()) {
      setError("Enter a TypeSafe API key.")
      return
    }
    setSaving(true)
    setError(null)
    try {
      const next = await invoke<TypeSafeCredentialStatus>("set_typesafe_api_key", { apiKey })
      setApiKey("")
      onSaved(next)
      setOpen(false)
    } catch (saveError) {
      setError(String(saveError))
    } finally {
      setSaving(false)
    }
  }

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        setOpen(next)
        if (!next) {
          setApiKey("")
          setError(null)
        }
      }}
    >
      <DialogTrigger asChild>
        <Button variant="outline" size="sm">
          <KeyRoundIcon data-icon="inline-start" />
          TypeSafe
          <Badge variant={status?.configured ? "secondary" : "outline"}>
            {status?.configured ? "Configured" : "Set key"}
          </Badge>
        </Button>
      </DialogTrigger>
      <DialogContent>
        <form className="contents" onSubmit={save}>
          <DialogHeader>
            <DialogTitle>TypeSafe API key</DialogTitle>
            <DialogDescription>
              Saved to your macOS Keychain. The key is never included in telemetry, logs, or event history.
            </DialogDescription>
          </DialogHeader>
          <FieldGroup>
            <Field data-invalid={Boolean(error)}>
              <FieldLabel htmlFor="typesafe-api-key">API key</FieldLabel>
              <Input
                id="typesafe-api-key"
                type="password"
                value={apiKey}
                placeholder={status?.configured ? "Enter a replacement key" : "Paste your TypeSafe API key"}
                autoComplete="off"
                spellCheck={false}
                aria-invalid={Boolean(error)}
                onChange={(event) => setApiKey(event.target.value)}
              />
              <FieldDescription>
                {status?.configured ? `Currently configured via ${status.source}.` : "Required only for Jev analysis."}
              </FieldDescription>
              {error && <FieldError>{error}</FieldError>}
            </Field>
          </FieldGroup>
          <DialogFooter>
            <DialogClose asChild>
              <Button type="button" variant="outline">Cancel</Button>
            </DialogClose>
            <Button type="submit" disabled={saving || !apiKey.trim()}>
              {saving && <LoaderCircleIcon data-icon="inline-start" className="animate-spin" />}
              Save to Keychain
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}

export default App
