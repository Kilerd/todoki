import { emitEvent, queryEvents, type EventBusEvent } from "@/api/eventBus";
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
  ClipboardList,
  Code,
  FileText,
  History,
  MessageSquare,
  PlayCircle,
  RefreshCw,
  Shield,
  StopCircle,
  Terminal,
  XCircle,
  Wrench,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";

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

// Types for agent.output_batch messages
interface OutputBatchData {
  messages: string[];
  session_id: string;
  stream: "system" | "assistant" | "plan" | "tool_use" | "tool_result";
  ts: number;
}

interface AgentMessage {
  type: "agent_message";
  chunk?: boolean;
  text: string;
}

interface PlanMessage {
  type: "plan";
  plan: {
    entries: Array<{
      content: string;
      priority: string;
      status: string;
    }>;
  };
}

interface ToolCallMessage {
  type: "tool_call" | "tool_call_update";
  id: string;
  kind: string;
  title: string;
  status: "pending" | "completed" | "error";
  raw_input?: Record<string, unknown>;
  raw_output?: string | null;
  content?: Array<{ type: string; content: unknown }>;
  meta?: {
    claudeCode?: {
      toolName?: string;
      toolResponse?: unknown;
    };
  };
}

function OutputBatchContent({ data }: { data: OutputBatchData }) {
  const parsedMessages = useMemo(() => {
    return data.messages.map((msg) => {
      try {
        return JSON.parse(msg);
      } catch {
        return { type: "raw", text: msg };
      }
    });
  }, [data.messages]);

  const streamIcon = useMemo(() => {
    switch (data.stream) {
      case "system":
        return <Terminal className="h-3 w-3 text-slate-500" />;
      case "assistant":
        return <MessageSquare className="h-3 w-3 text-blue-500" />;
      case "plan":
        return <ClipboardList className="h-3 w-3 text-purple-500" />;
      case "tool_use":
        return <Wrench className="h-3 w-3 text-orange-500" />;
      case "tool_result":
        return <Code className="h-3 w-3 text-green-500" />;
      default:
        return <Circle className="h-3 w-3 text-slate-400" />;
    }
  }, [data.stream]);

  // Render assistant messages (combine chunks into text)
  if (data.stream === "assistant") {
    const text = parsedMessages
      .filter((m): m is AgentMessage => m.type === "agent_message")
      .map((m) => m.text)
      .join("");

    if (!text.trim()) return null;

    return (
      <div className="flex items-start gap-2 mt-2">
        {streamIcon}
        <div className="flex-1 text-sm text-slate-700 whitespace-pre-wrap">
          {text}
        </div>
      </div>
    );
  }

  // Render plan messages
  if (data.stream === "plan") {
    const planMsg = parsedMessages.find((m): m is PlanMessage => m.type === "plan");
    if (!planMsg?.plan?.entries) return null;

    return (
      <div className="mt-2">
        <div className="flex items-center gap-2 mb-2">
          {streamIcon}
          <span className="text-xs font-medium text-purple-600">Plan</span>
        </div>
        <div className="space-y-1 pl-5">
          {planMsg.plan.entries.map((entry, idx) => (
            <div
              key={idx}
              className={cn(
                "flex items-center gap-2 text-xs",
                entry.status === "completed" ? "text-green-600" : "text-slate-600"
              )}
            >
              {entry.status === "completed" ? (
                <CheckCircle2 className="h-3 w-3" />
              ) : (
                <Circle className="h-3 w-3" />
              )}
              <span>{entry.content}</span>
            </div>
          ))}
        </div>
      </div>
    );
  }

  // Render tool_use messages
  if (data.stream === "tool_use") {
    const toolCalls = parsedMessages.filter(
      (m): m is ToolCallMessage => m.type === "tool_call" || m.type === "tool_call_update"
    );

    // Dedupe by id, keep latest
    const uniqueTools = new Map<string, ToolCallMessage>();
    for (const tool of toolCalls) {
      uniqueTools.set(tool.id, tool);
    }

    if (uniqueTools.size === 0) return null;

    return (
      <div className="mt-2 space-y-2">
        {Array.from(uniqueTools.values()).map((tool) => (
          <div key={tool.id} className="flex items-start gap-2">
            {streamIcon}
            <div className="flex-1">
              <div className="flex items-center gap-2">
                <span className="text-xs font-medium text-orange-600">
                  {tool.title || tool.kind}
                </span>
                <Badge
                  variant="outline"
                  className={cn(
                    "text-[10px] h-4",
                    tool.status === "completed"
                      ? "text-green-600 border-green-300"
                      : tool.status === "error"
                        ? "text-red-600 border-red-300"
                        : "text-slate-500 border-slate-300"
                  )}
                >
                  {tool.status}
                </Badge>
              </div>
              {tool.raw_input && Object.keys(tool.raw_input).length > 0 && (
                <div className="text-[10px] text-slate-500 font-mono mt-1 truncate max-w-md">
                  {Object.entries(tool.raw_input)
                    .map(([k, v]) => `${k}: ${typeof v === "string" ? v : JSON.stringify(v)}`)
                    .join(", ")}
                </div>
              )}
            </div>
          </div>
        ))}
      </div>
    );
  }

  // Render tool_result messages (collapsed by default)
  if (data.stream === "tool_result") {
    const results = parsedMessages.filter(
      (m): m is ToolCallMessage => m.type === "tool_call_update" && m.status === "completed"
    );

    if (results.length === 0) return null;

    return (
      <div className="mt-2">
        <details className="text-xs">
          <summary className="flex items-center gap-2 cursor-pointer hover:text-slate-800">
            {streamIcon}
            <span className="text-green-600">
              {results.length} tool result{results.length > 1 ? "s" : ""}
            </span>
          </summary>
          <div className="mt-1 pl-5 space-y-1">
            {results.map((result) => (
              <div key={result.id} className="text-[10px] text-slate-500 font-mono">
                {result.meta?.claudeCode?.toolName || result.kind}
              </div>
            ))}
          </div>
        </details>
      </div>
    );
  }

  // Render system messages (collapsed by default)
  if (data.stream === "system") {
    return (
      <div className="mt-2">
        <details className="text-xs">
          <summary className="flex items-center gap-2 cursor-pointer hover:text-slate-800">
            {streamIcon}
            <span className="text-slate-500">System message</span>
          </summary>
          <pre className="mt-1 pl-5 text-[10px] text-slate-500 font-mono overflow-x-auto max-h-32">
            {JSON.stringify(parsedMessages, null, 2)}
          </pre>
        </details>
      </div>
    );
  }

  return null;
}

