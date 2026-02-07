import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { useOs } from "@mantine/hooks";
import { orderBy } from "lodash";
import { ArrowRight, Eye } from "lucide-react";
import { useMemo, useState } from "react";
import NavBar from "../components/NavBar";
import PreviewTaskItem from "../components/PreviewTaskItem";
import TaskItem from "../components/TaskItem";
import { useBacklogTasks, createTask } from "../hooks/useTasks";
import { parseTask } from "../utils/taskParser";

function Kbd({ children }: { children: React.ReactNode }) {
  return (
    <kbd className="pointer-events-none inline-flex h-5 select-none items-center gap-1 rounded border bg-muted px-1.5 font-mono text-[10px] font-medium text-muted-foreground opacity-100">
      {children}
    </kbd>
  );
}

function Backlog() {
  const os = useOs();
  const { tasks, isLoading } = useBacklogTasks();
  const [newTaskText, setNewTaskText] = useState("");

  const parsedTask = useMemo(() => parseTask(newTaskText), [newTaskText]);

  const backlogTasks = useMemo(() => {
    return tasks.filter((item) => item.archived === false);
  }, [tasks]);

  const handleNewTask = async () => {
    await createTask({
      ...parsedTask,
      group: parsedTask.group ?? null,
      status: "backlog",
    });
    setNewTaskText("");
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
      handleNewTask();
    }
  };

  return (
    <div className="container mx-auto mt-12">
      <NavBar />

      <div className="mt-4">
        <div className="flex gap-2">
          <Input
            value={newTaskText}
            onChange={(e) => setNewTaskText(e.target.value)}
            placeholder="Add a new task to backlog"
            onKeyDown={handleKeyDown}
          />
          <Button
            disabled={newTaskText.trim() === ""}
            onClick={handleNewTask}
            size="icon"
          >
            <ArrowRight className="h-4 w-4" />
          </Button>
        </div>

        <div className="mt-4 px-6 py-4 bg-gray-100 rounded">
          <div className="space-y-4">
            <Accordion type="single" collapsible>
              <AccordionItem value="preview">
                <AccordionTrigger>
                  <div className="flex items-center gap-2">
                    <Eye className="h-5 w-5" />
                    Preview
                  </div>
                </AccordionTrigger>
                <AccordionContent>
                  {newTaskText.trim() !== "" && (
                    <PreviewTaskItem
                      {...parsedTask}
                      id={"previewTask"}
                      archived={false}
                      status="backlog"
                    />
                  )}
                </AccordionContent>
              </AccordionItem>
            </Accordion>

            <div className="flex flex-wrap gap-4">
              <div className="flex items-center gap-2">
                Group: <Kbd>+Group</Kbd>
              </div>
              <div className="flex items-center gap-2">
                Priority: <Kbd>!!!</Kbd>
              </div>
              <div className="flex items-center gap-2">
                Create:{" "}
                <Kbd>{os === "macos" ? "Command" : "Control"} + Enter</Kbd>
              </div>
            </div>
          </div>
        </div>
      </div>

      {isLoading ? (
        <div className="space-y-4 mt-4">
          <Skeleton className="h-9 w-full" />
          <Skeleton className="h-9 w-full" />
          <Skeleton className="h-9 w-full" />
          <Skeleton className="h-9 w-full" />
        </div>
      ) : (
        <div className="space-y-2 mt-4">
          {orderBy(backlogTasks, ["priority", "create_at"], ["desc", "asc"]).map(
            (task) => (
              <TaskItem key={task.id} {...task}></TaskItem>
            )
          )}
          {backlogTasks.length === 0 && (
            <div className="text-center text-gray-500 py-8">
              No tasks in backlog
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export default Backlog;
