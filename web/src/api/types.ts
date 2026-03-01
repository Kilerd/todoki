// Types extracted from OpenAPI schema
import type {operations} from "./schema";

// Extract TaskResponse from get_tasks operation response
export type TaskResponse =
    operations["get_tasks"]["responses"]["200"]["content"]["application/json"][number];

// Extract Project from list_projects operation response
export type Project =
    operations["list_projects"]["responses"]["200"]["content"]["application/json"][number];

// Extract TaskComment from TaskResponse.comments
export type TaskComment = TaskResponse["comments"][number];

// Extract TaskEvent from TaskResponse.events
export type TaskEvent = TaskResponse["events"][number];

// Extract Artifact from TaskResponse.artifacts
export type Artifact = TaskResponse["artifacts"][number];

// Extract TaskCreateRequest from create_task operation request body
export type TaskCreateRequest =
    operations["create_task"]["requestBody"]["content"]["application/json"];

// Extract TaskUpdateRequest from update_task operation request body
export type TaskUpdateRequest =
    operations["update_task"]["requestBody"]["content"]["application/json"];

// Task status type (lowercase values used in UI)
export type TaskStatus = operations["create_task"]["requestBody"]["content"]["application/json"]["status"];

// Relay info type for API responses
export interface RelayInfo {
    relay_id: string;
    name: string;
    role: string;
    safe_paths: string[];
    labels: Record<string, string>;
    projects: string[];
    setup_script: string | null;
    connected_at: number;
    active_session_count: number;
}