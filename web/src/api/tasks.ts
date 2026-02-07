import client from "./client";
import type { components } from "./schema";

type TaskCreate = components["schemas"]["TaskCreate"];
type TaskUpdate = components["schemas"]["TaskUpdate"];
type TaskStatusUpdate = components["schemas"]["TaskStatusUpdate"];
type TaskCommentCreate = components["schemas"]["TaskCommentCreate"];

const getTasks = client.path("/api/tasks").method("get").create();
const createTaskApi = client.path("/api/tasks").method("post").create();
const getTask = client.path("/api/tasks/{task_id}").method("get").create();
const updateTaskApi = client.path("/api/tasks/{task_id}").method("put").create();
const updateTaskStatusApi = client.path("/api/tasks/{task_id}/status").method("post").create();
const archiveTaskApi = client.path("/api/tasks/{task_id}/archive").method("post").create();
const unarchiveTaskApi = client.path("/api/tasks/{task_id}/unarchive").method("post").create();
const deleteTaskApi = client.path("/api/tasks/{task_id}").method("delete").create();
const addCommentApi = client.path("/api/tasks/{task_id}/comments").method("post").create();

export async function fetchTasks() {
  const response = await getTasks({});
  return response.data;
}

export async function fetchTask(taskId: string) {
  const response = await getTask({ task_id: taskId });
  return response.data;
}

export async function createTask(task: TaskCreate) {
  const response = await createTaskApi(task);
  return response.data;
}

export async function updateTask(taskId: string, task: TaskUpdate) {
  const response = await updateTaskApi({ task_id: taskId, ...task });
  return response.data;
}

export async function updateTaskStatus(taskId: string, status: string) {
  const response = await updateTaskStatusApi({ task_id: taskId, status });
  return response.data;
}

export async function archiveTask(taskId: string) {
  const response = await archiveTaskApi({ task_id: taskId });
  return response.data;
}

export async function unarchiveTask(taskId: string) {
  const response = await unarchiveTaskApi({ task_id: taskId });
  return response.data;
}

export async function deleteTask(taskId: string) {
  await deleteTaskApi({ task_id: taskId });
}

export async function addComment(taskId: string, content: string) {
  const response = await addCommentApi({ task_id: taskId, content });
  return response.data;
}
