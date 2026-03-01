import { useMemo, useRef, useEffect } from "react";
import { Loader2 } from "lucide-react";
import { cn } from "@/lib/utils";
import { AssistantMessage } from "./AssistantMessage";
import { ToolCallMessage } from "./ToolCallMessage";
import { PlanMessage } from "./PlanMessage";
import { SystemMessage } from "./SystemMessage";
import { PermissionMessage } from "./PermissionMessage";
import {
  type OutputBatchData,
  type AgentMessage,
  type PlanMessage as PlanMessageType,
  type ToolCallMessage as ToolCallData,
  type SystemMessageData,
  parseOutputBatchMessages,
} from "./types";

interface Event {
  cursor: number;
  kind: string;
  time: string;
  agent_id: string;
  session_id: string | null;
  task_id: string | null;
  data: Record<string, unknown>;
}

interface ConversationViewProps {
  events: Event[];
  isConnected?: boolean;
  isLoading?: boolean;
  autoScroll?: boolean;
  className?: string;
}

interface PermissionRequestData {
  request_id: string;
  session_id: string;
  tool_call?: {
    title?: string;
    kind?: string;
  };
}

interface PermissionResponseData {
  request_id: string;
  session_id: string;
  outcome: {
    selected?: { option_id: string };
    cancelled?: boolean;
  };
}

interface PermissionState {
  request_id: string;
  session_id: string;
  tool_call?: { title?: string; kind?: string };
  outcome?: { selected?: { option_id: string }; cancelled?: boolean };
}

interface ParsedMessage {
  id: string;
  cursor: number;
  timestamp: number;
  stream: OutputBatchData["stream"] | "permission";
  sessionId: string;
  agentId: string;
  taskId?: string | null;
  // Content
  text?: string;
  planEntries?: { content: string; priority: string; status: string }[];
  toolCalls?: ToolCallData[];
  systemData?: SystemMessageData[];
  permissionData?: PermissionState;
}

