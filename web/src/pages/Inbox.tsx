import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Sheet,
  SheetContent,
} from "@/components/ui/sheet";
import { cn } from "@/lib/utils";
import { ChevronLeft, ChevronRight, FileText, Menu, PanelRightClose, Plus } from "lucide-react";
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

  // Responsive layout states
  const [isSidebarCollapsed, setIsSidebarCollapsed] = useState(false); // PC only
  const [isSidebarOpen, setIsSidebarOpen] = useState(false); // Mobile + Medium sheet
  const [isArtifactDrawerOpen, setIsArtifactDrawerOpen] = useState(false); // Mobile + Medium (collapsed) sheet
  const [isArtifactCollapsed, setIsArtifactCollapsed] = useState(false); // Medium only

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

  // Check if there are artifacts to show
  const hasArtifacts = useMemo(() => {
    return allEvents.some((e) => e.kind.startsWith("artifact."));
  }, [allEvents]);

  // Auto-collapse artifact panel when no artifacts
  useEffect(() => {
    if (!hasArtifacts) {
      setIsArtifactCollapsed(true);
    } else {
      setIsArtifactCollapsed(false);
    }
  }, [hasArtifacts]);

  // Handler for closing artifact preview
  const handleCloseArtifact = () => {
    setSearchParams({});
  };

  return (
    <div className="h-screen flex flex-col overflow-hidden">
      {/* Top Bar - visible on mobile and medium (< lg) */}
      <div className="lg:hidden flex items-center justify-between h-14 px-4 border-b border-slate-200 bg-white">
        <div className="flex items-center gap-2">
          <Button
            variant="ghost"
            size="icon"
            onClick={() => setIsSidebarOpen(true)}
          >
            <Menu className="h-5 w-5" />
          </Button>
          <span className="font-medium">Tasks</span>
        </div>
        {/* Artifact button:
            - Mobile: always show when has artifacts
            - Medium: show when collapsed */}
        {hasArtifacts && selectedTaskId && (
          <Button
            variant="ghost"
            size="icon"
            className={cn(
              // On medium, only show when collapsed
              "md:hidden",
              isArtifactCollapsed && "md:flex"
            )}
            onClick={() => setIsArtifactDrawerOpen(true)}
          >
            <FileText className="h-5 w-5" />
          </Button>
        )}
      </div>

      <div className="flex-1 flex overflow-hidden">
        {/* Column 1: Project Task List */}
        {/* PC (lg+): visible with collapse toggle */}
        <div
          className={cn(
            "hidden lg:flex flex-col border-r border-slate-200 bg-white transition-all duration-300",
            isSidebarCollapsed ? "w-0 overflow-hidden" : "w-80"
          )}
          data-testid="project-list"
        >
          <ProjectTaskList />
        </div>

        {/* Sidebar collapse toggle button - PC only */}
        <div className="hidden lg:flex items-center">
          <Button
            variant="ghost"
            size="icon"
            className="h-8 w-8 -ml-4 rounded-full border border-slate-200 bg-white shadow-sm hover:bg-slate-100"
            onClick={() => setIsSidebarCollapsed(!isSidebarCollapsed)}
          >
            {isSidebarCollapsed ? (
              <ChevronRight className="h-4 w-4" />
            ) : (
              <ChevronLeft className="h-4 w-4" />
            )}
          </Button>
        </div>

        {/* Sidebar sheet - Mobile + Medium */}
        <Sheet open={isSidebarOpen} onOpenChange={setIsSidebarOpen}>
          <SheetContent side="left" className="w-80 p-0">
            <ProjectTaskList />
          </SheetContent>
        </Sheet>

        {/* Column 2: Task Detail Panel */}
        <div
          className={cn(
            "flex-1 md:flex-none md:w-[400px] lg:w-[480px] border-r border-slate-200 bg-white overflow-hidden relative",
            // When only sidebar is collapsed on PC, allow more space with limit
            isSidebarCollapsed && !isArtifactCollapsed && "lg:flex-1 lg:max-w-[600px]",
            // When artifact is collapsed, take full remaining width
            isArtifactCollapsed && "md:flex-1 md:w-auto lg:w-auto lg:max-w-none"
          )}
          data-testid="task-detail-panel"
        >
          <TaskDetailPanel
            events={allEvents}
            isConnected={isConnected}
            isLoading={isLoadingHistory || isReplaying}
          />
          {/* Artifact expand button - shown on md+ when collapsed */}
          {isArtifactCollapsed && hasArtifacts && selectedTaskId && (
            <Button
              variant="ghost"
              size="icon"
              className="hidden md:flex absolute top-2 right-2 z-10 h-8 w-8 rounded-full border border-slate-200 bg-white shadow-sm hover:bg-slate-100"
              onClick={() => setIsArtifactCollapsed(false)}
            >
              <FileText className="h-4 w-4" />
            </Button>
          )}
        </div>

        {/* Column 3: Artifact Preview */}
        {/* PC + Medium: visible unless collapsed */}
        {/* Mobile: hidden (use sheet) */}
        <div
          className={cn(
            "hidden flex-1 min-w-0 bg-slate-50 overflow-hidden relative",
            // Show on md+ unless collapsed
            !isArtifactCollapsed && "md:block",
            // Hide when no task selected
            !selectedTaskId && "md:hidden"
          )}
          data-testid="artifact-preview"
        >
          {/* Collapse button for md+ screens */}
          <Button
            variant="ghost"
            size="icon"
            className="hidden md:flex absolute top-2 right-2 z-10 h-8 w-8 rounded-full border border-slate-200 bg-white shadow-sm hover:bg-slate-100"
            onClick={() => setIsArtifactCollapsed(true)}
          >
            <PanelRightClose className="h-4 w-4" />
          </Button>
          <ArtifactPreview events={allEvents} onClose={handleCloseArtifact} />
        </div>

        {/* Artifact drawer - Mobile + Medium (when collapsed) */}
        <Sheet open={isArtifactDrawerOpen} onOpenChange={setIsArtifactDrawerOpen}>
          <SheetContent side="bottom" className="h-[80vh] p-0">
            <ArtifactPreview
              events={allEvents}
              onClose={() => {
                setIsArtifactDrawerOpen(false);
                // On medium, also uncollapse when closing drawer
                setIsArtifactCollapsed(false);
              }}
            />
          </SheetContent>
        </Sheet>
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
