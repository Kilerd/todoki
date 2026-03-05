import { useMemo, useState, useEffect } from "react";
import { useSearchParams } from "react-router-dom";
import {
  X,
  Clock,
  Play,
  Check,
  Archive,
  RotateCcw,
  Trash2,
  Inbox as InboxIcon,
  Send,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Textarea } from "@/components/ui/textarea";
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
import { executeTask, fetchTask, fetchTaskExecution } from "../api/tasks";
import { emitEvent } from "../api/eventBus";
import { useToast } from "@/hooks/use-toast";
import { format } from "date-fns";
import type { TaskStatus, TaskResponse } from "../api/types";

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
  const selectedTaskId = searchParams.get("task");
  const { tasks, refresh } = useTasks();
  const { projects } = useProjects();
  const { toast } = useToast();
  const [isExecuting, setIsExecuting] = useState(false);
  const [fetchedTask, setFetchedTask] = useState<TaskResponse | null>(null);
  const [humanInput, setHumanInput] = useState("");
  const [isSendingInput, setIsSendingInput] = useState(false);

  // First try to find task in inbox tasks
  const inboxTask = useMemo(
    () => tasks.find((t) => t.id === selectedTaskId),
    [tasks, selectedTaskId]
  );

  // Fetch task separately if not found in inbox (e.g., done tasks)
  useEffect(() => {
    if (selectedTaskId && !inboxTask) {
      fetchTask({ task_id: selectedTaskId })
        .then(({ data }) => setFetchedTask(data))
        .catch(() => setFetchedTask(null));
    } else {
      setFetchedTask(null);
    }
  }, [selectedTaskId, inboxTask]);

  const task = inboxTask || fetchedTask;

  const project = useMemo(
    () => projects.find((p) => p.id === task?.project_id),
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

  const handleSendInput = async () => {
    if (!selectedTaskId || !humanInput.trim()) return;
    setIsSendingInput(true);
    try {
      // If agent is running, send to relay; otherwise send as human message
      if (task?.agent?.status === "running") {
        // Get execution info (relay_id, session_id)
        const { data: execInfo } = await fetchTaskExecution({ task_id: selectedTaskId });

        // Send input via event bus to relay
        await emitEvent({
          agent_id: "00000000-0000-0000-0000-000000000001",
          kind: "relay.input_requested",
          data: {
            relay_id: execInfo.relay_id,
            session_id: execInfo.session_id,
            input: humanInput + "\n",
          },
        });
      } else {
        // Send as human message for PM or other listeners
        await emitEvent({
          agent_id: "00000000-0000-0000-0000-000000000001",
          task_id: selectedTaskId,
          kind: "human.message",
          data: {
            content: humanInput,
          },
        });
      }

      setHumanInput("");
      toast({
        title: "Message sent",
        description: task?.agent?.status === "running"
          ? "Your message was sent to the agent"
          : "Your message was posted to the conversation",
      });
    } catch (e) {
      toast({
        title: "Failed to send message",
        description: e instanceof Error ? e.message : "Unknown error",
        variant: "destructive",
      });
    } finally {
      setIsSendingInput(false);
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
      {/* Compact Header with Metadata */}
      <div className="flex items-center gap-2 p-2 border-b border-slate-200 flex-wrap">
        <div className="flex items-center gap-2 flex-1 min-w-0">
          {task.priority > 0 && (
            <span className="text-red-500 font-medium text-xs">
              {"!".repeat(task.priority)}
            </span>
          )}
          <Badge
            variant="outline"
            className={
              task.status === "in-progress"
                ? "bg-blue-50 text-blue-700 border-blue-200 text-xs"
                : task.status === "in-review"
                ? "bg-purple-50 text-purple-700 border-purple-200 text-xs"
                : "bg-slate-50 text-slate-700 border-slate-200 text-xs"
            }
          >
            {task.status}
          </Badge>
          <h2 className="text-sm font-semibold text-slate-800 truncate flex-1">
            {task.content}
          </h2>
        </div>
        <div className="flex items-center gap-2 text-xs text-slate-500">
          {project && (
            <div className="flex items-center gap-1">
              <div
                className="w-1.5 h-1.5 rounded-full"
                style={{ backgroundColor: project.color || "#94a3b8" }}
              />
              <span className="font-medium">{project.name}</span>
            </div>
          )}
          {task.create_at && (
            <span className="hidden sm:inline">
              {format(new Date(task.create_at), "MMM d")}
            </span>
          )}
        </div>
        <Button
          variant="ghost"
          size="icon"
          onClick={handleClose}
          className="flex-shrink-0 h-6 w-6"
        >
          <X className="h-3 w-3" />
        </Button>
      </div>

      {/* Compact Actions */}
      <div className="flex flex-wrap items-center gap-1.5 px-2 py-1.5 border-b border-slate-100">
        {task.status === "backlog" && !task.archived && (
          <Button
            variant="outline"
            size="sm"
            onClick={() => handleStatusChange("todo")}
            className="cursor-pointer h-7 text-xs"
          >
            <InboxIcon className="h-3 w-3 mr-1" />
            Inbox
          </Button>
        )}

        {["todo", "in-progress", "in-review"].includes(task.status) &&
          !task.archived && (
            <>
              <Button
                variant="outline"
                size="sm"
                onClick={() => handleStatusChange("backlog")}
                className="cursor-pointer h-7 text-xs"
              >
                <Clock className="h-3 w-3 mr-1" />
                Later
              </Button>
              <Button
                size="sm"
                onClick={() => handleStatusChange("done")}
                className="bg-teal-600 hover:bg-teal-700 cursor-pointer h-7 text-xs"
              >
                <Check className="h-3 w-3 mr-1" />
                Done
              </Button>
              {(!task.agent || task.agent.status !== "running") && (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleExecute}
                  disabled={isExecuting}
                  className="cursor-pointer h-7 text-xs"
                >
                  <Play className="h-3 w-3 mr-1" />
                  {isExecuting ? "Starting..." : "Execute"}
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
              className="cursor-pointer h-7 text-xs"
            >
              <RotateCcw className="h-3 w-3 mr-1" />
              Reopen
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={handleArchive}
              className="cursor-pointer h-7 text-xs"
            >
              <Archive className="h-3 w-3 mr-1" />
              Archive
            </Button>
          </>
        )}

        {task.archived && (
          <Button
            variant="outline"
            size="sm"
            onClick={handleUnarchive}
            className="cursor-pointer h-7 text-xs"
          >
            <RotateCcw className="h-3 w-3 mr-1" />
            Restore
          </Button>
        )}

        <Button
          variant="ghost"
          size="sm"
          onClick={handleDelete}
          className="text-red-500 hover:text-red-600 hover:bg-red-50 cursor-pointer h-7 text-xs"
        >
          <Trash2 className="h-3 w-3 mr-1" />
          Delete
        </Button>

        {/* Live Status Indicator */}
        <div className="flex items-center gap-1.5 ml-auto">
          <div
            className={`w-1.5 h-1.5 rounded-full ${isConnected ? "bg-green-500" : "bg-slate-300"}`}
          />
          <span className="text-xs text-slate-500">
            {isConnected ? "Live" : "Offline"}
          </span>
        </div>
      </div>

      {/* Conversation View */}
      <div className="flex-1 overflow-hidden flex flex-col">
        <ConversationView
          events={events}
          isConnected={isConnected}
          isLoading={isLoading}
          autoScroll
          className="flex-1"
        />

        {/* Human Input - always available for conversation with PM or agent */}
        <div className="p-2 border-t border-slate-200 bg-slate-50">
          <div className="flex gap-2">
            <Textarea
              value={humanInput}
              onChange={(e) => setHumanInput(e.target.value)}
              placeholder={
                task.agent?.status === "running"
                  ? "Send guidance to the agent..."
                  : "Send a message..."
              }
              className="min-h-[48px] max-h-[96px] resize-none bg-white text-sm"
              onKeyDown={(e) => {
                if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
                  e.preventDefault();
                  handleSendInput();
                }
              }}
            />
            <Button
              onClick={handleSendInput}
              disabled={isSendingInput || !humanInput.trim()}
              size="sm"
              className="self-end h-8"
            >
              <Send className="h-3 w-3" />
            </Button>
          </div>
          <p className="text-xs text-slate-400 mt-1">
            Cmd+Enter to send
          </p>
        </div>
      </div>
    </div>
  );
}