export function ConversationView({
  events,
  isConnected,
  isLoading,
  autoScroll = true,
  className,
}: ConversationViewProps) {
  const scrollRef = useRef<HTMLDivElement>(null);

  // Parse output_batch events into chat messages
  const messages = useMemo(() => {
    const result: ParsedMessage[] = [];

    // First pass: build maps for tool states and permission states
    const toolStates = new Map<string, ToolCallData>();
    const permissionStates = new Map<string, PermissionState>();

    for (const event of events) {
      // Build permission states (permission.responded updates permission.requested)
      if (event.kind === "permission.requested") {
        const data = event.data as unknown as PermissionRequestData;
        const key = `${data.request_id}-${data.session_id}`;
        if (!permissionStates.has(key)) {
          permissionStates.set(key, {
            request_id: data.request_id,
            session_id: data.session_id,
            tool_call: data.tool_call,
          });
        }
        continue;
      }

      if (event.kind === "permission.responded") {
        const data = event.data as unknown as PermissionResponseData;
        const key = `${data.request_id}-${data.session_id}`;
        const existing = permissionStates.get(key);
        if (existing) {
          // Update with outcome, keep everything else from permission.requested
          permissionStates.set(key, {
            ...existing,
            outcome: data.outcome,
          });
        }
        continue;
      }

      // Build tool states (tool_result updates tool_use)
      if (event.kind !== "agent.output_batch") continue;
      const data = event.data as unknown as OutputBatchData;
      if (!data.messages) continue;

      if (data.stream === "tool_use" || data.stream === "tool_result") {
        const parsed = parseOutputBatchMessages(data.messages);
        for (const msg of parsed) {
          const tool = msg as ToolCallData;
          if (tool.type === "tool_call" || tool.type === "tool_call_update") {
            const existing = toolStates.get(tool.id);
            if (!existing) {
              // First time seeing this tool
              toolStates.set(tool.id, tool);
            } else {
              // Merge: keep existing values, update with new non-empty values
              // Priority: completed/error status > pending, non-empty title > empty/generic
              const hasRealTitle = (t: string | undefined) => t && t !== "Terminal" && t.length > 0;
              const hasRealInput = (input: Record<string, unknown> | undefined) =>
                input && Object.keys(input).length > 0;

              toolStates.set(tool.id, {
                ...existing,
                // Update status if it's a terminal state (completed/error)
                status: tool.status === "completed" || tool.status === "error"
                  ? tool.status
                  : existing.status,
                // Prefer non-generic title
                title: hasRealTitle(tool.title) ? tool.title : existing.title,
                // Prefer non-empty kind
                kind: tool.kind || existing.kind,
                // Prefer non-empty raw_input
                raw_input: hasRealInput(tool.raw_input) ? tool.raw_input : existing.raw_input,
                // Update content if provided
                content: tool.content || existing.content,
              });
            }
          }
        }
      }
    }

    // Track which tools have been rendered
    const renderedTools = new Set<string>();

    for (const event of events) {
      // Handle permission.requested events (use merged state with response)
      if (event.kind === "permission.requested") {
        const data = event.data as unknown as PermissionRequestData;
        const key = `${data.request_id}-${data.session_id}`;
        const mergedState = permissionStates.get(key);
        result.push({
          id: `${event.cursor}-permission`,
          cursor: event.cursor,
          timestamp: new Date(event.time).getTime() * 1000000,
          stream: "permission",
          sessionId: data.session_id || event.session_id || "",
          agentId: event.agent_id,
          taskId: event.task_id,
          permissionData: mergedState || data,
        });
        continue;
      }

      // Skip permission.responded - merged into permission.requested above
      if (event.kind === "permission.responded") continue;

      if (event.kind !== "agent.output_batch") continue;

      const data = event.data as unknown as OutputBatchData;
      if (!data.messages || !data.stream) continue;

      const parsed = parseOutputBatchMessages(data.messages);

      switch (data.stream) {
        case "assistant": {
          const text = parsed
            .filter((m): m is AgentMessage => (m as AgentMessage).type === "agent_message")
            .map((m) => m.text)
            .join("");
          if (text.trim()) {
            result.push({
              id: `${event.cursor}-assistant`,
              cursor: event.cursor,
              timestamp: data.ts,
              stream: "assistant",
              sessionId: data.session_id,
              agentId: event.agent_id,
              taskId: event.task_id,
              text,
            });
          }
          break;
        }

        case "plan": {
          const planMsg = parsed.find(
            (m): m is PlanMessageType => (m as PlanMessageType).type === "plan"
          );
          if (planMsg?.plan?.entries) {
            result.push({
              id: `${event.cursor}-plan`,
              cursor: event.cursor,
              timestamp: data.ts,
              stream: "plan",
              sessionId: data.session_id,
              agentId: event.agent_id,
              taskId: event.task_id,
              planEntries: planMsg.plan.entries,
            });
          }
          break;
        }

        case "tool_use": {
          // Get tools from this event, use latest state from toolStates
          const eventTools: ToolCallData[] = [];
          for (const msg of parsed) {
            const tool = msg as ToolCallData;
            if ((tool.type === "tool_call" || tool.type === "tool_call_update") && !renderedTools.has(tool.id)) {
              // Use the latest state
              const latestState = toolStates.get(tool.id) || tool;
              eventTools.push(latestState);
              renderedTools.add(tool.id);
            }
          }

          if (eventTools.length > 0) {
            result.push({
              id: `${event.cursor}-tools`,
              cursor: event.cursor,
              timestamp: data.ts,
              stream: "tool_use",
              sessionId: data.session_id,
              agentId: event.agent_id,
              taskId: event.task_id,
              toolCalls: eventTools,
            });
          }
          break;
        }

        case "tool_result": {
          // Skip - merged into tool_use above
          break;
        }

        case "system": {
          result.push({
            id: `${event.cursor}-system`,
            cursor: event.cursor,
            timestamp: data.ts,
            stream: "system",
            sessionId: data.session_id,
            agentId: event.agent_id,
            taskId: event.task_id,
            systemData: parsed as SystemMessageData[],
          });
          break;
        }
      }
    }

    // Sort by cursor to maintain order
    const sorted = result.sort((a, b) => a.cursor - b.cursor);

    return sorted;
  }, [events]);

  // Auto-scroll to bottom when new messages arrive
  useEffect(() => {
    if (autoScroll && scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [messages, autoScroll]);

  if (messages.length === 0 && !isLoading) {
    return (
      <div className={cn("flex items-center justify-center h-full text-slate-400", className)}>
        <div className="text-center">
          <p className="text-sm">No activity yet</p>
          <p className="text-xs mt-1">Messages will appear here when the agent runs</p>
        </div>
      </div>
    );
  }

  return (
    <div ref={scrollRef} className={cn("overflow-y-auto", className)}>
      <div className="divide-y divide-slate-100 px-4">
        {messages.map((msg) => {
          switch (msg.stream) {
            case "assistant":
              return (
                <AssistantMessage
                  key={msg.id}
                  text={msg.text || ""}
                  timestamp={msg.timestamp}
                />
              );

            case "plan":
              return (
                <PlanMessage
                  key={msg.id}
                  entries={msg.planEntries || []}
                  timestamp={msg.timestamp}
                />
              );

            case "tool_use":
              // tool_result is merged into tool_use
              return (
                <ToolCallMessage
                  key={msg.id}
                  tools={msg.toolCalls || []}
                  timestamp={msg.timestamp}
                />
              );

            case "system":
              return (
                <SystemMessage
                  key={msg.id}
                  data={msg.systemData || []}
                  timestamp={msg.timestamp}
                />
              );

            case "permission":
              return (
                <PermissionMessage
                  key={msg.id}
                  requestId={msg.permissionData?.request_id || ""}
                  sessionId={msg.sessionId}
                  agentId={msg.agentId}
                  taskId={msg.taskId}
                  toolCall={msg.permissionData?.tool_call}
                  outcome={msg.permissionData?.outcome}
                  timestamp={msg.timestamp}
                />
              );

            default:
              return null;
          }
        })}
      </div>

      {/* Loading indicator */}
      {isLoading && (
        <div className="flex items-center gap-2 px-4 py-3 text-slate-500">
          <Loader2 className="h-4 w-4 animate-spin" />
          <span className="text-xs">Loading...</span>
        </div>
      )}

      {/* Connection status */}
      {isConnected === false && (
        <div className="px-4 py-2 text-xs text-amber-600 bg-amber-50 border-t border-amber-100">
          Disconnected from event stream
        </div>
      )}
    </div>
  );
}