function PermissionActions({ event }: { event: Event }) {
  const [isLoading, setIsLoading] = useState(false);
  const [responded, setResponded] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleRespond = async (outcome: "allow" | "allow_always" | "reject") => {
    setIsLoading(true);
    setError(null);

    try {
      const { request_id, session_id } = event.data as {
        request_id?: string;
        session_id?: string;
      };

      if (!request_id || !session_id) {
        throw new Error("Missing required fields in permission request");
      }

      await emitEvent({
        agent_id: event.agent_id,
        session_id,
        task_id: event.task_id,
        kind: "permission.responded",
        data: {
          relay_id: "",
          request_id: request_id,
          session_id: session_id,
          outcome: outcome === "reject" ? { cancelled: true } : { selected: { option_id: outcome } },
        },
      }); // relay_id injected by backend

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
  const isOutputBatch = event.kind === "agent.output_batch";

  // Extract tool call info for permission requests
  const toolCallInfo = useMemo(() => {
    if (!isPermissionRequest) return null;
    const data = event.data as { tool_call?: { title?: string } };
    return data?.tool_call?.title;
  }, [event.data, isPermissionRequest]);

  // Get stream type badge for output_batch
  const outputBatchData = isOutputBatch ? (event.data as OutputBatchData) : null;

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
            {outputBatchData && (
              <Badge
                variant="outline"
                className={cn(
                  "text-[10px] h-4",
                  outputBatchData.stream === "assistant" && "text-blue-600 border-blue-300",
                  outputBatchData.stream === "plan" && "text-purple-600 border-purple-300",
                  outputBatchData.stream === "tool_use" && "text-orange-600 border-orange-300",
                  outputBatchData.stream === "tool_result" && "text-green-600 border-green-300",
                  outputBatchData.stream === "system" && "text-slate-500 border-slate-300"
                )}
              >
                {outputBatchData.stream}
              </Badge>
            )}
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

          {/* Specialized output_batch rendering */}
          {isOutputBatch && outputBatchData && (
            <OutputBatchContent data={outputBatchData} />
          )}

          {/* Generic data view for non-output_batch events */}
          {hasDetails && !isOutputBatch && (
            <details className="text-xs text-slate-600 mt-2">
              <summary className="cursor-pointer hover:text-slate-800 select-none">
                View data
              </summary>
              <pre className="mt-2 p-2 bg-slate-50 rounded border border-slate-200 overflow-x-auto">
                {JSON.stringify(event.data, null, 2)}
              </pre>
            </details>
          )}

          {/* Raw data toggle for output_batch */}
          {isOutputBatch && hasDetails && (
            <details className="text-xs text-slate-400 mt-2">
              <summary className="cursor-pointer hover:text-slate-600 select-none">
                Raw data
              </summary>
              <pre className="mt-2 p-2 bg-slate-50 rounded border border-slate-200 overflow-x-auto text-[10px]">
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

  // Historical events state
  const [historicalEvents, setHistoricalEvents] = useState<Event[]>([]);
  const [isLoadingHistory, setIsLoadingHistory] = useState(false);
  const [historyError, setHistoryError] = useState<string | null>(null);
  const [hasLoadedHistory, setHasLoadedHistory] = useState(false);

  // Load historical events on mount
  useEffect(() => {
    if (hasLoadedHistory) return;

    const loadHistory = async () => {
      setIsLoadingHistory(true);
      setHistoryError(null);
      try {
        const kindsParam = kinds?.join(",");
        const response = await queryEvents({
          cursor: 0,
          kinds: kindsParam,
          task_id: taskId,
          agent_id: agentId,
          limit: maxEvents,
        });

        const converted: Event[] = response.events.map((e: EventBusEvent) => ({
          cursor: e.cursor,
          kind: e.kind,
          time: e.time,
          agent_id: e.agent_id,
          session_id: e.session_id,
          task_id: e.task_id,
          data: e.data,
        }));
        setHistoricalEvents(converted);
        setHasLoadedHistory(true);
      } catch (err) {
        setHistoryError(
          err instanceof Error ? err.message : "Failed to load history"
        );
      } finally {
        setIsLoadingHistory(false);
      }
    };

    loadHistory();
  }, [kinds, taskId, agentId, maxEvents, hasLoadedHistory]);

  // Merge historical and real-time events, dedupe by cursor
  const allEvents = useMemo(() => {
    const merged = [...historicalEvents, ...events];
    const seen = new Set<number>();
    return merged
      .filter((e) => {
        if (seen.has(e.cursor)) return false;
        seen.add(e.cursor);
        return true;
      })
      .sort((a, b) => a.cursor - b.cursor);
  }, [historicalEvents, events]);

  const displayedEvents = useMemo(() => {
    return allEvents.slice(-maxEvents);
  }, [allEvents, maxEvents]);

  const handleClearAll = () => {
    clearEvents();
    setHistoricalEvents([]);
    setHasLoadedHistory(false);
  };

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

              {(isReplaying || isLoadingHistory) && (
                <Badge variant="outline" className="text-xs">
                  <RefreshCw className="h-3 w-3 mr-1 animate-spin" />
                  {isLoadingHistory ? "Loading history" : "Replaying"}
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
                onClick={handleClearAll}
                className="cursor-pointer"
              >
                Clear
              </Button>
            </div>
          </div>

          {/* Subscribed kinds */}
          {kinds && kinds.length > 0 && (
            <div className="flex items-center gap-2 mt-3 flex-wrap">
              <span className="text-xs text-slate-500">Subscribed:</span>
              {kinds.map((kind) => (
                <Badge
                  key={kind}
                  variant="secondary"
                  className="text-xs font-mono"
                >
                  {kind}
                </Badge>
              ))}
            </div>
          )}

          {(error || historyError) && (
            <div className="flex items-center gap-2 mt-3 text-sm text-red-600 bg-red-50 p-2 rounded">
              <AlertCircle className="h-4 w-4" />
              {error || historyError}
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
