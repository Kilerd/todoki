import { useCallback, useEffect, useState } from "react";
import * as api from "../api/projects";
import type { Project } from "../api/types";

// Global state for projects
let globalProjects: Project[] = [];
let globalListeners: Set<() => void> = new Set();

function notifyListeners() {
  globalListeners.forEach((listener) => listener());
}

export function useProjects() {
  const [projects, setProjects] = useState<Project[]>(globalProjects);
  const [isLoading, setIsLoading] = useState(globalProjects.length === 0);

  const refresh = useCallback(async () => {
    setIsLoading(true);
    try {
      const { data } = await api.fetchProjects({ include_archived: false });
      globalProjects = data;
      setProjects(data);
      notifyListeners();
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    const listener = () => setProjects(globalProjects);
    globalListeners.add(listener);

    if (globalProjects.length === 0) {
      refresh();
    }

    return () => {
      globalListeners.delete(listener);
    };
  }, [refresh]);

  const getProjectById = useCallback(
    (projectId: string): Project | undefined => {
      return projects.find((p) => p.id === projectId);
    },
    [projects]
  );

  return { projects, isLoading, refresh, getProjectById };
}

// Utility function to get project from global store (for use outside React components)
export function getProjectById(projectId: string): Project | undefined {
  return globalProjects.find((p) => p.id === projectId);
}

export function useProject(projectId: string) {
  const [project, setProject] = useState<Project | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  const refresh = useCallback(async () => {
    setIsLoading(true);
    try {
      const { data } = await api.fetchProject({ project_id: projectId });
      setProject(data);
    } finally {
      setIsLoading(false);
    }
  }, [projectId]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return { project, isLoading, refresh };
}

async function refreshProjects() {
  const { data } = await api.fetchProjects({ include_archived: false });
  globalProjects = data;
  notifyListeners();
}

export async function createProject(projectData: {
  name: string;
  description?: string | null;
  color?: string | null;
}): Promise<Project> {
  const { data } = await api.createProject(projectData);
  await refreshProjects();
  return data;
}

export async function updateProject(
  projectId: string,
  projectData: {
    name?: string | null;
    description?: string | null;
    color?: string | null;
    archived?: boolean | null;
    general_template?: string | null;
    business_template?: string | null;
    coding_template?: string | null;
    qa_template?: string | null;
  }
): Promise<Project> {
  const { data } = await api.updateProject({
    project_id: projectId,
    ...projectData,
  });
  await refreshProjects();
  return data;
}

export async function deleteProject(projectId: string): Promise<void> {
  await api.deleteProject({ project_id: projectId });
  await refreshProjects();
}

export async function getProjectByName(name: string): Promise<Project | null> {
  const { data } = await api.fetchProjectByName({ name });
  return data;
}

export async function getOrCreateProject(name: string, color?: string): Promise<Project> {
  const existing = await getProjectByName(name);
  if (existing) {
    return existing;
  }
  return createProject({ name, color });
}

export type { Project };
