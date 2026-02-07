import { useCallback, useEffect, useState } from "react";
import * as api from "../api/tasks";
import type { TaskResponse, TaskStatus } from "../api/schema";

// Simple global state for tasks to enable refresh across components
let globalTasks: TaskResponse[] = [];
let globalBacklogTasks: TaskResponse[] = [];
let globalListeners: Set<() => void> = new Set();
let globalBacklogListeners: Set<() => void> = new Set();

function notifyListeners() {
  globalListeners.forEach((listener) => listener());
}

function notifyBacklogListeners() {
  globalBacklogListeners.forEach((listener) => listener());
}

export function useTasks() {
  const [tasks, setTasks] = useState<TaskResponse[]>(globalTasks);
  const [isLoading, setIsLoading] = useState(globalTasks.length === 0);

  const refresh = useCallback(async () => {
    setIsLoading(true);
    try {
      const data = await api.fetchInboxTasks();
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

export function useBacklogTasks() {
  const [tasks, setTasks] = useState<TaskResponse[]>(globalBacklogTasks);
  const [isLoading, setIsLoading] = useState(globalBacklogTasks.length === 0);

  const refresh = useCallback(async () => {
    setIsLoading(true);
    try {
      const data = await api.fetchBacklogTasks();
      globalBacklogTasks = data;
      setTasks(data);
      notifyBacklogListeners();
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    const listener = () => setTasks(globalBacklogTasks);
    globalBacklogListeners.add(listener);

    if (globalBacklogTasks.length === 0) {
      refresh();
    }

    return () => {
      globalBacklogListeners.delete(listener);
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

let globalTodayDoneTasks: TaskResponse[] = [];
let globalTodayDoneListeners: Set<() => void> = new Set();

function notifyTodayDoneListeners() {
  globalTodayDoneListeners.forEach((listener) => listener());
}

export function useTodayDoneTasks() {
  const [tasks, setTasks] = useState<TaskResponse[]>(globalTodayDoneTasks);
  const [isLoading, setIsLoading] = useState(globalTodayDoneTasks.length === 0);

  const refresh = useCallback(async () => {
    setIsLoading(true);
    try {
      const data = await api.fetchTodayDoneTasks();
      globalTodayDoneTasks = data;
      setTasks(data);
      notifyTodayDoneListeners();
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    const listener = () => setTasks(globalTodayDoneTasks);
    globalTodayDoneListeners.add(listener);

    refresh();

    return () => {
      globalTodayDoneListeners.delete(listener);
    };
  }, [refresh]);

  return { tasks, isLoading, refresh };
}

async function refreshAllTasks() {
  const [inboxData, backlogData, todayDoneData] = await Promise.all([
    api.fetchInboxTasks(),
    api.fetchBacklogTasks(),
    api.fetchTodayDoneTasks(),
  ]);
  globalTasks = inboxData;
  globalBacklogTasks = backlogData;
  globalTodayDoneTasks = todayDoneData;
  notifyListeners();
  notifyBacklogListeners();
  notifyTodayDoneListeners();
}

// Re-export API functions with auto-refresh
export async function createTask(task: Parameters<typeof api.createTask>[0]) {
  const result = await api.createTask(task);
  await refreshAllTasks();
  return result;
}

export async function updateTaskStatus(taskId: string, status: TaskStatus) {
  const result = await api.updateTaskStatus(taskId, status);
  await refreshAllTasks();
  return result;
}

export async function updateTask(
  taskId: string,
  task: Parameters<typeof api.updateTask>[1]
) {
  const result = await api.updateTask(taskId, task);
  await refreshAllTasks();
  return result;
}

export async function archiveTask(taskId: string) {
  const result = await api.archiveTask(taskId);
  await refreshAllTasks();
  return result;
}

export async function unarchiveTask(taskId: string) {
  const result = await api.unarchiveTask(taskId);
  await refreshAllTasks();
  return result;
}

export async function deleteTask(taskId: string) {
  await api.deleteTask(taskId);
  await refreshAllTasks();
}

export async function addComment(taskId: string, content: string) {
  const result = await api.addComment(taskId, content);
  await refreshAllTasks();
  return result;
}
