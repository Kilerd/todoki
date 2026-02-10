import { deleteTask, unarchiveTask } from "../hooks/useTasks";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { RotateCcw, Trash2 } from "lucide-react";
import type { TaskResponse } from "../api/types";
import { cn } from "@/lib/utils";

type Props = TaskResponse;

const STATUS_LABELS: Record<string, string> = {
  backlog: "Backlog",
  todo: "Todo",
  "in-progress": "In Progress",
  "in-review": "In Review",
  done: "Done",
};

const STATUS_COLORS: Record<string, string> = {
  backlog: "bg-gray-100 text-gray-700",
  todo: "bg-blue-100 text-blue-700",
  "in-progress": "bg-yellow-100 text-yellow-700",
  "in-review": "bg-purple-100 text-purple-700",
  done: "bg-green-100 text-green-700",
};

export default function ArchivedTaskItem(props: Props) {
  return (
    <div className="flex items-center justify-between p-2 rounded-md text-gray-600 hover:bg-gray-100 group">
      <div className="flex items-center gap-2">
        <span className="text-gray-500 flex items-center gap-1.5">
          <div
            className="w-2 h-2 rounded-full"
            style={{ backgroundColor: props.project?.color ?? "#6B7280" }}
          />
          {props.project?.name ?? "Inbox"}
        </span>
        <span className={cn("px-2 py-0.5 rounded text-xs opacity-60", STATUS_COLORS[props.status])}>
          {STATUS_LABELS[props.status]}
        </span>

        <div className="leading-7 line-through">
          {props.priority > 0 && (
            <span className="text-red-600 font-bold pr-2">
              {"!".repeat(props.priority)}
            </span>
          )}
          {props.content}
        </div>
        <Badge variant="outline">ARCHIVED</Badge>
      </div>
      <div className="hidden group-hover:flex items-center gap-2">
        <Button
          variant="ghost"
          size="icon"
          onClick={() => unarchiveTask(props.id)}
        >
          <RotateCcw className="h-4 w-4" />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          onClick={() => deleteTask(props.id)}
          className="text-pink-600 hover:text-pink-700"
        >
          <Trash2 className="h-4 w-4" />
        </Button>
      </div>
    </div>
  );
}
