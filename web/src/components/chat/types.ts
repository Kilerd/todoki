// Types for chat messages parsed from agent.output_batch events

export interface OutputBatchData {
  messages: string[];
  session_id: string;
  stream: StreamType;
  ts: number;
}

export type StreamType = "system" | "assistant" | "plan" | "tool_use" | "tool_result";

export interface AgentMessage {
  type: "agent_message";
  chunk?: boolean;
  text: string;
}

export interface PlanEntry {
  content: string;
  priority: string;
  status: string;
}

export interface PlanMessage {
  type: "plan";
  plan: {
    entries: PlanEntry[];
  };
}

export interface ToolCallMessage {
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

export interface SystemMessageData {
  type: string;
  commands?: Array<{
    name: string;
    description: string;
    input?: { hint?: string } | null;
  }>;
  meta?: unknown;
}

// Unified chat message type for rendering
export interface ChatMessageData {
  id: string;
  timestamp: number;
  stream: StreamType;
  sessionId: string;
  agentId: string;
  taskId?: string | null;
  // Parsed content based on stream type
  text?: string;
  plan?: PlanEntry[];
  toolCalls?: ToolCallMessage[];
  systemData?: SystemMessageData[];
}

// Helper to parse raw messages from output_batch
export function parseOutputBatchMessages(messages: string[]): unknown[] {
  return messages.map((msg) => {
    try {
      return JSON.parse(msg);
    } catch {
      return { type: "raw", text: msg };
    }
  });
}
