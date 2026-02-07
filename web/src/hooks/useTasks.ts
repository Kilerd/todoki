import { useCallback, useEffect, useState } from "react";
import * as api from "../api/tasks";
import type { TaskResponse } from "../api/schema";

// Simple global state for tasks to enable refresh across components
let globalTasks: TaskResponse[] = [];
let globalListeners: Set<() => void> = new Set();

function notifyListeners() {
  globalListeners.forEach((listener) => listener());
}

export function useTasks() {
  const [tasks, setTasks] = useState<TaskResponse[]>(globalTasks);
  const [isLoading, setIsLoading] = useState(globalTasks.length === 0);

  const refresh = useCallback(async () => {
    setIsLoading(true);
    try {
      const data = await api.fetchTasks();
      globalTasks = data;
      setTasks(data);
      notifyListeners();
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    const listener = () => setTasks(globalTasks);
    globalListeners.add(listener);

    if (globalTasks.length === 0) {
      refresh();
    }

    return () => {
      globalListeners.delete(listener);
    };
  }, [refresh]);

  return { tasks, isLoading, refresh };
}

export function useTask(taskId: string) {
  const [task, setTask] = useState<TaskResponse | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  const refresh = useCallback(async () => {
    setIsLoading(true);
    try {
      const data = await api.fetchTask(taskId);
      setTask(data);
    } finally {
      setIsLoading(false);
    }
  }, [taskId]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return { task, isLoading, refresh };
}

// Re-export API functions with auto-refresh
export async function createTask(task: Parameters<typeof api.createTask>[0]) {
  const result = await api.createTask(task);
  const data = await api.fetchTasks();
  globalTasks = data;
  notifyListeners();
  return result;
}

export async function updateTaskStatus(taskId: string, status: string) {
  const result = await api.updateTaskStatus(taskId, status);
  const data = await api.fetchTasks();
  globalTasks = data;
  notifyListeners();
  return result;
}

export async function updateTask(
  taskId: string,
  task: Parameters<typeof api.updateTask>[1]
) {
  const result = await api.updateTask(taskId, task);
  const data = await api.fetchTasks();
  globalTasks = data;
  notifyListeners();
  return result;
}

export async function archiveTask(taskId: string) {
  const result = await api.archiveTask(taskId);
  const data = await api.fetchTasks();
  globalTasks = data;
  notifyListeners();
  return result;
}

export async function unarchiveTask(taskId: string) {
  const result = await api.unarchiveTask(taskId);
  const data = await api.fetchTasks();
  globalTasks = data;
  notifyListeners();
  return result;
}

export async function deleteTask(taskId: string) {
  await api.deleteTask(taskId);
  const data = await api.fetchTasks();
  globalTasks = data;
  notifyListeners();
}

export async function addComment(taskId: string, content: string) {
  const result = await api.addComment(taskId, content);
  const data = await api.fetchTasks();
  globalTasks = data;
  notifyListeners();
  return result;
}
