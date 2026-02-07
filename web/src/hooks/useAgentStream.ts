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

interface ErrorMessage {
  type: "error";
  message: string;
}

type ServerMessage =
  | HistoryEventMessage
  | LiveEventMessage
  | HistoryEndMessage
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
  };
}

// Helper to parse ACP messages and concatenate text chunks
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
