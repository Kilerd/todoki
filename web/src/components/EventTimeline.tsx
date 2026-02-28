import { emitEvent } from "@/api/eventBus";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { useEventStream, type Event } from "@/hooks/useEventStream";
import { cn } from "@/lib/utils";
import dayjs from "dayjs";
import {
  Activity,
  AlertCircle,
  Bot,
  CheckCircle2,
  Circle,
  FileText,
  PlayCircle,
  RefreshCw,
  Shield,
  StopCircle,
  XCircle,
} from "lucide-react";
import { useMemo, useState } from "react";

interface EventTimelineProps {
  /** Event kind patterns to subscribe (e.g., ["task.*", "agent.*"]) */
  kinds?: string[];

  /** Starting cursor for historical replay */
  cursor?: number;

  /** Filter by agent ID */
  agentId?: string;

  /** Filter by task ID */
  taskId?: string;

  /** Authentication token */
  token?: string;

  /** Show connection status */
  showStatus?: boolean;

  /** Max events to display (default: 50) */
  maxEvents?: number;

  /** Enable auto-scroll to latest event */
  autoScroll?: boolean;
}

function getEventIcon(kind: string) {
  if (kind.startsWith("task.")) {
    if (kind === "task.created") return <FileText className="h-4 w-4 text-blue-500" />;
    if (kind === "task.completed") return <CheckCircle2 className="h-4 w-4 text-green-500" />;
    if (kind === "task.failed") return <XCircle className="h-4 w-4 text-red-500" />;
    return <FileText className="h-4 w-4 text-slate-500" />;
  }

  if (kind.startsWith("agent.")) {
    if (kind === "agent.started") return <PlayCircle className="h-4 w-4 text-green-500" />;
    if (kind === "agent.stopped") return <StopCircle className="h-4 w-4 text-yellow-500" />;
    if (kind === "agent.requirement_analyzed")
      return <Bot className="h-4 w-4 text-purple-500" />;
    return <Bot className="h-4 w-4 text-slate-500" />;
  }

  if (kind.startsWith("artifact.")) {
    return <FileText className="h-4 w-4 text-indigo-500" />;
  }

  if (kind.startsWith("permission.")) {
    if (kind === "permission.requested") return <Shield className="h-4 w-4 text-orange-500" />;
    if (kind === "permission.responded") return <Shield className="h-4 w-4 text-green-500" />;
    return <Shield className="h-4 w-4 text-slate-500" />;
  }

  if (kind.startsWith("system.")) {
    return <Activity className="h-4 w-4 text-slate-500" />;
  }

  return <Circle className="h-4 w-4 text-slate-400" />;
}

function getEventColor(kind: string): string {
  if (kind.includes("created")) return "border-l-blue-400";
  if (kind.includes("completed") || kind.includes("started")) return "border-l-green-400";
  if (kind.includes("failed")) return "border-l-red-400";
  if (kind.includes("stopped")) return "border-l-yellow-400";
  if (kind.includes("analyzed")) return "border-l-purple-400";
  return "border-l-slate-300";
}

function PermissionActions({ event }: { event: Event }) {
  const [isLoading, setIsLoading] = useState(false);
  const [responded, setResponded] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleRespond = async (outcome: "allow" | "allow_always" | "reject") => {
    setIsLoading(true);
    setError(null);

    try {
      const { agent_id, request_id, session_id } = event.data as {
        agent_id?: string;
        request_id?: string;
        session_id?: string;
      };

      if (!agent_id || !request_id || !session_id) {
        throw new Error("Missing required fields in permission request");
      }

      await emitEvent({
        kind: "permission.responded",
        data: {
          agent_id,
          request_id,
          session_id,
          outcome: outcome === "reject" ? { cancelled: true } : { selected: { option_id: outcome } },
        },
      });

      setResponded(true);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to respond");
    } finally {
      setIsLoading(false);
    }
  };

  if (responded) {
    return (
      <Badge variant="outline" className="text-green-600 border-green-300">
        <CheckCircle2 className="h-3 w-3 mr-1" />
        Responded
      </Badge>
    );
  }

  return (
    <div className="flex items-center gap-2 mt-2">
      <Button
        size="sm"
        variant="outline"
        className="h-7 text-xs cursor-pointer text-green-600 border-green-300 hover:bg-green-50"
        onClick={() => handleRespond("allow")}
        disabled={isLoading}
      >
        Allow
      </Button>
      <Button
        size="sm"
        variant="outline"
        className="h-7 text-xs cursor-pointer text-blue-600 border-blue-300 hover:bg-blue-50"
        onClick={() => handleRespond("allow_always")}
        disabled={isLoading}
      >
        Always Allow
      </Button>
      <Button
        size="sm"
        variant="outline"
        className="h-7 text-xs cursor-pointer text-red-600 border-red-300 hover:bg-red-50"
        onClick={() => handleRespond("reject")}
        disabled={isLoading}
      >
        Reject
      </Button>
      {error && <span className="text-xs text-red-500">{error}</span>}
    </div>
  );
}

