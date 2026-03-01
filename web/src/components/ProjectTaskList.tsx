import { useMemo, useState } from "react";
import { useSearchParams } from "react-router-dom";
import { ChevronDown, Plus } from "lucide-react";
import { orderBy } from "lodash";
import { cn } from "@/lib/utils";
import TaskItem from "./TaskItem";
import { useTasks, createTask } from "../hooks/useTasks";
import { useProjects } from "../hooks/useProjects";
import type { TaskResponse as Task, Project } from "../api/types";

interface ProjectGroupProps {
  project: Project;
  tasks: Task[];
  expanded: boolean;
  onToggle: () => void;
  selectedTaskId?: string;
  onTaskSelect: (taskId: string) => void;
  onAddTask: (projectId: string) => void;
  isAddingTask: boolean;
  newTaskContent: string;
  onNewTaskContentChange: (content: string) => void;
  onNewTaskSubmit: () => void;
  onNewTaskCancel: () => void;
  onNewTaskKeyDown: (e: React.KeyboardEvent) => void;
}

function ProjectGroup({
  project,
  tasks,
  expanded,
  onToggle,
  selectedTaskId,
  onTaskSelect,
  onAddTask,
  isAddingTask,
  newTaskContent,
  onNewTaskContentChange,
  onNewTaskSubmit,
  onNewTaskCancel,
  onNewTaskKeyDown,
}: ProjectGroupProps) {
  const sortedTasks = useMemo(
    () => orderBy(tasks, ["priority", "create_at"], ["desc", "asc"]),
    [tasks]
  );

  const handleAddClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    onAddTask(project.id);
  };

  return (
    <div className="mb-4">
      <div
        className={cn(
          "flex items-center gap-2 px-3 py-2 cursor-pointer select-none rounded-lg hover:bg-slate-50 transition-colors group",
          expanded && "bg-slate-50"
        )}
        onClick={onToggle}
      >
        <ChevronDown
          className={cn(
            "h-4 w-4 text-slate-400 transition-transform duration-200",
            !expanded && "-rotate-90"
          )}
        />
        <div
          className="w-2 h-2 rounded-full"
          style={{ backgroundColor: project.color || "#94a3b8" }}
        />
        <span className="text-sm font-medium text-slate-700 flex-1">
          {project.name}
        </span>
        <span className="text-xs text-slate-400">{tasks.length}</span>
        <button
          className="h-5 w-5 flex items-center justify-center rounded text-slate-400 hover:text-slate-600 hover:bg-slate-200 opacity-0 group-hover:opacity-100 transition-opacity"
          onClick={handleAddClick}
          title="Add task"
        >
          <Plus className="h-3.5 w-3.5" />
        </button>
      </div>

      {expanded && (
        <div className="mt-1 space-y-1 pl-6">
          {sortedTasks.map((task) => (
            <div
              key={task.id}
              className={cn(
                "cursor-pointer rounded-lg transition-colors",
                selectedTaskId === task.id
                  ? "bg-teal-50 ring-2 ring-teal-500 ring-inset"
                  : "hover:bg-slate-50"
              )}
              onClick={() => onTaskSelect(task.id)}
            >
              <TaskItem {...task} compact />
            </div>
          ))}
          {isAddingTask && (
            <div className="flex items-center gap-2 px-3 py-2">
              <input
                type="text"
                className="flex-1 text-sm bg-transparent border-none outline-none placeholder:text-slate-400"
                placeholder="New task..."
                value={newTaskContent}
                onChange={(e) => onNewTaskContentChange(e.target.value)}
                onKeyDown={onNewTaskKeyDown}
                onBlur={() => {
                  if (!newTaskContent.trim()) {
                    onNewTaskCancel();
                  }
                }}
                autoFocus
              />
            </div>
          )}
          {tasks.length === 0 && !isAddingTask && (
            <div className="text-xs text-slate-400 py-2 px-3">No tasks</div>
          )}
        </div>
      )}
    </div>
  );
}

