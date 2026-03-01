import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import { useOs } from "@mantine/hooks";
import { Plus } from "lucide-react";
import { useMemo, useState } from "react";
import { useSearchParams } from "react-router-dom";
import NavBar from "../components/NavBar";
import ProjectTaskList from "../components/ProjectTaskList";
import TaskDetailPanel from "../components/TaskDetailPanel";
import ArtifactPreview from "../components/ArtifactPreview";
import { createTask } from "../hooks/useTasks";
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

function Inbox() {
  const os = useOs();
  const [searchParams] = useSearchParams();
  const selectedTaskId = searchParams.get("task");
  const [newTaskText, setNewTaskText] = useState("");
  const [showProjectModal, setShowProjectModal] = useState(false);
  const [pendingTaskData, setPendingTaskData] = useState<{
    content: string;
    priority: number;
    suggestedName?: string;
  } | null>(null);

  const parsedTask = useMemo(() => parseTask(newTaskText), [newTaskText]);

  const handleNewTask = async () => {
    if (newTaskText.trim() === "") return;

    // If user typed +tag, try to find or create that project
    if (parsedTask.group) {
      const existingProject = await getProjectByName(parsedTask.group);
      if (existingProject) {
        // Project exists, create task directly
        await createTask({
          content: parsedTask.content,
          priority: parsedTask.priority,
          project_id: existingProject.id,
          status: "todo",
        });
        setNewTaskText("");
      } else {
        // Project doesn't exist, show modal to confirm creation
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
      status: "todo",
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
    <div className="h-screen overflow-hidden">
      <div className="h-full grid grid-cols-[320px_1fr] lg:grid-cols-[320px_480px_1fr] gap-0">
        {/* Column 1: Project Task List */}
        <div
          className="border-r border-slate-200 bg-white overflow-hidden"
          data-testid="project-list"
        >
          <ProjectTaskList />
        </div>

        {/* Column 2: Task Detail Panel */}
        <div
          className={cn(
            "border-r border-slate-200 bg-white overflow-hidden",
            !selectedTaskId && "hidden lg:block"
          )}
          data-testid="task-detail-panel"
        >
          <TaskDetailPanel />
        </div>

        {/* Column 3: Artifact Preview */}
        <div
          className={cn(
            "bg-slate-50 overflow-hidden",
            !selectedTaskId && "hidden"
          )}
          data-testid="artifact-preview"
        >
          <ArtifactPreview />
        </div>
      </div>

      {/* Project Selection Modal */}
      <Dialog
        open={showProjectModal}
        onOpenChange={(open) => {
          setShowProjectModal(open);
          if (!open) setPendingTaskData(null);
        }}
      >
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

export default Inbox;