function EventItem({ event }: { event: Event }) {
  const formattedTime = useMemo(
    () => dayjs(event.time).format("HH:mm:ss"),
    [event.time]
  );

  const hasDetails = event.data && Object.keys(event.data).length > 0;
  const isPermissionRequest = event.kind === "permission.requested";

  // Extract tool call info for permission requests
  const toolCallInfo = useMemo(() => {
    if (!isPermissionRequest) return null;
    const data = event.data as { tool_call?: { title?: string } };
    return data?.tool_call?.title;
  }, [event.data, isPermissionRequest]);

  return (
    <div
      className={cn(
        "border-l-2 pl-4 py-3 transition-colors hover:bg-slate-50",
        getEventColor(event.kind)
      )}
    >
      <div className="flex items-start gap-3">
        <div className="mt-0.5">{getEventIcon(event.kind)}</div>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 mb-1">
            <span className="text-sm font-medium text-slate-700">
              {event.kind}
            </span>
            <span className="text-xs text-slate-400 font-mono">
              #{event.cursor}
            </span>
          </div>

          <div className="flex items-center gap-2 text-xs text-slate-500 mb-2">
            <span className="font-mono">{formattedTime}</span>
            <span>•</span>
            <span className="font-mono">
              {event.agent_id.slice(0, 8)}
            </span>
            {event.task_id && (
              <>
                <span>•</span>
                <span className="font-mono">
                  Task: {event.task_id.slice(0, 8)}
                </span>
              </>
            )}
          </div>

          {/* Show tool call title for permission requests */}
          {toolCallInfo && (
            <div className="text-xs text-slate-600 bg-slate-50 p-2 rounded mb-2 font-mono">
              {toolCallInfo}
            </div>
          )}

          {/* Permission action buttons */}
          {isPermissionRequest && <PermissionActions event={event} />}

          {hasDetails && (
            <details className="text-xs text-slate-600 mt-2">
              <summary className="cursor-pointer hover:text-slate-800 select-none">
                View data
              </summary>
              <pre className="mt-2 p-2 bg-slate-50 rounded border border-slate-200 overflow-x-auto">
                {JSON.stringify(event.data, null, 2)}
              </pre>
            </details>
          )}
        </div>
      </div>
    </div>
  );
}

export function EventTimeline({
  kinds,
  cursor,
  agentId,
  taskId,
  token,
  showStatus = true,
  maxEvents = 50,
}: EventTimelineProps) {
  const { events, isConnected, isReplaying, error, reconnect, clearEvents } =
    useEventStream({
      kinds,
      cursor,
      agentId,
      taskId,
      token,
    });

  const displayedEvents = useMemo(() => {
    return events.slice(-maxEvents);
  }, [events, maxEvents]);

  return (
    <div className="space-y-4">
      {/* Status Bar */}
      {showStatus && (
        <Card className="p-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <div className="flex items-center gap-2">
                <Circle
                  className={cn(
                    "h-2 w-2",
                    isConnected ? "text-green-500" : "text-red-500"
                  )}
                  fill="currentColor"
                />
                <span className="text-sm font-medium text-slate-700">
                  {isConnected ? "Connected" : "Disconnected"}
                </span>
              </div>

              {isReplaying && (
                <Badge variant="outline" className="text-xs">
                  <RefreshCw className="h-3 w-3 mr-1 animate-spin" />
                  Replaying history
                </Badge>
              )}

              <span className="text-xs text-slate-500">
                {displayedEvents.length} events
                {displayedEvents.length >= maxEvents && ` (last ${maxEvents})`}
              </span>
            </div>

            <div className="flex items-center gap-2">
              {!isConnected && (
                <Button
                  size="sm"
                  variant="outline"
                  onClick={reconnect}
                  className="cursor-pointer"
                >
                  <RefreshCw className="h-3 w-3 mr-1.5" />
                  Reconnect
                </Button>
              )}
              <Button
                size="sm"
                variant="ghost"
                onClick={clearEvents}
                className="cursor-pointer"
              >
                Clear
              </Button>
            </div>
          </div>

          {error && (
            <div className="flex items-center gap-2 mt-3 text-sm text-red-600 bg-red-50 p-2 rounded">
              <AlertCircle className="h-4 w-4" />
              {error}
            </div>
          )}
        </Card>
      )}

      {/* Event List */}
      <Card className="p-0 overflow-hidden">
        {displayedEvents.length === 0 ? (
          <div className="p-8 text-center text-slate-400">
            {isReplaying ? (
              <div className="space-y-2">
                <Skeleton className="h-12 w-full" />
                <Skeleton className="h-12 w-full" />
                <Skeleton className="h-12 w-full" />
              </div>
            ) : (
              <div className="flex flex-col items-center gap-2">
                <Activity className="h-8 w-8 text-slate-300" />
                <p className="text-sm">No events yet</p>
                {!isConnected && (
                  <Button
                    size="sm"
                    variant="outline"
                    onClick={reconnect}
                    className="mt-2 cursor-pointer"
                  >
                    Connect
                  </Button>
                )}
              </div>
            )}
          </div>
        ) : (
          <div className="divide-y divide-slate-100">
            {displayedEvents.map((event) => (
              <EventItem key={event.cursor} event={event} />
            ))}
          </div>
        )}
      </Card>
    </div>
  );
}
