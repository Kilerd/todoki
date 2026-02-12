import { fetcher } from "./fetcher";

// Agent API endpoints using openapi-typescript-fetch
export const listAgents = fetcher.path("/api/agents").method("get").create();
export const getAgent = fetcher.path("/api/agents/{agent_id}").method("get").create();
export const createAgent = fetcher.path("/api/agents").method("post").create();
export const deleteAgent = fetcher.path("/api/agents/{agent_id}").method("delete").create();
export const startAgent = fetcher.path("/api/agents/{agent_id}/start").method("post").create();
export const stopAgent = fetcher.path("/api/agents/{agent_id}/stop").method("post").create();
export const getAgentSessions = fetcher.path("/api/agents/{agent_id}/sessions").method("get").create();
export const getAgentEvents = fetcher.path("/api/agents/{agent_id}/events").method("get").create();
export const sendInput = fetcher.path("/api/agents/{agent_id}/input").method("post").create();
export const respondPermission = fetcher.path("/api/agents/{agent_id}/permission").method("post").create();
