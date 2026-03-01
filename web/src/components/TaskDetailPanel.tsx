import { useMemo } from "react";
import { useSearchParams } from "react-router-dom";
import { X, Calendar, Tag, User, Clock } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { EventTimeline } from "./EventTimeline";
import { useTasks } from "../hooks/useTasks";
import { useProjects } from "../hooks/useProjects";
import { format } from "date-fns";

export default function TaskDetailPanel() {
  const [searchParams, setSearchParams] = useSearchParams();
  const selectedTaskId = searchParams.get("task");
  const { tasks } = useTasks();
  const { projects } = useProjects();

  const task = useMemo(
    () => tasks.find((t) => t.id === selectedTaskId),
    [tasks, selectedTaskId]
  );

  const project = useMemo(
    () => projects.find((p) => p.id === task?.project_id),
    [projects, task]
  );

  const handleClose = () => {
    setSearchParams({});
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

      {/* Event Timeline */}
      <div className="flex-1 overflow-hidden flex flex-col">
        <div className="p-4 border-b border-slate-100">
          <h3 className="text-sm font-semibold text-slate-700">Activity</h3>
          <p className="text-xs text-slate-500 mt-0.5">
            Real-time event stream for this task
          </p>
        </div>
        <div className="flex-1 overflow-y-auto p-4">
          <EventTimeline
            kinds={["task.*", "agent.*", "artifact.*"]}
            taskId={selectedTaskId}
            autoScroll
          />
        </div>
      </div>
    </div>
  );
}
