import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardFooter,
  CardHeader,
} from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { Textarea } from "@/components/ui/textarea";
import { useToast } from "@/hooks/use-toast";
import dayjs from "dayjs";
import { useState } from "react";
import ReactMarkdown from "react-markdown";
import { useParams } from "react-router-dom";
import NavBar from "../components/NavBar";
import TaskItemDetail from "../components/TaskItemDetail";
import { useTask, addComment } from "../hooks/useTasks";
import type { TaskEventResponse } from "../api/schema";

const STATUS_LABELS: Record<string, string> = {
  backlog: "Backlog",
  todo: "Todo",
  "in-progress": "In Progress",
  "in-review": "In Review",
  done: "Done",
};

export const eventTypeConverter = (event: TaskEventResponse) => {
  switch (event.event_type) {
    case "Create":
      return "Created";
    case "StatusChange":
      return `Status changed to: ${STATUS_LABELS[event.state ?? ""] ?? event.state}`;
    case "Archived":
      return "Archived";
    case "Unarchived":
      return "Unarchived";
    case "CreateComment":
      return "Comment added";
    default:
      return event.event_type;
  }
};

export default function TaskDetail() {
  const { id } = useParams();
  const [comment, setComment] = useState("");
  const { toast } = useToast();
  const { task, isLoading, refresh } = useTask(id!);

  const onSubmit = async () => {
    await addComment(id!, comment);
    toast({
      title: "Comment submitted",
      description: "Comment submitted successfully",
    });
    setComment("");
    refresh();
  };

  return (
    <div className="container mt-12">
      <NavBar />

      {!isLoading && task && (
        <div className="m-8">
          <div className="text-xl">
            <TaskItemDetail {...task} />
          </div>

          <div className="grid grid-cols-12 gap-4 mt-8">
            <div className="col-span-9">
              {task.comments.length === 0 ? (
                <div className="flex justify-center items-center my-8">
                  <p className="text-sm text-muted-foreground">No Comments</p>
                </div>
              ) : (
                task.comments.map((comment) => (
                  <Card key={comment.id} className="my-2">
                    <CardHeader>
                      <p className="text-sm text-muted-foreground">
                        {dayjs(comment.create_at).format("YYYY-MM-DD HH:mm:ss")}
                      </p>
                    </CardHeader>
                    <CardContent>
                      <ReactMarkdown>{comment.content}</ReactMarkdown>
                    </CardContent>
                  </Card>
                ))
              )}

              <Separator className="my-4" />

              <Card>
                <CardHeader>
                  <p className="text-sm text-muted-foreground">New Comment</p>
                </CardHeader>
                <CardContent>
                  <Textarea
                    placeholder="Your comment"
                    value={comment}
                    onChange={(e) => setComment(e.target.value)}
                  />
                </CardContent>
                <CardFooter className="flex justify-end">
                  <Button disabled={comment.trim() === ""} onClick={onSubmit}>
                    Submit
                  </Button>
                </CardFooter>
              </Card>
            </div>

            <div className="col-span-3">
              <div className="space-y-4">
                {task.events.map((event, index) => (
                  <div
                    key={index}
                    className="relative pl-6 pb-4 border-l-2 border-primary last:border-l-0"
                  >
                    <div className="absolute left-[-5px] w-2.5 h-2.5 rounded-full bg-primary" />
                    <h4 className="font-medium">{eventTypeConverter(event)}</h4>
                    <p className="text-xs text-muted-foreground mt-1">
                      {dayjs(event.datetime).format("YYYY-MM-DD HH:mm:ss")}
                    </p>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
