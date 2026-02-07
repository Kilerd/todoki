import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { cn } from "@/lib/utils";
import dayjs from "dayjs";
import { Archive, Edit, Loader2, MessageSquare, Trash2 } from "lucide-react";
import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import {
  archiveTask,
  deleteTask,
  updateTaskStatus,
} from "../hooks/useTasks";
import type { TaskResponse } from "../api/schema";

type Props = TaskResponse;

export default function TaskItem(props: Props) {
  const [checked, setChecked] = useState(false);
  const [isLoading, setIsLoading] = useState(false);

  useEffect(() => {
    setChecked(props.done);
  }, [props]);

  const handleClick = async (checked: boolean) => {
    setIsLoading(true);
    setChecked(checked);
    const status = checked ? "Done" : "Open";
    await updateTaskStatus(props.id, status);
    setIsLoading(false);
  };

  const handleUpdateState = async (state: string) => {
    setIsLoading(true);
    await updateTaskStatus(props.id, state);
    setIsLoading(false);
  };

  const openEditModel = () => {
    // TODO: implement edit modal
  };

  const current_index = props.states?.findIndex(
    (it) => it === props.current_state
  );
  const prevState =
    current_index !== undefined ? props.states?.[current_index - 1] : undefined;
  const nextState =
    current_index !== undefined ? props.states?.[current_index + 1] : undefined;
  const standDuration = Math.trunc(
    (dayjs().unix() - dayjs(props.events[0]?.datetime).unix()) / 86400
  );

  return (
    <div className="flex items-center justify-between py-1 px-2 rounded-lg hover:bg-gray-50 group">
      <div className="flex items-center gap-2 py-1">
        {isLoading ? (
          <Loader2 className="h-5 w-5 animate-spin" />
        ) : (
          <>
            {props.task_type === "Todo" && (
              <Checkbox
                className="h-5 w-5 bg-gray-100 "
                checked={checked}
                onCheckedChange={handleClick}
              />
            )}
            {props.task_type === "Stateful" && (
              <Badge>{props.current_state}</Badge>
            )}
          </>
        )}
        <Badge variant="outline">{props.group}</Badge>
        <Link
          to={`/tasks/${props.id}`}
          className={cn(
            "text-gray-900 no-underline leading-7",
            props.done || props.archived ? "line-through text-gray-500" : ""
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
        {standDuration > 0 && (
          <Badge variant="destructive">任务持续超过 {standDuration} 天</Badge>
        )}
      </div>

      <div className="hidden group-hover:flex items-center">
        {prevState !== undefined && (
          <Button
            variant="ghost"
            size="sm"
            onClick={() => handleUpdateState(prevState)}
          >
            Back to {prevState}
          </Button>
        )}
        {nextState !== undefined && (
          <Button size="sm" onClick={() => handleUpdateState(nextState)}>
            Goto {nextState}
          </Button>
        )}

        {!props.done && !props.archived && (
          <Button variant="ghost" size="icon" onClick={openEditModel}>
            <Edit className="h-4 w-4" />
          </Button>
        )}
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
