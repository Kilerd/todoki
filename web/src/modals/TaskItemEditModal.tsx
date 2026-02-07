import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useToast } from "@/hooks/use-toast";
import { DialogProps } from "@radix-ui/react-dialog";
import { useMemo, useState } from "react";
import PreviewTaskItem from "../components/PreviewTaskItem";
import { updateTask } from "../services/api";
import { parseTask } from "../utils/taskParser";

interface Props {
    id: string,
    priority: number,
    content: string,
    group: string,
    task_type: "Todo" | "Stateful",
    create_at: string,
    events: any[],
    done: boolean,
    archived: boolean,
    current_state: string,
    states: string[],
    habit_id?: string,
    habit_name?: string
}

export default function TaskItemEditModal({ onOpenChange, innerProps: props }: DialogProps & { innerProps: Props }) {
    const { toast } = useToast();
    const originalContent = (props.priority > 0 ? `${"!".repeat(props.priority)} ` : "") + props.content + (props.states ? ` [[${props.states.join(">")}]]` : "") + (props.group !== "default" ? ` +${props.group}` : "")

    const [newTaskText, setNewTaskText] = useState(originalContent);

    const parsedTask = useMemo(() => parseTask(newTaskText), [newTaskText])

    const handleSubmitClick = async () => {
        await updateTask(props.id, parsedTask.task_type, parsedTask.priority, parsedTask.content, parsedTask.group, parsedTask.states)
        toast({
            title: "更改成功",
            description: "更改成功"
        });
        onOpenChange?.(false);
    }

    return (
        <div className="flex flex-col space-y-4">
            <Input value={newTaskText} onChange={e => setNewTaskText(e.target.value)} />
            
            <PreviewTaskItem {...parsedTask} 
                id={"previewTask"} 
                done={false}
                archived={false}
                current_state={parsedTask.states?.[0]} 
            />

            <div className="flex justify-end space-x-2">
                <Button
                    variant="outline"
                    disabled={originalContent === newTaskText}
                    onClick={() => setNewTaskText(originalContent)}
                >
                    重置
                </Button>
                <Button
                    disabled={originalContent === newTaskText}
                    onClick={handleSubmitClick}
                >
                    提交更改
                </Button>
            </div>
        </div>
    )
}