import { useEffect, useRef, useState, useCallback } from "react";
import { getToken } from "@/lib/auth";

export interface AgentEvent {
  id: number;
  seq: number;
  ts: string;
  stream: string;
  message: string;
}

export interface ParsedAcpMessage {
  chunk?: boolean;
  text?: string;
  type?: string;
  [key: string]: unknown;
}

export interface ToolCallInfo {
  id: string;
  title?: string;
  kind?: unknown;
  status?: unknown;
  content?: unknown;
  raw_input?: unknown;
  raw_output?: unknown;
}

export interface AcpContent {
  type: "thought" | "message" | "tool_call" | "tool_call_update" | "other";
  text?: string;
  toolCall?: ToolCallInfo;
}

interface HistoryEventMessage {
  type: "history_event";
  id: number;
  seq: number;
  ts: string;
  stream: string;
  message: string;
}

interface LiveEventMessage {
  type: "live_event";
  id: number;
  seq: number;
  ts: string;
  stream: string;
  message: string;
}

interface HistoryEndMessage {
  type: "history_end";
  last_id: number | null;
}

interface InputResultMessage {
  type: "input_result";
  success: boolean;
  error: string | null;
}

interface ErrorMessage {
  type: "error";
  message: string;
}

type ServerMessage =
  | HistoryEventMessage
  | LiveEventMessage
  | HistoryEndMessage
  | InputResultMessage
  | ErrorMessage;

export interface UseAgentStreamOptions {
  agentId: string;
  enabled?: boolean;
  afterId?: number;
}

export interface UseAgentStreamResult {
  events: AgentEvent[];
  isConnected: boolean;
  isLoadingHistory: boolean;
  error: string | null;
  reconnect: () => void;
  sendInput: (input: string) => Promise<void>;
}

export function useAgentStream({
  agentId,
  enabled = true,
  afterId,
}: UseAgentStreamOptions): UseAgentStreamResult {
  const [events, setEvents] = useState<AgentEvent[]>([]);
  const [isConnected, setIsConnected] = useState(false);
  const [isLoadingHistory, setIsLoadingHistory] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimeoutRef = useRef<number | null>(null);
  const isMountedRef = useRef(true);

  const connect = useCallback(() => {
    // Clear any pending reconnect timer
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current);
      reconnectTimeoutRef.current = null;
    }

    if (!enabled || !agentId || !isMountedRef.current) return;

    const token = getToken();
    if (!token) {
      setError("No authentication token");
      return;
    }

    // Build WebSocket URL
    const apiUrl = import.meta.env.VITE_API_URL;
    const wsProtocol = apiUrl.startsWith("https") ? "wss" : "ws";
    const wsHost = apiUrl.replace(/^https?:\/\//, "");
    let wsUrl = `${wsProtocol}://${wsHost}/ws/agents/${agentId}/stream?token=${encodeURIComponent(token)}`;

    if (afterId !== undefined) {
      wsUrl += `&after_id=${afterId}`;
    }

    // Close existing connection without triggering reconnect
    if (wsRef.current) {
      wsRef.current.onclose = null;
      wsRef.current.close();
      wsRef.current = null;
    }

    setIsLoadingHistory(true);
    setError(null);

    const ws = new WebSocket(wsUrl);
    wsRef.current = ws;

    ws.onopen = () => {
      if (!isMountedRef.current) return;
      setIsConnected(true);
      setError(null);
    };

    ws.onmessage = (event) => {
      if (!isMountedRef.current) return;
      try {
        const msg: ServerMessage = JSON.parse(event.data);

        switch (msg.type) {
          case "history_event":
            setEvents((prev) => [
              ...prev,
              {
                id: msg.id,
                seq: msg.seq,
                ts: msg.ts,
                stream: msg.stream,
                message: msg.message,
              },
            ]);
            break;

          case "live_event":
            setEvents((prev) => [
              ...prev,
              {
                id: msg.id,
                seq: msg.seq,
                ts: msg.ts,
                stream: msg.stream,
                message: msg.message,
              },
            ]);
            break;

          case "history_end":
            setIsLoadingHistory(false);
            break;

          case "input_result":
            if (!msg.success && msg.error) {
              setError(`Input failed: ${msg.error}`);
            }
            break;

          case "error":
            setError(msg.message);
            break;
        }
      } catch (e) {
        console.error("Failed to parse WebSocket message:", e);
      }
    };

    ws.onclose = () => {
      if (!isMountedRef.current) return;
      setIsConnected(false);
      // Auto-reconnect after 3 seconds
      reconnectTimeoutRef.current = window.setTimeout(() => {
        if (enabled && isMountedRef.current) {
          connect();
        }
      }, 3000);
    };

    ws.onerror = () => {
      if (!isMountedRef.current) return;
      setError("WebSocket connection error");
    };
  }, [agentId, enabled, afterId]);

  const reconnect = useCallback(() => {
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current);
      reconnectTimeoutRef.current = null;
    }
    setEvents([]);
    connect();
  }, [connect]);

  const sendInput = useCallback(
    async (input: string): Promise<void> => {
      if (!wsRef.current || wsRef.current.readyState !== WebSocket.OPEN) {
        throw new Error("WebSocket not connected");
      }

      const message = JSON.stringify({
        type: "send_input",
        input,
      });

      wsRef.current.send(message);
    },
    []
  );

  useEffect(() => {
    isMountedRef.current = true;
    connect();

    return () => {
      isMountedRef.current = false;
      if (reconnectTimeoutRef.current) {
        clearTimeout(reconnectTimeoutRef.current);
        reconnectTimeoutRef.current = null;
      }
      if (wsRef.current) {
        wsRef.current.onclose = null;
        wsRef.current.close();
        wsRef.current = null;
      }
    };
  }, [connect]);

  return {
    events,
    isConnected,
    isLoadingHistory,
    error,
    reconnect,
    sendInput,
  };
}

