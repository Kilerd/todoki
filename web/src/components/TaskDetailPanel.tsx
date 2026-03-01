import { useMemo, useState } from "react";
import { useSearchParams, useNavigate } from "react-router-dom";
import {
  X,
  Calendar,
  Tag,
  Clock,
  Play,
  Check,
  Archive,
  RotateCcw,
  Trash2,
  Inbox as InboxIcon,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { ConversationView } from "./chat";
import type { Event } from "@/hooks/useEventStream";
import {
  useTasks,
  updateTaskStatus,
  archiveTask,
  unarchiveTask,
  deleteTask,
} from "../hooks/useTasks";
import { useProjects } from "../hooks/useProjects";
import { executeTask } from "../api/tasks";
import { useToast } from "@/hooks/use-toast";
import { format } from "date-fns";
import type { TaskStatus } from "../api/types";

interface TaskDetailPanelProps {
  events?: Event[];
  isConnected?: boolean;
  isLoading?: boolean;
}

export default function TaskDetailPanel({
  events = [],
  isConnected,
  isLoading,
}: TaskDetailPanelProps) {
  const [searchParams, setSearchParams] = useSearchParams();
  const navigate = useNavigate();
  const selectedTaskId = searchParams.get("task");
  const { tasks, refresh } = useTasks();
  const { projects } = useProjects();
  const { toast } = useToast();
  const [isExecuting, setIsExecuting] = useState(false);

  const task = useMemo(
    () => tasks.find((t) => t.id === selectedTaskId),
    [tasks, selectedTaskId]
  );

  const project = useMemo(
    () => projects.find((p) => p.id === task?.project?.id),
    [projects, task]
  );

  const handleClose = () => {
    setSearchParams({});
  };

  const handleStatusChange = async (status: TaskStatus) => {
    if (!selectedTaskId) return;
    await updateTaskStatus(selectedTaskId, status);
    refresh();
  };

  const handleArchive = async () => {
    if (!selectedTaskId) return;
    await archiveTask(selectedTaskId);
    refresh();
  };

  const handleUnarchive = async () => {
    if (!selectedTaskId) return;
    await unarchiveTask(selectedTaskId);
    refresh();
  };

  const handleDelete = async () => {
    if (!selectedTaskId) return;
    await deleteTask(selectedTaskId);
    setSearchParams({});
  };

  const handleExecute = async () => {
    if (!selectedTaskId) return;
    setIsExecuting(true);
    try {
      const { data } = await executeTask({ task_id: selectedTaskId });
      toast({
        title: "Task execution started",
        description: `Agent ${data.agent.name} is now running`,
      });
      refresh();
    } catch (e) {
      toast({
        title: "Failed to execute task",
        description: e instanceof Error ? e.message : "Unknown error",
        variant: "destructive",
      });
    } finally {
      setIsExecuting(false);
    }
  };

  if (!selectedTaskId || !task) {
    return (
      <div className="h-full flex items-center justify-center text-slate-400 text-sm">
        Select a task to view details
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col bg-white">
      {/* Header */}
      <div className="flex items-start justify-between p-4 border-b border-slate-200">
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 mb-2">
            {task.priority > 0 && (
              <span className="text-red-500 font-medium text-sm">
                {"!".repeat(task.priority)}
              </span>
            )}
            <Badge
              variant="outline"
              className={
                task.status === "in-progress"
                  ? "bg-blue-50 text-blue-700 border-blue-200"
                  : task.status === "in-review"
                  ? "bg-purple-50 text-purple-700 border-purple-200"
                  : "bg-slate-50 text-slate-700 border-slate-200"
              }
            >
              {task.status}
            </Badge>
          </div>
          <h2 className="text-lg font-semibold text-slate-800 break-words">
            {task.content}
          </h2>
        </div>
        <Button
          variant="ghost"
          size="icon"
          onClick={handleClose}
          className="flex-shrink-0 ml-2"
        >
          <X className="h-4 w-4" />
        </Button>
      </div>

      {/* Metadata */}
      <div className="p-4 border-b border-slate-100 space-y-3">
        {project && (
          <div className="flex items-center gap-2 text-sm">
            <Tag className="h-4 w-4 text-slate-400" />
            <span className="text-slate-600">Project:</span>
            <div className="flex items-center gap-1.5">
              <div
                className="w-2 h-2 rounded-full"
                style={{ backgroundColor: project.color || "#94a3b8" }}
              />
              <span className="font-medium text-slate-700">{project.name}</span>
            </div>
          </div>
        )}

        {task.create_at && (
          <div className="flex items-center gap-2 text-sm">
            <Calendar className="h-4 w-4 text-slate-400" />
            <span className="text-slate-600">Created:</span>
            <span className="text-slate-700">
              {format(new Date(task.create_at), "MMM d, yyyy 'at' h:mm a")}
            </span>
          </div>
        )}

        {task.update_at && task.update_at !== task.create_at && (
          <div className="flex items-center gap-2 text-sm">
            <Clock className="h-4 w-4 text-slate-400" />
            <span className="text-slate-600">Updated:</span>
            <span className="text-slate-700">
              {format(new Date(task.update_at), "MMM d, yyyy 'at' h:mm a")}
            </span>
          </div>
        )}
      </div>

      {/* Actions */}
      <div className="flex flex-wrap items-center gap-2 p-4 border-b border-slate-100">
        {task.status === "backlog" && !task.archived && (
          <Button
            variant="outline"
            size="sm"
            onClick={() => handleStatusChange("todo")}
            className="cursor-pointer"
          >
            <InboxIcon className="h-4 w-4 mr-1.5" />
            Move to Inbox
          </Button>
        )}

        {["todo", "in-progress", "in-review"].includes(task.status) &&
          !task.archived && (
            <>
              <Button
                variant="outline"
                size="sm"
                onClick={() => handleStatusChange("backlog")}
                className="cursor-pointer"
              >
                <Clock className="h-4 w-4 mr-1.5" />
                Later
              </Button>
              <Button
                size="sm"
                onClick={() => handleStatusChange("done")}
                className="bg-teal-600 hover:bg-teal-700 cursor-pointer"
              >
                <Check className="h-4 w-4 mr-1.5" />
                Done
              </Button>
              {(!task.agent || task.agent.status !== "running") && (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleExecute}
                  disabled={isExecuting}
                  className="cursor-pointer"
                >
                  <Play className="h-4 w-4 mr-1.5" />
                  {isExecuting ? "Starting..." : "Execute on Relay"}
                </Button>
              )}
            </>
          )}

        {task.status === "done" && !task.archived && (
          <>
            <Button
              variant="outline"
              size="sm"
              onClick={() => handleStatusChange("todo")}
              className="cursor-pointer"
            >
              <RotateCcw className="h-4 w-4 mr-1.5" />
              Reopen
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={handleArchive}
              className="cursor-pointer"
            >
              <Archive className="h-4 w-4 mr-1.5" />
              Archive
            </Button>
          </>
        )}

        {task.archived && (
          <Button
            variant="outline"
            size="sm"
            onClick={handleUnarchive}
            className="cursor-pointer"
          >
            <RotateCcw className="h-4 w-4 mr-1.5" />
            Restore
          </Button>
        )}

        <Button
          variant="ghost"
          size="sm"
          onClick={handleDelete}
          className="text-red-500 hover:text-red-600 hover:bg-red-50 ml-auto cursor-pointer"
        >
          <Trash2 className="h-4 w-4 mr-1.5" />
          Delete
        </Button>
      </div>

      {/* Conversation View */}
      <div className="flex-1 overflow-hidden flex flex-col">
        <div className="px-4 py-3 border-b border-slate-100 flex items-center justify-between">
          <div>
            <h3 className="text-sm font-semibold text-slate-700">Conversation</h3>
            <p className="text-xs text-slate-500 mt-0.5">
              Agent activity and messages
            </p>
          </div>
          <div className="flex items-center gap-2">
            <div
              className={`w-2 h-2 rounded-full ${isConnected ? "bg-green-500" : "bg-slate-300"}`}
            />
            <span className="text-xs text-slate-500">
              {isConnected ? "Live" : "Offline"}
            </span>
          </div>
        </div>
        <ConversationView
          events={events}
          isConnected={isConnected}
          isLoading={isLoading}
          autoScroll
          className="flex-1"
        />
      </div>
    </div>
  );
}
