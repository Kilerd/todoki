import { getToken } from "@/lib/auth";

const API_BASE_URL = import.meta.env.VITE_API_URL || "http://localhost:8201";

export interface EmitEventRequest {
  kind: string;
  data: Record<string, any>;
  task_id?: string;
  session_id?: string;
}

export interface EmitEventResponse {
  cursor: number;
}

export interface EventBusEvent {
  cursor: number;
  kind: string;
  time: string;
  agent_id: string;
  session_id: string | null;
  task_id: string | null;
  data: Record<string, any>;
}

export interface QueryEventsParams {
  cursor: number;
  kinds?: string;
  agent_id?: string;
  task_id?: string;
  limit?: number;
}

export interface QueryEventsResponse {
  events: EventBusEvent[];
  next_cursor: number;
}

/**
 * Query historical events from the event bus
 */
export async function queryEvents(
  params: QueryEventsParams
): Promise<QueryEventsResponse> {
  const token = getToken();
  if (!token) {
    throw new Error("Authentication token not found");
  }

  const searchParams = new URLSearchParams();
  searchParams.set("cursor", params.cursor.toString());
  if (params.kinds) searchParams.set("kinds", params.kinds);
  if (params.agent_id) searchParams.set("agent_id", params.agent_id);
  if (params.task_id) searchParams.set("task_id", params.task_id);
  if (params.limit) searchParams.set("limit", params.limit.toString());

  const response = await fetch(
    `${API_BASE_URL}/api/event-bus?${searchParams.toString()}`,
    {
      headers: {
        Authorization: `Bearer ${token}`,
      },
    }
  );

  if (!response.ok) {
    const error = await response.text();
    throw new Error(`Failed to query events: ${error}`);
  }

  return response.json();
}

/**
 * Get the latest cursor from the event bus
 */
export async function getLatestCursor(): Promise<number> {
  const token = getToken();
  if (!token) {
    throw new Error("Authentication token not found");
  }

  const response = await fetch(`${API_BASE_URL}/api/event-bus/latest`, {
    headers: {
      Authorization: `Bearer ${token}`,
    },
  });

  if (!response.ok) {
    const error = await response.text();
    throw new Error(`Failed to get latest cursor: ${error}`);
  }

  return response.json();
}

/**
 * Emit a new event to the event bus
 */
export async function emitEvent(
  request: EmitEventRequest
): Promise<EmitEventResponse> {
  const token = getToken();
  if (!token) {
    throw new Error("Authentication token not found");
  }

  const response = await fetch(`${API_BASE_URL}/api/event-bus/emit`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${token}`,
    },
    body: JSON.stringify(request),
  });

  if (!response.ok) {
    const error = await response.text();
    throw new Error(`Failed to emit event: ${error}`);
  }

  const cursor = await response.json();
  return { cursor };
}