// Helper to parse ACP messages and concatenate text chunks (legacy)
export function parseAcpEvents(events: AgentEvent[]): string {
  let result = "";

  for (const event of events) {
    if (event.stream === "acp") {
      try {
        const parsed: ParsedAcpMessage = JSON.parse(event.message);
        if (parsed.chunk && parsed.text) {
          result += parsed.text;
        } else if (parsed.type === "agent_message" && parsed.text) {
          result += parsed.text;
        }
      } catch {
        // Not JSON, append raw message
        result += event.message;
      }
    }
  }

  return result;
}

// Parse ACP events into structured content list
export function parseAcpEventsStructured(events: AgentEvent[]): AcpContent[] {
  const contents: AcpContent[] = [];
  // Track tool calls by id for merging updates
  const toolCalls = new Map<string, ToolCallInfo>();

  for (const event of events) {
    if (event.stream !== "acp") continue;

    try {
      const parsed: ParsedAcpMessage = JSON.parse(event.message);
      const msgType = parsed.type;

      if (msgType === "agent_thought" && parsed.text) {
        // Merge consecutive thoughts
        const last = contents[contents.length - 1];
        if (last && last.type === "thought") {
          last.text = (last.text || "") + parsed.text;
        } else {
          contents.push({ type: "thought", text: parsed.text });
        }
      } else if (msgType === "agent_message" && parsed.text) {
        // Merge consecutive messages
        const last = contents[contents.length - 1];
        if (last && last.type === "message") {
          last.text = (last.text || "") + parsed.text;
        } else {
          contents.push({ type: "message", text: parsed.text });
        }
      } else if (msgType === "tool_call") {
        const toolCall: ToolCallInfo = {
          id: String(parsed.id || ""),
          title: parsed.title as string | undefined,
          kind: parsed.kind,
          status: parsed.status,
          content: parsed.content,
          raw_input: parsed.raw_input,
          raw_output: parsed.raw_output,
        };
        toolCalls.set(toolCall.id, toolCall);
        contents.push({ type: "tool_call", toolCall });
      } else if (msgType === "tool_call_update") {
        const id = String(parsed.id || "");
        const existing = toolCalls.get(id);
        if (existing) {
          // Update existing tool call
          if (parsed.status !== undefined) existing.status = parsed.status;
          if (parsed.title !== undefined) existing.title = parsed.title as string;
          if (parsed.content !== undefined) existing.content = parsed.content;
          if (parsed.raw_input !== undefined) existing.raw_input = parsed.raw_input;
          if (parsed.raw_output !== undefined) existing.raw_output = parsed.raw_output;
        } else {
          // Add as update event
          contents.push({
            type: "tool_call_update",
            toolCall: {
              id,
              status: parsed.status,
              title: parsed.title as string | undefined,
              raw_output: parsed.raw_output,
            },
          });
        }
      } else if (parsed.chunk && parsed.text) {
        // Generic chunk - treat as message
        const last = contents[contents.length - 1];
        if (last && last.type === "message") {
          last.text = (last.text || "") + parsed.text;
        } else {
          contents.push({ type: "message", text: parsed.text });
        }
      }
    } catch {
      // Not JSON - treat as message
      contents.push({ type: "other", text: event.message });
    }
  }

  return contents;
}
