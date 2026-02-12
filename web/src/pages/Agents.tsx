import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import {
  Play,
  Square,
  Trash2,
  RefreshCw,
  Circle,
  Send,
  Bot,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import NavBar from "../components/NavBar";
import {
  listAgents,
  startAgent,
  stopAgent,
  deleteAgent,
  respondPermission,
} from "../api/agents";
import type { operations } from "../api/schema";

type Agent = operations["list_agents"]["responses"]["200"]["content"]["application/json"][number];
import {
  useAgentStream,
  parseAcpEvents,
} from "../hooks/useAgentStream";

interface PermissionOption {
  optionId: string;  // ACP uses camelCase
  name: string;
  kind: string;
}

interface PermissionRequest {
  request_id: string;
  tool_call_id: string;
  options: PermissionOption[];
  tool_call: {
    tool_call_id: string;
    title?: string;
    kind?: string;
    raw_input?: unknown;
  };
}

function PermissionRequestCard({
  request,
  agentId,
  onResponded,
}: {
  request: PermissionRequest;
  agentId: string;
  onResponded: () => void;
}) {
  const [isResponding, setIsResponding] = useState(false);

  const handleSelect = async (optionId: string) => {
    setIsResponding(true);
    try {
      await respondPermission({
        agent_id: agentId,
        request_id: request.request_id,
        outcome: {
          type: "selected",
          option_id: optionId,
        },
      });
      onResponded();
    } catch (e) {
      console.error("Failed to respond to permission:", e);
    } finally {
      setIsResponding(false);
    }
  };

  const handleCancel = async () => {
    setIsResponding(true);
    try {
      await respondPermission({
        agent_id: agentId,
        request_id: request.request_id,
        outcome: {
          type: "cancelled",
        },
      });
      onResponded();
    } catch (e) {
      console.error("Failed to cancel permission:", e);
    } finally {
      setIsResponding(false);
    }
  };

  return (
    <div className="bg-amber-50 border border-amber-200 rounded-lg p-4 my-2">
      <div className="flex items-center gap-2 mb-2">
        <span className="text-amber-600 font-medium">Permission Required</span>
      </div>
      <div className="text-sm text-slate-700 mb-3">
        <div className="font-medium">
          {String(request.tool_call.title ?? "Tool Call")}
        </div>
        {request.tool_call.raw_input != null && (
          <pre className="mt-1 text-xs bg-slate-100 p-2 rounded overflow-auto max-h-32">
            {JSON.stringify(request.tool_call.raw_input, null, 2)}
          </pre>
        )}
      </div>
      <div className="flex flex-wrap gap-2">
        {request.options.map((option) => (
          <Button
            key={option.optionId}
            size="sm"
            variant={option.kind === "reject_once" ? "outline" : "default"}
            onClick={() => handleSelect(option.optionId)}
            disabled={isResponding}
            className={cn(
              option.kind === "reject_once" && "text-red-600 border-red-200 hover:bg-red-50"
            )}
          >
            {option.name}
          </Button>
        ))}
        <Button
          size="sm"
          variant="ghost"
          onClick={handleCancel}
          disabled={isResponding}
        >
          Cancel
        </Button>
      </div>
    </div>
  );
}

function StatusBadge({ status }: { status: Agent["status"] }) {
  const variants: Record<Agent["status"], { color: string; label: string }> = {
    created: { color: "bg-slate-100 text-slate-600", label: "Created" },
    running: { color: "bg-green-100 text-green-700", label: "Running" },
    stopped: { color: "bg-yellow-100 text-yellow-700", label: "Stopped" },
    exited: { color: "bg-slate-100 text-slate-600", label: "Exited" },
    failed: { color: "bg-red-100 text-red-700", label: "Failed" },
  };

  const variant = variants[status] || variants.created;

  return (
    <span
      className={cn(
        "inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs font-medium",
        variant.color
      )}
    >
      <Circle
        className={cn(
          "h-2 w-2",
          status === "running" ? "fill-green-500" : "fill-current"
        )}
      />
      {variant.label}
    </span>
  );
}

function AgentListItem({
  agent,
  isSelected,
  onClick,
  onStart,
  onStop,
  onDelete,
}: {
  agent: Agent;
  isSelected: boolean;
  onClick: () => void;
  onStart: () => void;
  onStop: () => void;
  onDelete: () => void;
}) {
  return (
    <div
      className={cn(
        "p-3 border rounded-lg cursor-pointer transition-colors",
        isSelected
          ? "border-teal-500 bg-teal-50"
          : "border-slate-200 hover:border-slate-300"
      )}
      onClick={onClick}
    >
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Bot className="h-4 w-4 text-slate-400" />
          <span className="font-medium text-slate-700">{agent.name}</span>
        </div>
        <StatusBadge status={agent.status} />
      </div>

      <div className="mt-2 text-xs text-slate-500 truncate">
        {agent.command} {agent.args.join(" ")}
      </div>

      <div className="mt-3 flex gap-2">
        {agent.status === "running" ? (
          <Button
            size="sm"
            variant="outline"
            onClick={(e) => {
              e.stopPropagation();
              onStop();
            }}
          >
            <Square className="h-3 w-3 mr-1" />
            Stop
          </Button>
        ) : (
          <Button
            size="sm"
            variant="outline"
            onClick={(e) => {
              e.stopPropagation();
              onStart();
            }}
          >
            <Play className="h-3 w-3 mr-1" />
            Start
          </Button>
        )}
        <Button
          size="sm"
          variant="outline"
          className="text-red-600 hover:text-red-700"
          onClick={(e) => {
            e.stopPropagation();
            onDelete();
          }}
        >
          <Trash2 className="h-3 w-3" />
        </Button>
      </div>
    </div>
  );
}

function AgentOutput({
  agentId,
  isRunning,
}: {
  agentId: string;
  isRunning: boolean;
}) {
  const { events, isConnected, isLoadingHistory, error, reconnect, sendInput } =
    useAgentStream({
      agentId,
      enabled: true,
    });

  const outputRef = useRef<HTMLDivElement>(null);
  const [inputValue, setInputValue] = useState("");
  const [isSending, setIsSending] = useState(false);
  const [respondedRequests, setRespondedRequests] = useState<Set<string>>(new Set());

  // Auto-scroll to bottom
  useEffect(() => {
    if (outputRef.current) {
      outputRef.current.scrollTop = outputRef.current.scrollHeight;
    }
  }, [events]);

  // Group events by stream and render
  const renderedOutput = useMemo(() => {
    const acpText = parseAcpEvents(events);
    const nonAcpEvents = events.filter(
      (e) => e.stream !== "acp" && e.stream !== "permission_request"
    );

    // Parse permission request events
    const permissionRequests: PermissionRequest[] = [];
    for (const event of events) {
      if (event.stream === "permission_request") {
        try {
          const parsed = JSON.parse(event.message) as PermissionRequest;
          console.log("Permission request parsed:", parsed);
          console.log("Options:", parsed.options);
          // Only show if not already responded
          if (!respondedRequests.has(parsed.request_id)) {
            permissionRequests.push(parsed);
          }
        } catch {
          console.error("Failed to parse permission request:", event.message);
        }
      }
    }

    return { acpText, nonAcpEvents, permissionRequests };
  }, [events, respondedRequests]);

  const handlePermissionResponded = (requestId: string) => {
    setRespondedRequests((prev) => new Set([...prev, requestId]));
  };

  const handleSendInput = async () => {
    if (!inputValue.trim() || isSending || !isConnected) return;

    setIsSending(true);
    try {
      await sendInput(inputValue + "\n");
      setInputValue("");
    } catch (e) {
      console.error("Failed to send input:", e);
    } finally {
      setIsSending(false);
    }
  };

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center justify-between p-3 border-b">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium">Output</span>
          {isLoadingHistory && (
            <span className="text-xs text-slate-400">Loading history...</span>
          )}
        </div>
        <div className="flex items-center gap-2">
          <div
            className={cn(
              "h-2 w-2 rounded-full",
              isConnected ? "bg-green-500" : "bg-red-500"
            )}
            title={isConnected ? "Connected" : "Disconnected"}
          />
          <Button size="sm" variant="ghost" onClick={reconnect}>
            <RefreshCw className="h-3 w-3" />
          </Button>
        </div>
      </div>

      {/* Error */}
      {error && (
        <div className="p-2 bg-red-50 text-red-600 text-sm">{error}</div>
      )}

      {/* Output area */}
      <div
        ref={outputRef}
        className="flex-1 overflow-auto p-4 bg-slate-900 text-slate-100 font-mono text-sm"
      >
        {/* Non-ACP events (stdout/stderr) */}
        {renderedOutput.nonAcpEvents.map((event) => (
          <div
            key={event.id}
            className={cn(
              "whitespace-pre-wrap",
              event.stream === "stderr" && "text-red-400",
              event.stream === "system" && "text-yellow-400"
            )}
          >
            {event.message}
          </div>
        ))}

        {/* ACP concatenated text */}
        {renderedOutput.acpText && (
          <div className="whitespace-pre-wrap text-teal-300 mt-2">
            {renderedOutput.acpText}
          </div>
        )}

        {/* Permission requests */}
        {renderedOutput.permissionRequests.map((request) => (
          <PermissionRequestCard
            key={request.request_id}
            request={request}
            agentId={agentId}
            onResponded={() => handlePermissionResponded(request.request_id)}
          />
        ))}

        {events.length === 0 && !isLoadingHistory && (
          <div className="text-slate-500 italic">No output yet</div>
        )}
      </div>

      {/* Input area */}
      {isRunning && (
        <div className="p-3 border-t flex gap-2">
          <Input
            value={inputValue}
            onChange={(e) => setInputValue(e.target.value)}
            placeholder="Send input to agent..."
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                handleSendInput();
              }
            }}
            className="font-mono text-sm"
          />
          <Button onClick={handleSendInput} disabled={isSending}>
            <Send className="h-4 w-4" />
          </Button>
        </div>
      )}
    </div>
  );
}

