import { fetcher } from "./fetcher";

// Task API endpoints using openapi-typescript-fetch
export const fetchTasks = fetcher.path("/api/tasks").method("get").create();
export const fetchInboxTasks = fetcher.path("/api/tasks/inbox").method("get").create();
export const fetchTodayDoneTasks = fetcher.path("/api/tasks/done/today").method("get").create();
export const fetchBacklogTasks = fetcher.path("/api/tasks/backlog").method("get").create();
export const fetchTask = fetcher.path("/api/tasks/{task_id}").method("get").create();
export const createTask = fetcher.path("/api/tasks").method("post").create();
export const updateTask = fetcher.path("/api/tasks/{task_id}").method("put").create();
export const deleteTask = fetcher.path("/api/tasks/{task_id}").method("delete").create();
export const updateTaskStatus = fetcher.path("/api/tasks/{task_id}/status").method("post").create();
export const archiveTask = fetcher.path("/api/tasks/{task_id}/archive").method("post").create();
export const unarchiveTask = fetcher.path("/api/tasks/{task_id}/unarchive").method("post").create();
export const addComment = fetcher.path("/api/tasks/{task_id}/comments").method("post").create();
export const fetchReport = fetcher.path("/api/report").method("get").create();
