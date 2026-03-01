/**
 * WebSocket hook for real-time event streaming
 *
 * Connects to the server's event bus WebSocket and provides:
 * - Real-time event notifications
 * - Historical event replay from cursor
 * - Automatic reconnection with exponential backoff
 * - Event filtering by kind patterns
 */

import { useEffect, useRef, useState, useCallback, useMemo } from 'react';
import { getToken } from '@/lib/auth';

export interface Event {
  cursor: number;
  kind: string;
  time: string;
  agent_id: string;
  session_id: string | null;
  task_id: string | null;
  data: Record<string, any>;
}

interface WsMessage {
  type: 'subscribed' | 'event' | 'replay_complete' | 'error' | 'ping' | 'pong';
  cursor?: number;
  count?: number;
  kinds?: string[];
  message?: string;
  // Event fields
  kind?: string;
  time?: string;
  agent_id?: string;
  session_id?: string | null;
  task_id?: string | null;
  data?: Record<string, any>;
}

interface UseEventStreamOptions {
  /** Event kind patterns to subscribe (e.g., ["task.*", "agent.requirement_analyzed"]) */
  kinds?: string[];

  /** Starting cursor for historical replay */
  cursor?: number;

  /** Filter by agent ID */
  agentId?: string;

  /** Filter by task ID */
  taskId?: string;

  /** Enable auto-reconnect (default: true) */
  autoReconnect?: boolean;

  /** Max reconnection attempts (default: 10) */
  maxReconnectAttempts?: number;

  /** Authentication token */
  token?: string;

  /** Enable/disable the connection (default: true) */
  enabled?: boolean;
}

interface UseEventStreamReturn {
  /** Array of received events */
  events: Event[];

  /** WebSocket connection state */
  isConnected: boolean;

  /** Currently replaying historical events */
  isReplaying: boolean;

  /** Last error message */
  error: string | null;

  /** Manually reconnect */
  reconnect: () => void;

  /** Clear events */
  clearEvents: () => void;
}

export function useEventStream(options: UseEventStreamOptions = {}): UseEventStreamReturn {
  const {
    kinds,
    cursor,
    agentId,
    taskId,
    autoReconnect = true,
    maxReconnectAttempts = 10,
    token = getToken() || '',
    enabled = true,
  } = options;

  const [events, setEvents] = useState<Event[]>([]);
  const [isConnected, setIsConnected] = useState(false);
  const [isReplaying, setIsReplaying] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const reconnectAttemptsRef = useRef(0);

  // Stabilize kinds array reference by serializing to string
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const kindsKey = useMemo(() => kinds?.join(',') || '', [JSON.stringify(kinds)]);

  const buildWebSocketUrl = useCallback(() => {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    // Use VITE_API_URL from env, extract host from it, or fallback to current host with backend port
    const apiUrl = import.meta.env.VITE_API_URL;
    let host: string;
    if (apiUrl) {
      // Extract host from VITE_API_URL (e.g., "http://localhost:8201" -> "localhost:8201")
      const url = new URL(apiUrl);
      host = url.host;
    } else {
      // Fallback: assume backend is on same host with port 8201
      host = window.location.hostname + ':8201';
    }
    const params = new URLSearchParams();

    // Use kindsKey (serialized string) to avoid array reference issues
    if (kindsKey) {
      params.set('kinds', kindsKey);
    }
    if (cursor !== undefined) {
      params.set('cursor', cursor.toString());
    }
    if (agentId) {
      params.set('agent_id', agentId);
    }
    if (taskId) {
      params.set('task_id', taskId);
    }
    if (token) {
      params.set('token', token);
    }

    return `${protocol}//${host}/ws/event-bus?${params.toString()}`;
  }, [kindsKey, cursor, agentId, taskId, token]);

  const connect = useCallback(() => {
    if (!enabled) {
      return;
    }

    if (wsRef.current?.readyState === WebSocket.OPEN) {
      return;
    }

    const wsUrl = buildWebSocketUrl();
    console.log('[EventStream] Connecting to:', wsUrl);

    try {
      const ws = new WebSocket(wsUrl);

      ws.onopen = () => {
        console.log('[EventStream] Connected');
        setIsConnected(true);
        setError(null);
        reconnectAttemptsRef.current = 0;
      };

      ws.onmessage = (event) => {
        try {
          const msg: WsMessage = JSON.parse(event.data);

          switch (msg.type) {
            case 'subscribed':
              console.log('[EventStream] Subscribed:', msg.kinds);
              break;

            case 'event':
              if (msg.kind && msg.cursor !== undefined) {
                const newEvent: Event = {
                  cursor: msg.cursor,
                  kind: msg.kind,
                  time: msg.time || new Date().toISOString(),
                  agent_id: msg.agent_id || '',
                  session_id: msg.session_id || null,
                  task_id: msg.task_id || null,
                  data: msg.data || {},
                };
                setEvents((prev) => [...prev, newEvent]);
              }
              break;

            case 'replay_complete':
              console.log('[EventStream] Replay complete:', msg.count, 'events');
              setIsReplaying(false);
              break;

            case 'error':
              console.error('[EventStream] Server error:', msg.message);
              setError(msg.message || 'Unknown error');
              break;

            case 'ping':
              // Server ping, WebSocket auto-responds with pong
              break;

            default:
              console.warn('[EventStream] Unknown message type:', msg.type);
          }
        } catch (err) {
          console.error('[EventStream] Failed to parse message:', err);
        }
      };

      ws.onerror = (err) => {
        console.error('[EventStream] WebSocket error:', err);
        setError('WebSocket connection error');
      };

      ws.onclose = () => {
        console.log('[EventStream] Disconnected');
        setIsConnected(false);

        // Auto-reconnect with exponential backoff
        if (autoReconnect && reconnectAttemptsRef.current < maxReconnectAttempts) {
          const delay = Math.min(1000 * Math.pow(2, reconnectAttemptsRef.current), 30000);
          console.log(`[EventStream] Reconnecting in ${delay}ms...`);

          reconnectTimeoutRef.current = setTimeout(() => {
            reconnectAttemptsRef.current++;
            connect();
          }, delay);
        }
      };

      wsRef.current = ws;
    } catch (err) {
      console.error('[EventStream] Failed to create WebSocket:', err);
      setError('Failed to connect to event stream');
    }
  }, [buildWebSocketUrl, autoReconnect, maxReconnectAttempts, enabled]);

  const disconnect = useCallback(() => {
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current);
      reconnectTimeoutRef.current = null;
    }

    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }

    setIsConnected(false);
  }, []);

  const reconnect = useCallback(() => {
    reconnectAttemptsRef.current = 0;
    disconnect();
    connect();
  }, [connect, disconnect]);

  const clearEvents = useCallback(() => {
    setEvents([]);
  }, []);

  // Connect on mount, disconnect when disabled
  useEffect(() => {
    if (!enabled) {
      disconnect();
      return;
    }

    if (cursor !== undefined) {
      setIsReplaying(true);
    }
    connect();

    return () => {
      disconnect();
    };
  }, [connect, disconnect, cursor, enabled]);

  return {
    events,
    isConnected,
    isReplaying,
    error,
    reconnect,
    clearEvents,
  };
}
