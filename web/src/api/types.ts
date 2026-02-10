// Types extracted from OpenAPI schema
import type {operations} from "./schema";

// Extract TaskResponse from get_tasks operation response
export type TaskResponse =
    operations["get_tasks"]["responses"]["200"]["content"]["application/json"][number];

// Extract Project from TaskResponse.project
export type Project = TaskResponse["project"];

// Extract TaskComment from TaskResponse.comments
export type TaskComment = TaskResponse["comments"][number];

// Extract TaskEvent from TaskResponse.events
export type TaskEvent = TaskResponse["events"][number];

// Extract TaskCreateRequest from create_task operation request body
export type TaskCreateRequest =
    operations["create_task"]["requestBody"]["content"]["application/json"];

// Extract TaskUpdateRequest from update_task operation request body
export type TaskUpdateRequest =
    operations["update_task"]["requestBody"]["content"]["application/json"];

// Task status type (lowercase values used in UI)
export type TaskStatus = operations["create_task"]["requestBody"]["content"]["application/json"]["status"];