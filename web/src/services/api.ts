import axios from 'axios'

import {setupInterceptors} from './interceptors'
import {mutate} from "swr";

export const api = setupInterceptors(axios.create({
    baseURL: import.meta.env.VITE_API_URL
}))


export const fetcher = async (url: string) => {
    const res = await api.get(url);
    return res.data;
}


export const createTask = async (newTask: any) => {
    await api.post("/tasks", newTask);
}
export const updateTaskStatus = async (taskId: string, newStatus: string) => {
    await api.post(`/tasks/${taskId}/status`, {
        status: newStatus
    });
    await mutate("/tasks")
    await mutate(`/tasks/${taskId}`)
    await mutate("/timelines")
}

export const updateTask = async (taskId: string, taskType: "Todo" | "Stateful", priority: number, content: string, group?: string, states?: string[]) => {
    await api.put(`/tasks/${taskId}`, {
        priority, content, group, states, task_type: taskType
    });
    await mutate("/tasks")
    await mutate(`/tasks/${taskId}`)
    await mutate("/timelines")
}

export const deleteTask = async (taskId: string) => {
    await api.delete(`/tasks/${taskId}`,);
    await mutate("/tasks")
    await mutate("/timelines")
}

export const archiveTask = async (taskId: string) => {
    await api.post(`/tasks/${taskId}/archive`,);
    await mutate("/tasks")
    await mutate(`/tasks/${taskId}`)
    await mutate("/timelines")
}

export const unarchiveTask = async (taskId: string) => {
    await api.post(`/tasks/${taskId}/unarchive`,);
    await mutate(`/tasks/${taskId}`)
    await mutate("/tasks")
}

export const submitNewTaskComment = async (taskId: string, content: string) => {
    await api.post(`/tasks/${taskId}/comments`, {
        content
    });
    await mutate(`/tasks/${taskId}`)
    await mutate("/tasks")
}


export const enableHabit = async (habitId: string,) => {
    await api.post(`/habits/${habitId}/enable`);
}

export const disableHabit = async (habitId: string,) => {
    await api.post(`/habits/${habitId}/disable`);
}


export const dataExport = async () => {
    const res = await api.post("/users/data");
    return res.data
}
