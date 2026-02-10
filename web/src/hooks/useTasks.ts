import { useCallback, useEffect, useState } from "react";
import * as api from "../api/tasks";
import type { TaskResponse, TaskStatus } from "../api/types";

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
      const { data } = await api.fetchInboxTasks({});
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
      const { data } = await api.fetchBacklogTasks({});
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
      const { data } = await api.fetchTask({ task_id: taskId });
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
      const { data } = await api.fetchTodayDoneTasks({});
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
  const [inboxRes, backlogRes, todayDoneRes] = await Promise.all([
    api.fetchInboxTasks({}),
    api.fetchBacklogTasks({}),
    api.fetchTodayDoneTasks({}),
  ]);
  globalTasks = inboxRes.data;
  globalBacklogTasks = backlogRes.data;
  globalTodayDoneTasks = todayDoneRes.data;
  notifyListeners();
  notifyBacklogListeners();
  notifyTodayDoneListeners();
}

// Re-export API functions with auto-refresh
export async function createTask(task: {
  content: string;
  priority: number;
  project_id: string;
  status: TaskStatus;
}) {
  const { data } = await api.createTask(task);
  await refreshAllTasks();
  return data;
}

export async function updateTaskStatus(taskId: string, status: TaskStatus) {
  const { data } = await api.updateTaskStatus({
    task_id: taskId,
    status,
  });
  await refreshAllTasks();
  return data;
}

export async function updateTask(
  taskId: string,
  task: { content: string; priority: number; project_id: string }
) {
  const { data } = await api.updateTask({
    task_id: taskId,
    ...task,
  });
  await refreshAllTasks();
  return data;
}

export async function archiveTask(taskId: string) {
  const { data } = await api.archiveTask({ task_id: taskId });
  await refreshAllTasks();
  return data;
}

export async function unarchiveTask(taskId: string) {
  const { data } = await api.unarchiveTask({ task_id: taskId });
  await refreshAllTasks();
  return data;
}

export async function deleteTask(taskId: string) {
  await api.deleteTask({ task_id: taskId });
  await refreshAllTasks();
}

export async function addComment(taskId: string, content: string) {
  const { data } = await api.addComment({ task_id: taskId, content });
  await refreshAllTasks();
  return data;
}
