import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { cn } from "@/lib/utils";
import dayjs from "dayjs";
import { Archive, ChevronDown, Loader2, MessageSquare, Trash2 } from "lucide-react";
import { useState } from "react";
import { Link } from "react-router-dom";
import {
  archiveTask,
  deleteTask,
  updateTaskStatus,
} from "../hooks/useTasks";
import type { TaskResponse, TaskStatus } from "../api/schema";

type Props = TaskResponse;

const STATUS_LABELS: Record<string, string> = {
  backlog: "Backlog",
  todo: "Todo",
  "in-progress": "In Progress",
  "in-review": "In Review",
  done: "Done",
};

const STATUS_ORDER: TaskStatus[] = ["backlog", "todo", "in-progress", "in-review", "done"];

const STATUS_COLORS: Record<string, string> = {
  backlog: "bg-gray-100 text-gray-700",
  todo: "bg-blue-100 text-blue-700",
  "in-progress": "bg-yellow-100 text-yellow-700",
  "in-review": "bg-purple-100 text-purple-700",
  done: "bg-green-100 text-green-700",
};

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

  return (
    <div className="flex items-center justify-between py-1 px-2 rounded-lg hover:bg-gray-50 group">
      <div className="flex items-center gap-2 py-1">
        {isLoading ? (
          <Loader2 className="h-5 w-5 animate-spin" />
        ) : (
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button
                variant="ghost"
                size="sm"
                className={cn("h-6 px-2 text-xs", STATUS_COLORS[props.status])}
              >
                {STATUS_LABELS[props.status]}
                <ChevronDown className="ml-1 h-3 w-3" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="start">
              {STATUS_ORDER.map((status) => (
                <DropdownMenuItem
                  key={status}
                  onClick={() => handleStatusChange(status)}
                  disabled={status === props.status}
                >
                  <span className={cn("px-2 py-0.5 rounded text-xs", STATUS_COLORS[status])}>
                    {STATUS_LABELS[status]}
                  </span>
                </DropdownMenuItem>
              ))}
            </DropdownMenuContent>
          </DropdownMenu>
        )}
        <Badge variant="outline">{props.group}</Badge>
        <Link
          to={`/tasks/${props.id}`}
          className={cn(
            "text-gray-900 no-underline leading-7",
            isDone || props.archived ? "line-through text-gray-500" : ""
          )}
        >
          {props.priority > 0 && (
            <span className="text-red-900 font-bold pr-2">
              {"!".repeat(props.priority)}
            </span>
          )}
          {props.content}
        </Link>
        {props.comments.length > 0 && (
          <MessageSquare className="h-5 w-5 text-gray-500" />
        )}
        {standDuration > 0 && !isDone && (
          <Badge variant="destructive">
            {standDuration} day{standDuration > 1 ? "s" : ""}
          </Badge>
        )}
      </div>

      <div className="hidden group-hover:flex items-center">
        <Button
          variant="ghost"
          size="icon"
          onClick={() => archiveTask(props.id)}
        >
          <Archive className="h-4 w-4" />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          className="text-pink-600"
          onClick={() => deleteTask(props.id)}
        >
          <Trash2 className="h-4 w-4" />
        </Button>
      </div>
    </div>
  );
}
