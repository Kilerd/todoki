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
