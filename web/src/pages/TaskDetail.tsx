import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { useToast } from "@/hooks/use-toast";
import { cn } from "@/lib/utils";
import dayjs from "dayjs";
import {
  Archive,
  ArrowLeft,
  Check,
  Clock,
  Inbox,
  RotateCcw,
  Send,
  Trash2,
} from "lucide-react";
import { useState } from "react";
import ReactMarkdown from "react-markdown";
import { useNavigate, useParams } from "react-router-dom";
import {
  useTask,
  addComment,
  updateTaskStatus,
  archiveTask,
  unarchiveTask,
  deleteTask,
} from "../hooks/useTasks";
import type { TaskEventResponse, TaskStatus } from "../api/schema";

const STATUS_CONFIG: Record<
  string,
  { label: string; color: string }
> = {
  backlog: { label: "Later", color: "bg-slate-100 text-slate-600" },
  todo: { label: "Todo", color: "bg-blue-50 text-blue-600" },
  "in-progress": { label: "In Progress", color: "bg-amber-50 text-amber-600" },
  "in-review": { label: "In Review", color: "bg-purple-50 text-purple-600" },
  done: { label: "Done", color: "bg-teal-50 text-teal-600" },
};

function formatEvent(event: TaskEventResponse): string {
  switch (event.event_type) {
    case "Create":
      return "Created";
    case "StatusChange": {
      const fromLabel = STATUS_CONFIG[event.from_state ?? ""]?.label ?? event.from_state;
      const toLabel = STATUS_CONFIG[event.state ?? ""]?.label ?? event.state;
      if (fromLabel && toLabel) {
        return `${fromLabel} → ${toLabel}`;
      }
      return `→ ${toLabel}`;
    }
    case "Archived":
      return "Archived";
    case "Unarchived":
      return "Restored";
    case "CreateComment":
      return "Commented";
    default:
      return event.event_type;
  }
}

