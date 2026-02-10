import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { useOs } from "@mantine/hooks";
import { orderBy } from "lodash";
import { Clock, Plus } from "lucide-react";
import { useMemo, useState } from "react";
import NavBar from "../components/NavBar";
import TaskItem from "../components/TaskItem";
import { useBacklogTasks, createTask } from "../hooks/useTasks";
import { getProjectByName } from "../hooks/useProjects";
import ProjectSelectModal from "../modals/ProjectSelectModal";
import { parseTask } from "../utils/taskParser";
import type { Project } from "../api/types";

function Kbd({ children }: { children: React.ReactNode }) {
  return (
    <kbd className="inline-flex h-5 select-none items-center rounded border border-slate-200 bg-slate-50 px-1.5 font-mono text-[10px] font-medium text-slate-500">
      {children}
    </kbd>
  );
}

function Later() {
  const os = useOs();
  const { tasks, isLoading } = useBacklogTasks();
  const [newTaskText, setNewTaskText] = useState("");
  const [showProjectModal, setShowProjectModal] = useState(false);
  const [pendingTaskData, setPendingTaskData] = useState<{
    content: string;
    priority: number;
    suggestedName?: string;
  } | null>(null);

  const parsedTask = useMemo(() => parseTask(newTaskText), [newTaskText]);

  const backlogTasks = useMemo(() => {
    return tasks.filter((item) => item.archived === false);
  }, [tasks]);

  const handleNewTask = async () => {
    if (newTaskText.trim() === "") return;

    // If user typed +tag, try to find or create that project
    if (parsedTask.group) {
      const existingProject = await getProjectByName(parsedTask.group);
      if (existingProject) {
        await createTask({
          content: parsedTask.content,
          priority: parsedTask.priority,
          project_id: existingProject.id,
          status: "backlog",
        });
        setNewTaskText("");
      } else {
        setPendingTaskData({
          content: parsedTask.content,
          priority: parsedTask.priority,
          suggestedName: parsedTask.group,
        });
        setShowProjectModal(true);
      }
    } else {
      // No +tag specified, show project selection modal
      setPendingTaskData({
        content: parsedTask.content,
        priority: parsedTask.priority,
      });
      setShowProjectModal(true);
    }
  };

  const handleProjectSelect = async (project: Project) => {
    if (!pendingTaskData) return;

    await createTask({
      content: pendingTaskData.content,
      priority: pendingTaskData.priority,
      project_id: project.id,
      status: "backlog",
    });
    setNewTaskText("");
    setPendingTaskData(null);
    setShowProjectModal(false);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
      handleNewTask();
    }
  };

  return (
    <div className="container mx-auto mt-12 max-w-3xl">
      <NavBar />

      <div className="mt-8 space-y-6">
        {/* Task Input */}
        <section>
          <div className="flex gap-2">
            <Input
              value={newTaskText}
              onChange={(e) => setNewTaskText(e.target.value)}
              placeholder="Save for later..."
              onKeyDown={handleKeyDown}
              className="border-slate-200 focus-visible:ring-teal-500"
            />
            <Button
              disabled={newTaskText.trim() === ""}
              onClick={handleNewTask}
              className="bg-teal-600 hover:bg-teal-700 transition-colors duration-150"
            >
              <Plus className="h-4 w-4" />
            </Button>
          </div>

          {/* Hints */}
          <div className="flex flex-wrap gap-4 mt-3 text-xs text-slate-500">
            <span className="flex items-center gap-1.5">
              Group <Kbd>+tag</Kbd>
            </span>
            <span className="flex items-center gap-1.5">
              Priority <Kbd>!!!</Kbd>
            </span>
            <span className="flex items-center gap-1.5">
              Submit <Kbd>{os === "macos" ? "⌘" : "Ctrl"}+Enter</Kbd>
            </span>
          </div>

          {/* Preview */}
          {newTaskText.trim() !== "" && (
            <div className="mt-4 p-3 border border-dashed border-slate-200 rounded-lg">
              <div className="flex items-center gap-2 text-sm">
                <span className="text-slate-400">Preview:</span>
                {parsedTask.priority > 0 && (
                  <span className="text-red-500 font-medium">
                    {"!".repeat(parsedTask.priority)}
                  </span>
                )}
                <span className="text-slate-700">{parsedTask.content}</span>
                {parsedTask.group && (
                  <span className="text-xs px-1.5 py-0.5 bg-slate-100 text-slate-600 rounded">
                    {parsedTask.group}
                  </span>
                )}
              </div>
            </div>
          )}
        </section>

        {/* Task List */}
        {isLoading ? (
          <div className="space-y-3">
            <Skeleton className="h-10 w-full" />
            <Skeleton className="h-10 w-full" />
            <Skeleton className="h-10 w-full" />
          </div>
        ) : (
          <section>
            {/* Section Header */}
            <div className="flex items-center gap-3 mb-3">
              <span className="text-xs font-medium text-slate-400 uppercase tracking-wider">
                Backlog
              </span>
              {backlogTasks.length > 0 && (
                <span className="text-xs text-slate-400">
                  {backlogTasks.length}
                </span>
              )}
              <div className="flex-1 h-px bg-slate-100" />
            </div>

            {backlogTasks.length > 0 ? (
              <div className="space-y-1">
                {orderBy(
                  backlogTasks,
                  ["priority", "create_at"],
                  ["desc", "asc"]
                ).map((task) => (
                  <TaskItem key={task.id} {...task} />
                ))}
              </div>
            ) : (
              <div className="flex flex-col items-center justify-center py-16 text-slate-400">
                <Clock className="h-12 w-12 mb-4 stroke-1" />
                <p className="text-sm">No tasks saved for later</p>
                <p className="text-xs mt-1">
                  Add tasks here when you're not ready to work on them yet
                </p>
              </div>
            )}
          </section>
        )}
      </div>

      {/* Project Selection Modal */}
      <Dialog open={showProjectModal} onOpenChange={(open) => {
        setShowProjectModal(open);
        if (!open) setPendingTaskData(null);
      }}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Select Project</DialogTitle>
          </DialogHeader>
          <ProjectSelectModal
            open={showProjectModal}
            onOpenChange={setShowProjectModal}
            mode="select-or-create"
            suggestedName={pendingTaskData?.suggestedName}
            onSelect={handleProjectSelect}
          />
        </DialogContent>
      </Dialog>
    </div>
  );
}

export default Later;
