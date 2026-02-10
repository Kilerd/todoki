import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { cn } from "@/lib/utils";
import dayjs, { Dayjs } from "dayjs";
import { Archive, ChevronDown, Edit, RotateCcw, Trash2 } from "lucide-react";
import { Link } from "react-router-dom";
import type { TaskResponse, TaskEvent, TaskStatus } from "../api/types";
import {
  archiveTask,
  deleteTask,
  unarchiveTask,
  updateTaskStatus,
} from "../hooks/useTasks";

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

type TaskEventBadgeProps = TaskEvent;

const valueConverter = (event: TaskEventBadgeProps) => {
  switch (event.event_type) {
    case "Create":
      return "Created";
    case "StatusChange": {
      const fromLabel = STATUS_LABELS[event.from_state ?? ""] ?? event.from_state;
      const toLabel = STATUS_LABELS[event.state ?? ""] ?? event.state;
      if (fromLabel && toLabel) {
        return `${fromLabel} → ${toLabel}`;
      }
      return `→ ${toLabel}`;
    }
    case "Archived":
      return "Archived";
    case "Unarchived":
      return "Unarchived";
    case "CreateComment":
      return "Comment";
    default:
      return event.event_type;
  }
};

export function TaskEventBadge(props: TaskEventBadgeProps) {
  return (
    <div title={dayjs(props.datetime).format("HH:mm:ss")}>
      <Badge>{valueConverter(props)}</Badge>
    </div>
  );
}

interface Props extends TaskResponse {
  grouped_day?: Dayjs;
}

export default function TimelineTaskItem(props: Props) {
  const handleStatusChange = async (newStatus: TaskStatus) => {
    await updateTaskStatus(props.id, newStatus);
  };

  const openEditModel = () => {
    // TODO: Replace with your modal implementation
  };

  let day_events: TaskEvent[] = [];
  if (props.grouped_day !== undefined) {
    day_events = props.events
      .filter(
        (event) =>
          dayjs(event.datetime).format("MMM D, YYYY") ===
          dayjs(props.grouped_day).format("MMM D, YYYY")
      )
      .reverse();
  }

  const isDone = props.status === "done";

  return (
    <div>
      <div className="flex items-center justify-between p-2 rounded-sm hover:bg-gray-100 group">
        <div className="flex items-center gap-2">
          <span className="text-gray-500 min-w-[5vh] flex items-center gap-1.5">
            <div
              className="w-2 h-2 rounded-full"
              style={{ backgroundColor: props.project?.color ?? "#6B7280" }}
            />
            {props.project?.name ?? "Inbox"}
          </span>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button
                variant="ghost"
                size="sm"
                className={cn("h-6 px-2 text-xs", STATUS_COLORS[props.status])}
                disabled={props.archived}
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

          <Link
            to={`/inbox/${props.id}`}
            className={cn(
              "leading-7",
              isDone || props.archived ? "line-through text-gray-500" : ""
            )}
          >
            {props.priority > 0 && (
              <span className="text-red-600 font-bold pr-2">
                {"!".repeat(props.priority)}
              </span>
            )}
            {props.content}
          </Link>
          {props.archived && <Badge variant="outline">ARCHIVED</Badge>}
        </div>

        <div className="hidden group-hover:flex items-center gap-2">
          {!isDone && !props.archived && (
            <Button variant="ghost" size="icon" onClick={openEditModel}>
              <Edit className="h-4 w-4" />
            </Button>
          )}
          {props.archived ? (
            <Button
              variant="ghost"
              size="icon"
              onClick={() => unarchiveTask(props.id)}
            >
              <RotateCcw className="h-4 w-4" />
            </Button>
          ) : (
            <Button
              variant="ghost"
              size="icon"
              onClick={() => archiveTask(props.id)}
            >
              <Archive className="h-4 w-4" />
            </Button>
          )}
          <Button
            variant="ghost"
            size="icon"
            onClick={() => deleteTask(props.id)}
          >
            <Trash2 className="h-4 w-4" />
          </Button>
        </div>
      </div>
      {day_events.map((event) => (
        <div
          key={event.datetime}
          className="flex items-center gap-2 ml-14 text-gray-500 text-sm py-0.5"
        >
          <span>{dayjs(event.datetime).format("HH:mm:ss")}</span>
          <span>{valueConverter(event)}</span>
        </div>
      ))}
    </div>
  );
}