export default function ProjectTaskList() {
  const { tasks, isLoading } = useTasks();
  const { projects, isLoading: isProjectsLoading } = useProjects();
  const [searchParams, setSearchParams] = useSearchParams();
  const selectedTaskId = searchParams.get("task") || undefined;

  // Track expanded state for each project (empty initially, will auto-expand based on tasks)
  const [expandedProjects, setExpandedProjects] = useState<Set<string>>(
    () => new Set()
  );

  // Track which project is in "new task" mode
  const [newTaskProjectId, setNewTaskProjectId] = useState<string | null>(null);
  const [newTaskContent, setNewTaskContent] = useState("");

  const handleTaskSelect = (taskId: string) => {
    setSearchParams({ task: taskId });
  };

  const toggleProject = (projectId: string) => {
    setExpandedProjects((prev) => {
      const next = new Set(prev);
      if (next.has(projectId)) {
        next.delete(projectId);
      } else {
        next.add(projectId);
      }
      return next;
    });
  };

  const handleAddTask = (projectId: string) => {
    setNewTaskProjectId(projectId);
    setNewTaskContent("");
    // Ensure the project is expanded when adding a task
    setExpandedProjects((prev) => new Set([...prev, projectId]));
  };

  const handleNewTaskSubmit = async () => {
    if (!newTaskContent.trim() || !newTaskProjectId) return;

    await createTask({
      content: newTaskContent.trim(),
      priority: 0,
      project_id: newTaskProjectId,
      status: "todo",
    });
    setNewTaskContent("");
    setNewTaskProjectId(null);
  };

  const handleNewTaskKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleNewTaskSubmit();
    } else if (e.key === "Escape") {
      setNewTaskProjectId(null);
      setNewTaskContent("");
    }
  };

  // Group tasks by project
  const tasksByProject = useMemo(() => {
    const grouped = new Map<string, Task[]>();

    // Initialize all projects
    projects.forEach((project) => {
      grouped.set(project.id, []);
    });

    // Group tasks
    tasks
      .filter((task) => task.status !== "done" && task.status !== "archived")
      .forEach((task) => {
        const projectId = task.project?.id;
        if (projectId) {
          if (!grouped.has(projectId)) {
            grouped.set(projectId, []);
          }
          grouped.get(projectId)!.push(task);
        }
      });

    return grouped;
  }, [tasks, projects]);

  // Sort projects: Inbox first, then by name
  const sortedProjects = useMemo(() => {
    return orderBy(
      projects,
      [(p) => (p.name.toLowerCase() === "inbox" ? 0 : 1), (p) => p.name.toLowerCase()],
      ["asc", "asc"]
    );
  }, [projects]);

  if (isLoading || isProjectsLoading) {
    return (
      <div className="p-4 space-y-2">
        <div className="h-8 bg-slate-100 rounded animate-pulse" />
        <div className="h-8 bg-slate-100 rounded animate-pulse" />
        <div className="h-8 bg-slate-100 rounded animate-pulse" />
      </div>
    );
  }

  return (
    <div className="h-full overflow-y-auto p-4">
      <div className="mb-4">
        <h2 className="text-lg font-semibold text-slate-800">Projects</h2>
        <p className="text-xs text-slate-500 mt-1">
          {tasks.filter((t) => t.status !== "done" && t.status !== "archived").length} active tasks
        </p>
      </div>

      <div className="space-y-1">
        {sortedProjects.map((project) => {
          const projectTasks = tasksByProject.get(project.id) || [];
          const hasActiveTasks = projectTasks.length > 0;

          // Auto-expand if has active tasks, or manually expanded
          const shouldExpand = expandedProjects.has(project.id) || hasActiveTasks;

          return (
            <ProjectGroup
              key={project.id}
              project={project}
              tasks={projectTasks}
              expanded={shouldExpand}
              onToggle={() => toggleProject(project.id)}
              selectedTaskId={selectedTaskId}
              onTaskSelect={handleTaskSelect}
              onAddTask={handleAddTask}
              isAddingTask={newTaskProjectId === project.id}
              newTaskContent={newTaskContent}
              onNewTaskContentChange={setNewTaskContent}
              onNewTaskSubmit={handleNewTaskSubmit}
              onNewTaskCancel={() => {
                setNewTaskProjectId(null);
                setNewTaskContent("");
              }}
              onNewTaskKeyDown={handleNewTaskKeyDown}
            />
          );
        })}
      </div>
    </div>
  );
}
