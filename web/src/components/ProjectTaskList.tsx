import { useMemo, useState, useCallback } from "react";
import { useSearchParams } from "react-router-dom";
import { ChevronDown, Plus, History, Loader2 } from "lucide-react";
import { orderBy, groupBy } from "lodash";
import { cn } from "@/lib/utils";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Badge } from "@/components/ui/badge";
import TaskItem from "./TaskItem";
import { useTasks } from "../hooks/useTasks";
import { useProjects } from "../hooks/useProjects";
import { fetchProjectDoneTasks } from "../api/projects";
import TaskCreateModal from "../modals/TaskCreateModal";
import {
  getTaskPhase,
  getPhaseLabel,
  isTerminalStatus,
  type TaskPhase,
} from "@/lib/taskStatus";
import type { TaskResponse as Task, Project } from "../api/types";

const DONE_TASKS_PAGE_SIZE = 20;

interface ProjectGroupProps {
  project: Project;
  tasks: Task[];
  doneTasks: Task[];
  expanded: boolean;
  onToggle: () => void;
  selectedTaskId?: string;
  onTaskSelect: (taskId: string) => void;
  onAddTask: (projectId: string) => void;
  onLoadDoneTasks: (projectId: string) => void;
  isLoadingDone: boolean;
  hasMoreDone: boolean;
  showDone: boolean;
}

const PHASE_ORDER: TaskPhase[] = ["simple", "plan", "coding", "cross-review", "done"];

const PHASE_COLORS: Record<TaskPhase, string> = {
  simple: "bg-slate-100",
  plan: "bg-purple-100",
  coding: "bg-blue-100",
  "cross-review": "bg-amber-100",
  done: "bg-green-100",
};

