import { fetcher } from "./fetcher";

// Project API endpoints using openapi-typescript-fetch
export const fetchProjects = fetcher.path("/api/projects").method("get").create();
export const createProject = fetcher.path("/api/projects").method("post").create();
export const fetchProject = fetcher.path("/api/projects/{project_id}").method("get").create();
export const fetchProjectByName = fetcher.path("/api/projects/by-name/{name}").method("get").create();
export const updateProject = fetcher.path("/api/projects/{project_id}").method("put").create();
export const deleteProject = fetcher.path("/api/projects/{project_id}").method("delete").create();
export const fetchProjectDoneTasks = fetcher.path("/api/projects/{project_id}/tasks/done").method("get").create();
