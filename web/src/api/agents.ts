import { getToken } from "@/lib/auth";

const API_URL = import.meta.env.VITE_API_URL;

export interface Agent {
  id: string;
  name: string;
  workdir: string;
  command: string;
  args: string[];
  execution_mode: "local" | "remote";
  relay_id: string | null;
  status: "created" | "running" | "stopped" | "exited" | "failed";
  created_at: string;
  updated_at: string;
}

export interface AgentSession {
  id: string;
  agent_id: string;
  relay_id: string | null;
  status: "running" | "completed" | "failed" | "cancelled";
  started_at: string;
  ended_at: string | null;
}

export interface CreateAgentRequest {
  name: string;
  workdir: string;
  command: string;
  args?: string[];
  execution_mode?: "local" | "remote";
  relay_id?: string;
  auto_start?: boolean;
}

export interface SendInputRequest {
  input: string;
}

async function fetchWithAuth(url: string, options: RequestInit = {}) {
  const token = getToken();
  const headers = new Headers(options.headers);
  if (token) {
    headers.set("Authorization", `Bearer ${token}`);
  }
  headers.set("Content-Type", "application/json");

  const response = await fetch(url, { ...options, headers });

  if (!response.ok) {
    const error = await response.text();
    throw new Error(error || response.statusText);
  }

  return response.json();
}

export async function listAgents(): Promise<Agent[]> {
  return fetchWithAuth(`${API_URL}/api/agents`);
}

export async function getAgent(agentId: string): Promise<Agent> {
  return fetchWithAuth(`${API_URL}/api/agents/${agentId}`);
}

export async function createAgent(req: CreateAgentRequest): Promise<{ agent: Agent; session?: AgentSession }> {
  return fetchWithAuth(`${API_URL}/api/agents`, {
    method: "POST",
    body: JSON.stringify(req),
  });
}

export async function deleteAgent(agentId: string): Promise<void> {
  return fetchWithAuth(`${API_URL}/api/agents/${agentId}`, {
    method: "DELETE",
  });
}

export async function startAgent(agentId: string): Promise<AgentSession> {
  return fetchWithAuth(`${API_URL}/api/agents/${agentId}/start`, {
    method: "POST",
  });
}

export async function stopAgent(agentId: string): Promise<void> {
  return fetchWithAuth(`${API_URL}/api/agents/${agentId}/stop`, {
    method: "POST",
  });
}

export async function getAgentSessions(agentId: string): Promise<AgentSession[]> {
  return fetchWithAuth(`${API_URL}/api/agents/${agentId}/sessions`);
}

export async function sendInput(agentId: string, input: string): Promise<void> {
  return fetchWithAuth(`${API_URL}/api/agents/${agentId}/input`, {
    method: "POST",
    body: JSON.stringify({ input }),
  });
}

export async function respondPermission(
  agentId: string,
  requestId: string,
  outcome: { type: "selected"; option_id: string } | { type: "cancelled" }
): Promise<void> {
  return fetchWithAuth(`${API_URL}/api/agents/${agentId}/permission`, {
    method: "POST",
    body: JSON.stringify({ request_id: requestId, ...outcome }),
  });
}

