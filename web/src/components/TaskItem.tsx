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
import dayjs from "dayjs";
import {
  Archive,
  Check,
  Clock,
  Inbox,
  Loader2,
  MessageSquare,
  MoreHorizontal,
  Play,
  Trash2,
} from "lucide-react";
import { useState } from "react";
import { Link } from "react-router-dom";
import {
  archiveTask,
  deleteTask,
  updateTaskStatus,
} from "../hooks/useTasks";
import type { TaskResponse, TaskStatus } from "../api/schema";

type Props = TaskResponse;

export default function TaskItem(props: Props) {
  const [isLoading, setIsLoading] = useState(false);

  const handleStatusChange = async (newStatus: TaskStatus) => {
    setIsLoading(true);
    await updateTaskStatus(props.id, newStatus);
    setIsLoading(false);
  };

  const standDuration = Math.trunc(
    (dayjs().unix() - dayjs(props.events[0]?.datetime).unix()) / 86400
  );

  const isDone = props.status === "done";
  const isBacklog = props.status === "backlog";
  const isTodo = props.status === "todo";
  const isWorking =
    props.status === "in-progress" || props.status === "in-review";
  const isActive = isTodo || isWorking;

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

          {props.group && (
            <div className="mt-1">
              <Badge
                variant="outline"
                className="text-[10px] text-slate-400 border-slate-200 font-normal"
              >
                {props.group}
              </Badge>
            </div>
          )}
        </div>
      </div>

      {/* Action buttons - context-aware */}
      <div className="hidden group-hover:flex items-center gap-1 shrink-0">
        {/* Backlog task: Move to Inbox */}
        {isBacklog && (
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
        )}

        {/* Todo task: Start working */}
        {isTodo && (
          <Button
            variant="ghost"
            size="sm"
            className="h-7 px-2 text-xs text-slate-500 hover:text-amber-600 cursor-pointer"
            onClick={() => handleStatusChange("in-progress")}
            title="Start working"
          >
            <Play className="h-3.5 w-3.5 mr-1" />
            Start
          </Button>
        )}

        {/* Active task: Mark as done */}
        {isActive && (
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
            {/* Active task: Move to Later */}
            {isActive && (
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