export default function TaskDetail() {
  const { id } = useParams();
  const navigate = useNavigate();
  const [comment, setComment] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const { toast } = useToast();
  const { task, isLoading, refresh } = useTask(id!);

  const onSubmitComment = async () => {
    if (comment.trim() === "") return;
    setIsSubmitting(true);
    await addComment(id!, comment);
    toast({ title: "Comment added" });
    setComment("");
    setIsSubmitting(false);
    refresh();
  };

  const handleStatusChange = async (status: TaskStatus) => {
    await updateTaskStatus(id!, status);
    refresh();
  };

  const handleArchive = async () => {
    await archiveTask(id!);
    refresh();
  };

  const handleUnarchive = async () => {
    await unarchiveTask(id!);
    refresh();
  };

  const handleDelete = async () => {
    await deleteTask(id!);
    navigate("/inbox");
  };

  if (isLoading || !task) {
    return (
      <div className="container mx-auto mt-12 max-w-3xl">
        <div className="animate-pulse space-y-4">
          <div className="h-8 w-32 bg-slate-200 rounded" />
          <div className="h-12 w-full bg-slate-200 rounded" />
          <div className="h-64 w-full bg-slate-200 rounded" />
        </div>
      </div>
    );
  }

  const isDone = task.status === "done";
  const isBacklog = task.status === "backlog";
  const isActive =
    task.status === "todo" ||
    task.status === "in-progress" ||
    task.status === "in-review";

  const statusConfig = STATUS_CONFIG[task.status];

  return (
    <div className="container mx-auto mt-12 max-w-3xl pb-12">
      {/* Back button */}
      <button
        onClick={() => navigate(-1)}
        className="flex items-center gap-2 text-sm text-slate-500 hover:text-slate-700 transition-colors duration-150 cursor-pointer mb-6"
      >
        <ArrowLeft className="h-4 w-4" />
        Back
      </button>

      {/* Task Header */}
      <div className="space-y-4">
        {/* Status & Group badges */}
        <div className="flex items-center gap-2">
          <span
            className={cn(
              "inline-flex items-center px-2.5 py-1 text-xs font-medium rounded",
              statusConfig.color
            )}
          >
            {statusConfig.label}
          </span>
          {task.group && (
            <Badge variant="outline" className="text-xs text-slate-500">
              {task.group}
            </Badge>
          )}
          {task.archived && (
            <Badge variant="outline" className="text-xs text-orange-500 border-orange-200">
              Archived
            </Badge>
          )}
        </div>

        {/* Task content */}
        <h1
          className={cn(
            "text-2xl font-medium text-slate-800",
            (isDone || task.archived) && "line-through text-slate-400"
          )}
        >
          {task.priority > 0 && (
            <span className="text-red-500 mr-2">
              {"!".repeat(task.priority)}
            </span>
          )}
          {task.content}
        </h1>

        {/* Meta info */}
        <div className="flex items-center gap-4 text-xs text-slate-400">
          <span>Created {dayjs(task.create_at).format("MMM D, YYYY")}</span>
          {task.comments.length > 0 && (
            <span>{task.comments.length} comments</span>
          )}
        </div>
      </div>

      {/* Actions */}
      <div className="flex items-center gap-2 mt-6 pt-6 border-t border-slate-100">
        {isBacklog && !task.archived && (
          <Button
            variant="outline"
            size="sm"
            onClick={() => handleStatusChange("todo")}
            className="cursor-pointer"
          >
            <Inbox className="h-4 w-4 mr-1.5" />
            Move to Inbox
          </Button>
        )}

        {isActive && !task.archived && (
          <>
            <Button
              variant="outline"
              size="sm"
              onClick={() => handleStatusChange("backlog")}
              className="cursor-pointer"
            >
              <Clock className="h-4 w-4 mr-1.5" />
              Later
            </Button>
            <Button
              size="sm"
              onClick={() => handleStatusChange("done")}
              className="bg-teal-600 hover:bg-teal-700 cursor-pointer"
            >
              <Check className="h-4 w-4 mr-1.5" />
              Done
            </Button>
          </>
        )}

        {isDone && !task.archived && (
          <>
            <Button
              variant="outline"
              size="sm"
              onClick={() => handleStatusChange("todo")}
              className="cursor-pointer"
            >
              <RotateCcw className="h-4 w-4 mr-1.5" />
              Reopen
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={handleArchive}
              className="cursor-pointer"
            >
              <Archive className="h-4 w-4 mr-1.5" />
              Archive
            </Button>
          </>
        )}

        {task.archived && (
          <Button
            variant="outline"
            size="sm"
            onClick={handleUnarchive}
            className="cursor-pointer"
          >
            <RotateCcw className="h-4 w-4 mr-1.5" />
            Restore
          </Button>
        )}

        <Button
          variant="ghost"
          size="sm"
          onClick={handleDelete}
          className="text-red-500 hover:text-red-600 hover:bg-red-50 ml-auto cursor-pointer"
        >
          <Trash2 className="h-4 w-4 mr-1.5" />
          Delete
        </Button>
      </div>

      {/* Activity Timeline */}
      <div className="mt-8">
        <h2 className="text-xs font-medium text-slate-400 uppercase tracking-wider mb-4">
          Activity
        </h2>
        <div className="space-y-2">
          {[...task.events].reverse().map((event, idx) => (
            <div
              key={event.id || idx}
              className="flex items-center gap-3 text-sm"
            >
              <span className="text-xs text-slate-400 font-mono w-32 shrink-0">
                {dayjs(event.datetime).format("MMM D, HH:mm")}
              </span>
              <span className="text-slate-600">{formatEvent(event)}</span>
            </div>
          ))}
        </div>
      </div>

      {/* Comments */}
      <div className="mt-8">
        <h2 className="text-xs font-medium text-slate-400 uppercase tracking-wider mb-4">
          Comments ({task.comments.length})
        </h2>

        {task.comments.length === 0 ? (
          <p className="text-sm text-slate-400 py-4">No comments yet</p>
        ) : (
          <div className="space-y-4">
            {task.comments.map((c) => (
              <div
                key={c.id}
                className="border border-slate-200 rounded-lg p-4 bg-white"
              >
                <div className="text-xs text-slate-400 mb-2">
                  {dayjs(c.create_at).format("MMM D, YYYY HH:mm")}
                </div>
                <div className="prose prose-sm prose-slate max-w-none">
                  <ReactMarkdown>{c.content}</ReactMarkdown>
                </div>
              </div>
            ))}
          </div>
        )}

        {/* New comment */}
        <div className="mt-6 border border-slate-200 rounded-lg p-4 bg-white">
          <Textarea
            placeholder="Write a comment..."
            value={comment}
            onChange={(e) => setComment(e.target.value)}
            className="border-0 p-0 focus-visible:ring-0 resize-none min-h-[80px]"
          />
          <div className="flex justify-end mt-3">
            <Button
              size="sm"
              disabled={comment.trim() === "" || isSubmitting}
              onClick={onSubmitComment}
              className="bg-teal-600 hover:bg-teal-700 cursor-pointer"
            >
              <Send className="h-4 w-4 mr-1.5" />
              Send
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
