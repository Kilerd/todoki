import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { cn } from "@/lib/utils";
import {
  getTaskPhase,
  isTerminalStatus,
  getNextStatus,
  getStatusLabel,
  getStatusColorClasses,
} from "@/lib/taskStatus";
import dayjs from "dayjs";
import {
  Archive,
  Bot,
  Check,
  CheckCircle,
  Clock,
  Code,
  FileSearch,
  Inbox,
  Loader2,
  MessageSquare,
  MoreHorizontal,
  PenLine,
  Play,
  RefreshCcw,
  Send,
  Trash2,
  XCircle,
} from "lucide-react";
import { useState } from "react";
import { Link } from "react-router-dom";
import {
  archiveTask,
  deleteTask,
  updateTaskStatus,
  executeTask,
} from "../hooks/useTasks";
import { getProjectById } from "../hooks/useProjects";
import type { TaskResponse, TaskStatus } from "../api/types";

type Props = TaskResponse & {
  compact?: boolean;
};

export default function TaskItem(props: Props) {
  const [isLoading, setIsLoading] = useState(false);

  const handleStatusChange = async (newStatus: TaskStatus) => {
    setIsLoading(true);
    await updateTaskStatus(props.id, newStatus);
    setIsLoading(false);
  };

  const handleExecute = async () => {
    setIsLoading(true);
    try {
      await executeTask(props.id);
    } catch (e) {
      console.error("Failed to execute task:", e);
    }
    setIsLoading(false);
  };

  const standDuration = Math.trunc(
    (dayjs().unix() - dayjs(props.events[0]?.datetime).unix()) / 86400
  );

  const phase = getTaskPhase(props.status);
  const isDone = isTerminalStatus(props.status);
  const isBacklog = props.status === "backlog";
  const isTodo = props.status === "todo";
  const isActive = !isDone && !isBacklog;
  const nextStatus = getNextStatus(props.status);

  // Compact mode for three-column layout
  if (props.compact) {
    return (
      <div className="flex items-center justify-between py-2.5 px-3 group">
        <div className="flex items-center gap-2.5 min-w-0 flex-1">
          {isLoading && (
            <Loader2 className="h-4 w-4 animate-spin text-slate-400 shrink-0" />
          )}
          <div className="flex items-center gap-2 min-w-0 flex-1">
            {props.priority > 0 && (
              <span className="text-red-500 font-medium text-sm shrink-0">
                {"!".repeat(props.priority)}
              </span>
            )}
            <span
              className={cn(
                "text-sm text-slate-700 line-clamp-1",
                isDone || props.archived ? "line-through text-slate-400" : ""
              )}
            >
              {props.content}
            </span>
            {props.comments.length > 0 && (
              <MessageSquare className="h-3.5 w-3.5 text-slate-400 shrink-0" />
            )}
          </div>
        </div>
        <Badge
          variant="outline"
          className={cn("text-xs shrink-0 ml-2", getStatusColorClasses(props.status))}
        >
          {getStatusLabel(props.status)}
        </Badge>
      </div>
    );
  }

  return (
    <div className="flex items-center justify-between py-3 px-3 rounded-lg border border-slate-200 hover:border-slate-300 bg-white group transition-colors duration-150">
      <div className="flex items-start gap-2 min-w-0">
        {isLoading && (
          <Loader2 className="h-5 w-5 mt-0.5 animate-spin text-slate-400 shrink-0" />
        )}

        <div className="flex flex-col min-w-0">
          <div className="flex items-center gap-2">
            <Link
              to={`/inbox/${props.id}`}
              className={cn(
                "text-sm text-slate-700 no-underline leading-6 truncate hover:text-teal-600 transition-colors duration-150 cursor-pointer",
                isDone || props.archived ? "line-through text-slate-400" : ""
              )}
            >
              {props.priority > 0 && (
                <span className="text-red-500 font-medium pr-1.5">
                  {"!".repeat(props.priority)}
                </span>
              )}
              {props.content}
            </Link>

            {props.comments.length > 0 && (
              <MessageSquare className="h-4 w-4 text-slate-400 shrink-0" />
            )}

            {standDuration > 0 && !isDone && (
              <Badge
                variant="outline"
                className="text-xs text-amber-600 border-amber-200 bg-amber-50 shrink-0"
              >
                {standDuration}d
              </Badge>
            )}
          </div>

          {props.project_id && (() => {
            const project = getProjectById(props.project_id);
            return project ? (
              <div className="mt-1 flex items-center gap-1.5">
                <div
                  className="w-2 h-2 rounded-full"
                  style={{ backgroundColor: project.color }}
                />
                <Badge
                  variant="outline"
                  className="text-[10px] text-slate-400 border-slate-200 font-normal"
                >
                  {project.name}
                </Badge>
              </div>
            ) : null;
          })()}
        </div>
      </div>

      {/* Action buttons - context-aware */}
      <div className="hidden group-hover:flex items-center gap-1 shrink-0">
        {/* Backlog task: Move to Inbox or Start Agile */}
        {isBacklog && (
          <>
            <Button
              variant="ghost"
              size="sm"
              className="h-7 px-2 text-xs text-slate-500 hover:text-teal-600 cursor-pointer"
              onClick={() => handleStatusChange("todo")}
              title="Move to Inbox"
            >
              <Inbox className="h-3.5 w-3.5 mr-1" />
              Inbox
            </Button>
            <Button
              variant="ghost"
              size="sm"
              className="h-7 px-2 text-xs text-slate-500 hover:text-purple-600 cursor-pointer"
              onClick={() => handleStatusChange("plan-pending")}
              title="Start agile workflow"
            >
              <PenLine className="h-3.5 w-3.5 mr-1" />
              Agile
            </Button>
          </>
        )}

        {/* Todo task: Start simple or Start Agile */}
        {isTodo && (
          <>
            <Button
              variant="ghost"
              size="sm"
              className="h-7 px-2 text-xs text-slate-500 hover:text-amber-600 cursor-pointer"
              onClick={() => handleStatusChange("in-progress")}
              title="Start working (simple)"
            >
              <Play className="h-3.5 w-3.5 mr-1" />
              Start
            </Button>
            <Button
              variant="ghost"
              size="sm"
              className="h-7 px-2 text-xs text-slate-500 hover:text-purple-600 cursor-pointer"
              onClick={() => handleStatusChange("plan-pending")}
              title="Start agile workflow"
            >
              <PenLine className="h-3.5 w-3.5 mr-1" />
              Agile
            </Button>
          </>
        )}

        {/* Plan phase actions */}
        {phase === "plan" && (
          <>
            {props.status === "plan-pending" && (
              <Button
                variant="ghost"
                size="sm"
                className="h-7 px-2 text-xs text-slate-500 hover:text-purple-600 cursor-pointer"
                onClick={() => handleStatusChange("plan-in-progress")}
                title="Start planning"
              >
                <PenLine className="h-3.5 w-3.5 mr-1" />
                Plan
              </Button>
            )}
            {props.status === "plan-in-progress" && (
              <Button
                variant="ghost"
                size="sm"
                className="h-7 px-2 text-xs text-slate-500 hover:text-purple-600 cursor-pointer"
                onClick={() => handleStatusChange("plan-review")}
                title="Submit for review"
              >
                <Send className="h-3.5 w-3.5 mr-1" />
                Review
              </Button>
            )}
            {props.status === "plan-review" && (
              <Button
                variant="ghost"
                size="sm"
                className="h-7 px-2 text-xs text-slate-500 hover:text-green-600 cursor-pointer"
                onClick={() => handleStatusChange("plan-done")}
                title="Approve plan"
              >
                <CheckCircle className="h-3.5 w-3.5 mr-1" />
                Approve
              </Button>
            )}
            {props.status === "plan-done" && (
              <Button
                variant="ghost"
                size="sm"
                className="h-7 px-2 text-xs text-slate-500 hover:text-blue-600 cursor-pointer"
                onClick={() => handleStatusChange("coding-pending")}
                title="Start coding phase"
              >
                <Code className="h-3.5 w-3.5 mr-1" />
                Coding
              </Button>
            )}
          </>
        )}

        {/* Coding phase actions */}
        {phase === "coding" && (
          <>
            {(props.status === "coding-pending" || props.status === "in-progress") && (
              <Button
                variant="ghost"
                size="sm"
                className="h-7 px-2 text-xs text-slate-500 hover:text-blue-600 cursor-pointer"
                onClick={() => handleStatusChange("coding-in-progress")}
                title="Start coding"
              >
                <Code className="h-3.5 w-3.5 mr-1" />
                Code
              </Button>
            )}
            {props.status === "coding-in-progress" && (
              <Button
                variant="ghost"
                size="sm"
                className="h-7 px-2 text-xs text-slate-500 hover:text-blue-600 cursor-pointer"
                onClick={() => handleStatusChange("coding-review")}
                title="Submit PR"
              >
                <Send className="h-3.5 w-3.5 mr-1" />
                PR
              </Button>
            )}
            {(props.status === "coding-review" || props.status === "in-review") && (
              <Button
                variant="ghost"
                size="sm"
                className="h-7 px-2 text-xs text-slate-500 hover:text-green-600 cursor-pointer"
                onClick={() => handleStatusChange("coding-done")}
                title="Merge PR"
              >
                <CheckCircle className="h-3.5 w-3.5 mr-1" />
                Merge
              </Button>
            )}
            {props.status === "coding-done" && (
              <Button
                variant="ghost"
                size="sm"
                className="h-7 px-2 text-xs text-slate-500 hover:text-amber-600 cursor-pointer"
                onClick={() => handleStatusChange("cross-review-pending")}
                title="Request cross review"
              >
                <FileSearch className="h-3.5 w-3.5 mr-1" />
                Cross Review
              </Button>
            )}
          </>
        )}

        {/* Cross-review phase actions */}
        {phase === "cross-review" && (
          <>
            {props.status === "cross-review-pending" && (
              <Button
                variant="ghost"
                size="sm"
                className="h-7 px-2 text-xs text-slate-500 hover:text-amber-600 cursor-pointer"
                onClick={() => handleStatusChange("cross-review-in-progress")}
                title="Start review"
              >
                <FileSearch className="h-3.5 w-3.5 mr-1" />
                Review
              </Button>
            )}
            {props.status === "cross-review-in-progress" && (
              <>
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-7 px-2 text-xs text-slate-500 hover:text-green-600 cursor-pointer"
                  onClick={() => handleStatusChange("cross-review-pass")}
                  title="Pass review"
                >
                  <CheckCircle className="h-3.5 w-3.5 mr-1" />
                  Pass
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-7 px-2 text-xs text-slate-500 hover:text-red-600 cursor-pointer"
                  onClick={() => handleStatusChange("cross-review-fail")}
                  title="Fail review"
                >
                  <XCircle className="h-3.5 w-3.5 mr-1" />
                  Fail
                </Button>
              </>
            )}
            {props.status === "cross-review-pass" && (
              <Button
                variant="ghost"
                size="sm"
                className="h-7 px-2 text-xs text-slate-500 hover:text-teal-600 cursor-pointer"
                onClick={() => handleStatusChange("done")}
                title="Complete task"
              >
                <Check className="h-3.5 w-3.5 mr-1" />
                Done
              </Button>
            )}
            {props.status === "cross-review-fail" && (
              <Button
                variant="ghost"
                size="sm"
                className="h-7 px-2 text-xs text-slate-500 hover:text-blue-600 cursor-pointer"
                onClick={() => handleStatusChange("coding-pending")}
                title="Return to coding"
              >
                <RefreshCcw className="h-3.5 w-3.5 mr-1" />
                Rework
              </Button>
            )}
          </>
        )}

        {/* Active task: Execute with agent */}
        {isActive && !isDone && (
          <Button
            variant="ghost"
            size="sm"
            className="h-7 px-2 text-xs text-slate-500 hover:text-violet-600 cursor-pointer"
            onClick={handleExecute}
            title="Execute with agent"
          >
            <Bot className="h-3.5 w-3.5 mr-1" />
            Run
          </Button>
        )}

        {/* Simple flow: Quick done */}
        {(isTodo || props.status === "in-progress" || props.status === "in-review") && (
          <Button
            variant="ghost"
            size="sm"
            className="h-7 px-2 text-xs text-slate-500 hover:text-teal-600 cursor-pointer"
            onClick={() => handleStatusChange("done")}
            title="Mark as done"
          >
            <Check className="h-3.5 w-3.5 mr-1" />
            Done
          </Button>
        )}

        {/* Done task: Archive */}
        {isDone && (
          <Button
            variant="ghost"
            size="sm"
            className="h-7 px-2 text-xs text-slate-500 hover:text-slate-700 cursor-pointer"
            onClick={() => archiveTask(props.id)}
            title="Archive"
          >
            <Archive className="h-3.5 w-3.5 mr-1" />
            Archive
          </Button>
        )}

        {/* More actions menu */}
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button
              variant="ghost"
              size="icon"
              className="h-7 w-7 text-slate-400 hover:text-slate-600 cursor-pointer"
            >
              <MoreHorizontal className="h-4 w-4" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            {/* Move forward to next status */}
            {nextStatus && !isDone && (
              <DropdownMenuItem
                onClick={() => handleStatusChange(nextStatus)}
                className="cursor-pointer"
              >
                <Play className="h-4 w-4 mr-2" />
                {getStatusLabel(nextStatus)}
              </DropdownMenuItem>
            )}
            {/* Active task: Move to Later */}
            {isActive && !isDone && (
              <DropdownMenuItem
                onClick={() => handleStatusChange("backlog")}
                className="cursor-pointer"
              >
                <Clock className="h-4 w-4 mr-2" />
                Move to Later
              </DropdownMenuItem>
            )}
            {isDone && (
              <DropdownMenuItem
                onClick={() => handleStatusChange("todo")}
                className="cursor-pointer"
              >
                <Inbox className="h-4 w-4 mr-2" />
                Reopen
              </DropdownMenuItem>
            )}
            {!isDone && (
              <DropdownMenuItem
                onClick={() => archiveTask(props.id)}
                className="cursor-pointer"
              >
                <Archive className="h-4 w-4 mr-2" />
                Archive anyway
              </DropdownMenuItem>
            )}
            <DropdownMenuSeparator />
            <DropdownMenuItem
              onClick={() => deleteTask(props.id)}
              className="text-red-600 cursor-pointer"
            >
              <Trash2 className="h-4 w-4 mr-2" />
              Delete
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </div>
  );
}