function Agents() {
  const [agents, setAgents] = useState<Agent[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [selectedAgentId, setSelectedAgentId] = useState<string | null>(null);

  const loadAgents = async () => {
    try {
      const { data } = await listAgents({});
      setAgents(data);
      // Auto-select first agent if none selected
      if (!selectedAgentId && data.length > 0) {
        setSelectedAgentId(data[0].id);
      }
    } catch (e) {
      console.error("Failed to load agents:", e);
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    loadAgents();
  }, []);

  const selectedAgent = useMemo(
    () => agents.find((a) => a.id === selectedAgentId),
    [agents, selectedAgentId]
  );

  const handleStart = async (agentId: string) => {
    try {
      await startAgent({ agent_id: agentId });
      loadAgents();
    } catch (e) {
      console.error("Failed to start agent:", e);
    }
  };

  const handleStop = async (agentId: string) => {
    try {
      await stopAgent({ agent_id: agentId });
      loadAgents();
    } catch (e) {
      console.error("Failed to stop agent:", e);
    }
  };

  const handleDelete = async (agentId: string) => {
    if (!confirm("Are you sure you want to delete this agent?")) return;

    try {
      await deleteAgent({ agent_id: agentId });
      if (selectedAgentId === agentId) {
        setSelectedAgentId(null);
      }
      loadAgents();
    } catch (e) {
      console.error("Failed to delete agent:", e);
    }
  };

  return (
    <div className="container mx-auto mt-12 max-w-6xl">
      <NavBar />

      <div className="mt-8">
        <div className="flex items-center justify-between mb-6">
          <h1 className="text-xl font-semibold text-slate-800">Agents</h1>
          <Badge variant="outline">{agents.length} agents</Badge>
        </div>

        {isLoading ? (
          <div className="space-y-3">
            <Skeleton className="h-24 w-full" />
            <Skeleton className="h-24 w-full" />
          </div>
        ) : agents.length === 0 ? (
          <div className="text-center py-12 text-slate-400">
            No agents yet. Create one via the API.
          </div>
        ) : (
          <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
            {/* Agent List */}
            <div className="space-y-3">
              {agents.map((agent) => (
                <AgentListItem
                  key={agent.id}
                  agent={agent}
                  isSelected={agent.id === selectedAgentId}
                  onClick={() => setSelectedAgentId(agent.id)}
                  onStart={() => handleStart(agent.id)}
                  onStop={() => handleStop(agent.id)}
                  onDelete={() => handleDelete(agent.id)}
                />
              ))}
            </div>

            {/* Agent Output */}
            <div className="lg:col-span-2 border rounded-lg overflow-hidden h-[600px]">
              {selectedAgent ? (
                <AgentOutput
                  agentId={selectedAgent.id}
                  isRunning={selectedAgent.status === "running"}
                />
              ) : (
                <div className="flex items-center justify-center h-full text-slate-400">
                  Select an agent to view output
                </div>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

export default Agents;
