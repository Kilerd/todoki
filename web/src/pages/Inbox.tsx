import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { cn } from "@/lib/utils";
import { Plus } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useSearchParams } from "react-router-dom";
import ProjectTaskList from "../components/ProjectTaskList";
import TaskDetailPanel from "../components/TaskDetailPanel";
import ArtifactPreview from "../components/ArtifactPreview";
import TaskCreateModal from "../modals/TaskCreateModal";
import { useEventStream, type Event } from "../hooks/useEventStream";
import { queryEvents, type EventBusEvent } from "../api/eventBus";

function Inbox() {
  const [searchParams, setSearchParams] = useSearchParams();
  const selectedTaskId = searchParams.get("task");
  const [showTaskCreateModal, setShowTaskCreateModal] = useState(false);

  // Event stream for all task-related events
  const { events, isConnected, isReplaying } = useEventStream({
    kinds: ["agent.output_batch", "permission.*", "artifact.*"],
    taskId: selectedTaskId || undefined,
  });

  // Historical events
  const [historicalEvents, setHistoricalEvents] = useState<Event[]>([]);
  const [isLoadingHistory, setIsLoadingHistory] = useState(false);

  // Load historical events when task changes
  useEffect(() => {
    if (!selectedTaskId) {
      setHistoricalEvents([]);
      return;
    }

    const loadHistory = async () => {
      setIsLoadingHistory(true);
      try {
        const response = await queryEvents({
          cursor: 0,
          kinds: "agent.output_batch,permission.*,artifact.*",
          task_id: selectedTaskId,
          limit: 500,
        });

        const converted: Event[] = response.events.map((e: EventBusEvent) => ({
          cursor: e.cursor,
          kind: e.kind,
          time: e.time,
          agent_id: e.agent_id,
          session_id: e.session_id,
          task_id: e.task_id,
          data: e.data,
        }));
        setHistoricalEvents(converted);
      } catch {
        // Ignore errors
      } finally {
        setIsLoadingHistory(false);
      }
    };

    loadHistory();
  }, [selectedTaskId]);

  // Merge historical and real-time events
  const allEvents = useMemo(() => {
    const merged = [...historicalEvents, ...events];
    const seen = new Set<number>();
    return merged
      .filter((e) => {
        if (seen.has(e.cursor)) return false;
        seen.add(e.cursor);
        return true;
      })
      .sort((a, b) => a.cursor - b.cursor);
  }, [historicalEvents, events]);

  // Handler for closing artifact preview
  const handleCloseArtifact = () => {
    setSearchParams({});
  };

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
          <TaskDetailPanel
            events={allEvents}
            isConnected={isConnected}
            isLoading={isLoadingHistory || isReplaying}
          />
        </div>

        {/* Column 3: Artifact Preview */}
        <div
          className={cn(
            "bg-slate-50 overflow-hidden",
            !selectedTaskId && "hidden"
          )}
          data-testid="artifact-preview"
        >
          <ArtifactPreview events={allEvents} onClose={handleCloseArtifact} />
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
