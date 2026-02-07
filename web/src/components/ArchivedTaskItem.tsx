import { useState } from "react";
import { deleteTask, unarchiveTask } from "../hooks/useTasks";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Checkbox } from "@/components/ui/checkbox";
import { RotateCcw, Trash2 } from "lucide-react";
import type { TaskResponse } from "../api/schema";

type Props = TaskResponse;

export default function ArchivedTaskItem(props: Props) {
  const [checked] = useState(props.done);

  return (
    <div className="flex items-center justify-between p-2 rounded-md text-gray-600 hover:bg-gray-100 group">
      <div className="flex items-center gap-2">
        <span className="text-gray-500">{props.group}</span>
        {props.task_type === "Todo" && <Checkbox disabled checked={checked} />}
        {props.task_type === "Stateful" && (
          <Badge variant="secondary">{props.current_state}</Badge>
        )}

        <div className="leading-7">
          {props.priority > 0 && (
            <span className="text-red-600 font-bold pr-2">
              {"!".repeat(props.priority)}
            </span>
          )}
          {props.content}
        </div>
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
