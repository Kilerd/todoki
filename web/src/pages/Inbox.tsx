import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { cn } from "@/lib/utils";
import { Plus } from "lucide-react";
import { useState } from "react";
import { useSearchParams } from "react-router-dom";
import ProjectTaskList from "../components/ProjectTaskList";
import TaskDetailPanel from "../components/TaskDetailPanel";
import ArtifactPreview from "../components/ArtifactPreview";
import TaskCreateModal from "../modals/TaskCreateModal";

function Inbox() {
  const [searchParams] = useSearchParams();
  const selectedTaskId = searchParams.get("task");
  const [showTaskCreateModal, setShowTaskCreateModal] = useState(false);

  return (
    <div className="h-screen overflow-hidden relative">
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

      {/* Floating Action Button */}
      <Button
        className="fixed bottom-6 right-6 h-14 w-14 rounded-full shadow-lg hover:shadow-xl transition-shadow"
        onClick={() => setShowTaskCreateModal(true)}
        title="Create new task"
      >
        <Plus className="h-6 w-6" />
      </Button>

      {/* Task Create Modal */}
      <Dialog
        open={showTaskCreateModal}
        onOpenChange={setShowTaskCreateModal}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Create New Task</DialogTitle>
          </DialogHeader>
          <TaskCreateModal
            open={showTaskCreateModal}
            onOpenChange={setShowTaskCreateModal}
            onSuccess={() => setShowTaskCreateModal(false)}
          />
        </DialogContent>
      </Dialog>
    </div>
  );
}

export default Inbox;
