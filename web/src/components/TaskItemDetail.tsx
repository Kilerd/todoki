import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import dayjs, { Dayjs } from "dayjs";
import { Archive, Edit, RotateCcw, Trash2 } from "lucide-react";
import { useState } from "react";
import { Link } from "react-router-dom";
import type { TaskResponse, TaskEventResponse } from "../api/schema";
import { eventTypeConverter } from "../pages/TaskDetail";
import {
  archiveTask,
  deleteTask,
  unarchiveTask,
  updateTaskStatus,
} from "../hooks/useTasks";

interface Props extends TaskResponse {
  grouped_day?: Dayjs;
}

export default function TaskItemDetail(props: Props) {
  const [checked, setChecked] = useState(props.done);

  const handleClick = async (newChecked: boolean) => {
    setChecked(newChecked);
    const status = newChecked ? "Done" : "Open";
    await updateTaskStatus(props.id, status);
  };

  const handleUpdateState = async (state: string) => {
    await updateTaskStatus(props.id, state);
  };

  const openEditModel = () => {
    // TODO: Replace with your modal implementation
  };

  const current_index = (props.states ?? []).findIndex(
    (it) => it === props.current_state
  );
  const prevState = props.states?.[current_index - 1];
  const nextState = props.states?.[current_index + 1];
  let day_events: TaskEventResponse[] = [];
  if (props.grouped_day !== undefined) {
    day_events = props.events
      .filter(
        (event) =>
          dayjs(event.datetime).format("MMM D, YYYY") ===
          dayjs(props.grouped_day).format("MMM D, YYYY")
      )
      .reverse();
  }

  return (
    <div>
      <div className="flex items-center justify-between p-2 rounded-sm hover:bg-gray-100 group">
        <div className="flex items-center gap-2">
          <span className="text-gray-500 min-w-[5vh]">{props.group}</span>
          {props.task_type === "Todo" && (
            <Checkbox
              disabled={props.archived}
              checked={checked}
              onClick={() => handleClick(!checked)}
            />
          )}
          {props.task_type === "Stateful" && (
            <Badge variant="secondary">{props.current_state}</Badge>
          )}

          <Link
            to={`/tasks/${props.id}`}
            className={`leading-7 ${
              props.done || props.archived ? "line-through text-gray-500" : ""
            }`}
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
          {!props.archived && prevState !== undefined && (
            <Button
              variant="secondary"
              size="sm"
              onClick={() => handleUpdateState(prevState)}
            >
              Back to {prevState}
            </Button>
          )}
          {!props.archived && nextState !== undefined && (
            <Button size="sm" onClick={() => handleUpdateState(nextState)}>
              Goto {nextState}
            </Button>
          )}
          {!props.done && !props.archived && (
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
          <span>{eventTypeConverter(event)}</span>
        </div>
      ))}
    </div>
  );
}
