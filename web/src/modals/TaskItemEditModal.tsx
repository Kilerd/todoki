import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useToast } from "@/hooks/use-toast";
import { DialogProps } from "@radix-ui/react-dialog";
import { useMemo, useState } from "react";
import PreviewTaskItem from "../components/PreviewTaskItem";
import { updateTask } from "../hooks/useTasks";
import { parseTask } from "../utils/taskParser";
import type { TaskResponse } from "../api/schema";

type Props = TaskResponse;

export default function TaskItemEditModal({ onOpenChange, innerProps: props }: DialogProps & { innerProps: Props }) {
    const { toast } = useToast();
    const originalContent = (props.priority > 0 ? `${"!".repeat(props.priority)} ` : "") + props.content + (props.group !== "default" ? ` +${props.group}` : "")

    const [newTaskText, setNewTaskText] = useState(originalContent);

    const parsedTask = useMemo(() => parseTask(newTaskText), [newTaskText])

    const handleSubmitClick = async () => {
        await updateTask(props.id, {
            priority: parsedTask.priority,
            content: parsedTask.content,
            group: parsedTask.group ?? null,
        });
        toast({
            title: "Updated",
            description: "Task updated successfully"
        });
        onOpenChange?.(false);
    }

    return (
        <div className="flex flex-col space-y-4">
            <Input value={newTaskText} onChange={e => setNewTaskText(e.target.value)} />

            <PreviewTaskItem {...parsedTask}
                id={"previewTask"}
                archived={false}
                status={props.status}
            />

            <div className="flex justify-end space-x-2">
                <Button
                    variant="outline"
                    disabled={originalContent === newTaskText}
                    onClick={() => setNewTaskText(originalContent)}
                >
                    Reset
                </Button>
                <Button
                    disabled={originalContent === newTaskText}
                    onClick={handleSubmitClick}
                >
                    Submit
                </Button>
            </div>
        </div>
    )
}