function ProjectGroup({
  project,
  tasks,
  doneTasks,
  expanded,
  onToggle,
  selectedTaskId,
  onTaskSelect,
  onAddTask,
  onLoadDoneTasks,
  isLoadingDone,
  hasMoreDone,
  showDone,
}: ProjectGroupProps) {
  const sortedTasks = useMemo(
    () => orderBy(tasks, ["priority", "create_at"], ["desc", "asc"]),
    [tasks]
  );

  const sortedDoneTasks = useMemo(
    () => orderBy(doneTasks, ["create_at"], ["desc"]),
    [doneTasks]
  );

  // Group tasks by phase
  const tasksByPhase = useMemo(() => {
    const grouped = groupBy(sortedTasks, (task) => getTaskPhase(task.status));
    return grouped as Record<TaskPhase, Task[]>;
  }, [sortedTasks]);

  // Check if we have any agile tasks
  const hasAgileTasks = useMemo(
    () => sortedTasks.some((t) => !["backlog", "todo"].includes(t.status)),
    [sortedTasks]
  );

  const handleAddClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    onAddTask(project.id);
  };

  const handleLoadDoneClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    onLoadDoneTasks(project.id);
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
          {/* Show phase swimlanes if there are agile tasks */}
          {hasAgileTasks ? (
            <div className="space-y-3">
              {PHASE_ORDER.filter((phase) => phase !== "done").map((phase) => {
                const phaseTasks = tasksByPhase[phase] || [];
                if (phaseTasks.length === 0) return null;

                return (
                  <div key={phase} className="space-y-1">
                    <div className="flex items-center gap-2 px-2 py-1">
                      <Badge
                        variant="outline"
                        className={cn(
                          "text-[10px] font-medium",
                          PHASE_COLORS[phase]
                        )}
                      >
                        {getPhaseLabel(phase)}
                      </Badge>
                      <span className="text-[10px] text-slate-400">
                        {phaseTasks.length}
                      </span>
                    </div>
                    {phaseTasks.map((task) => (
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
                  </div>
                );
              })}
            </div>
          ) : (
            /* Simple list for non-agile tasks */
            <>
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
            </>
          )}

          {tasks.length === 0 && !showDone && (
            <div className="text-xs text-slate-400 py-2 px-3">No active tasks</div>
          )}

          {/* Done tasks section */}
          {showDone && sortedDoneTasks.length > 0 && (
            <div className="mt-3 pt-3 border-t border-slate-100">
              <div className="text-xs text-slate-400 px-3 pb-2 flex items-center gap-1.5">
                <History className="h-3 w-3" />
                Completed ({doneTasks.length})
              </div>
              {sortedDoneTasks.map((task) => (
                <div
                  key={task.id}
                  className={cn(
                    "cursor-pointer rounded-lg transition-colors opacity-60",
                    selectedTaskId === task.id
                      ? "bg-teal-50 ring-2 ring-teal-500 ring-inset opacity-100"
                      : "hover:bg-slate-50 hover:opacity-80"
                  )}
                  onClick={() => onTaskSelect(task.id)}
                >
                  <TaskItem {...task} compact />
                </div>
              ))}
            </div>
          )}

          {/* Load more button */}
          <button
            className={cn(
              "w-full text-xs text-slate-400 hover:text-slate-600 py-2 px-3 text-left hover:bg-slate-50 rounded-lg transition-colors flex items-center gap-1.5",
              isLoadingDone && "cursor-not-allowed"
            )}
            onClick={handleLoadDoneClick}
            disabled={isLoadingDone}
          >
            {isLoadingDone ? (
              <>
                <Loader2 className="h-3 w-3 animate-spin" />
                Loading...
              </>
            ) : hasMoreDone || !showDone ? (
              <>
                <History className="h-3 w-3" />
                {showDone ? "Load more completed" : "Load completed tasks"}
              </>
            ) : doneTasks.length > 0 ? (
              <span className="text-slate-300">No more completed tasks</span>
            ) : null}
          </button>
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

  // Track user's manual expand/collapse choices
  const [expandedProjects, setExpandedProjects] = useState<Set<string>>(
    () => new Set()
  );
  const [collapsedProjects, setCollapsedProjects] = useState<Set<string>>(
    () => new Set()
  );

  // Track task creation modal
  const [showTaskCreateModal, setShowTaskCreateModal] = useState(false);
  const [selectedProjectForTask, setSelectedProjectForTask] = useState<string | null>(null);

  // Track done tasks per project
  const [doneTasks, setDoneTasks] = useState<Map<string, Task[]>>(
    () => new Map()
  );
  const [doneTasksLoading, setDoneTasksLoading] = useState<Set<string>>(
    () => new Set()
  );
  const [doneTasksHasMore, setDoneTasksHasMore] = useState<Map<string, boolean>>(
    () => new Map()
  );

  const handleTaskSelect = (taskId: string) => {
    setSearchParams({ task: taskId });
  };

  const toggleProject = (projectId: string, currentlyExpanded: boolean) => {
    if (currentlyExpanded) {
      // Collapse: add to collapsed, remove from expanded
      setCollapsedProjects((prev) => new Set([...prev, projectId]));
      setExpandedProjects((prev) => {
        const next = new Set(prev);
        next.delete(projectId);
        return next;
      });
    } else {
      // Expand: add to expanded, remove from collapsed
      setExpandedProjects((prev) => new Set([...prev, projectId]));
      setCollapsedProjects((prev) => {
        const next = new Set(prev);
        next.delete(projectId);
        return next;
      });
    }
  };

  const handleAddTask = (projectId: string) => {
    setSelectedProjectForTask(projectId);
    setShowTaskCreateModal(true);
    // Ensure the project is expanded when adding a task
    setExpandedProjects((prev) => new Set([...prev, projectId]));
    setCollapsedProjects((prev) => {
      const next = new Set(prev);
      next.delete(projectId);
      return next;
    });
  };

  const loadDoneTasks = useCallback(async (projectId: string) => {
    // Prevent concurrent loads
    if (doneTasksLoading.has(projectId)) return;

    setDoneTasksLoading((prev) => new Set([...prev, projectId]));

    try {
      const currentTasks = doneTasks.get(projectId) || [];
      const offset = currentTasks.length;

      const { data: newTasks } = await fetchProjectDoneTasks({
        project_id: projectId,
        offset,
        limit: DONE_TASKS_PAGE_SIZE,
      });

      setDoneTasks((prev) => {
        const next = new Map(prev);
        const existing = next.get(projectId) || [];
        // Deduplicate by id
        const existingIds = new Set(existing.map((t) => t.id));
        const uniqueNew = newTasks.filter((t) => !existingIds.has(t.id));
        next.set(projectId, [...existing, ...uniqueNew]);
        return next;
      });

      // Check if there are more tasks
      setDoneTasksHasMore((prev) => {
        const next = new Map(prev);
        next.set(projectId, newTasks.length === DONE_TASKS_PAGE_SIZE);
        return next;
      });
    } catch (error) {
      console.error("Failed to load done tasks:", error);
    } finally {
      setDoneTasksLoading((prev) => {
        const next = new Set(prev);
        next.delete(projectId);
        return next;
      });
    }
  }, [doneTasks, doneTasksLoading]);

  // Group tasks by project
  const tasksByProject = useMemo(() => {
    const grouped = new Map<string, Task[]>();

    // Initialize all projects
    projects.forEach((project) => {
      grouped.set(project.id, []);
    });

    // Group tasks (exclude terminal states and archived)
    tasks
      .filter((task) => !isTerminalStatus(task.status) && !task.archived)
      .forEach((task) => {
        const projectId = task.project_id;
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
          {tasks.filter((t) => !isTerminalStatus(t.status) && !t.archived).length} active tasks
        </p>
      </div>

      <div className="space-y-1">
        {sortedProjects.map((project) => {
          const projectTasks = tasksByProject.get(project.id) || [];
          const projectDoneTasks = doneTasks.get(project.id) || [];
          const hasActiveTasks = projectTasks.length > 0;
          const hasDoneTasks = projectDoneTasks.length > 0;

          // Determine expand state: user choice > default (has active tasks)
          let shouldExpand: boolean;
          if (collapsedProjects.has(project.id)) {
            shouldExpand = false;
          } else if (expandedProjects.has(project.id)) {
            shouldExpand = true;
          } else {
            // Default: expand if has active tasks or loaded done tasks
            shouldExpand = hasActiveTasks || hasDoneTasks;
          }

          return (
            <ProjectGroup
              key={project.id}
              project={project}
              tasks={projectTasks}
              doneTasks={projectDoneTasks}
              expanded={shouldExpand}
              onToggle={() => toggleProject(project.id, shouldExpand)}
              selectedTaskId={selectedTaskId}
              onTaskSelect={handleTaskSelect}
              onAddTask={handleAddTask}
              onLoadDoneTasks={loadDoneTasks}
              isLoadingDone={doneTasksLoading.has(project.id)}
              hasMoreDone={doneTasksHasMore.get(project.id) ?? true}
              showDone={projectDoneTasks.length > 0}
            />
          );
        })}
      </div>

      {/* Task Create Modal */}
      <Dialog
        open={showTaskCreateModal}
        onOpenChange={(open) => {
          setShowTaskCreateModal(open);
          if (!open) setSelectedProjectForTask(null);
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Create New Task</DialogTitle>
          </DialogHeader>
          <TaskCreateModal
            open={showTaskCreateModal}
            onOpenChange={setShowTaskCreateModal}
            projectId={selectedProjectForTask || undefined}
            onSuccess={() => {
              setShowTaskCreateModal(false);
              setSelectedProjectForTask(null);
            }}
          />
        </DialogContent>
      </Dialog>
    </div>
  );
}
