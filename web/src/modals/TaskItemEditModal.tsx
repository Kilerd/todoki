import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from "@/components/ui/select";
import { useToast } from "@/hooks/use-toast";
import { DialogProps } from "@radix-ui/react-dialog";
import { useMemo, useState } from "react";
import PreviewTaskItem from "../components/PreviewTaskItem";
import { updateTask } from "../hooks/useTasks";
import { useProjects, getOrCreateProject } from "../hooks/useProjects";
import { parseTask } from "../utils/taskParser";
import type { TaskResponse } from "../api/types";

type Props = TaskResponse;

export default function TaskItemEditModal({ onOpenChange, innerProps: props }: DialogProps & { innerProps: Props }) {
    const { toast } = useToast();
    const { projects } = useProjects();
    const originalContent = (props.priority > 0 ? `${"!".repeat(props.priority)} ` : "") + props.content;
    const [newTaskText, setNewTaskText] = useState(originalContent);
    const [selectedProjectId, setSelectedProjectId] = useState(props.project_id);
    const [isSubmitting, setIsSubmitting] = useState(false);

    const parsedTask = useMemo(() => parseTask(newTaskText), [newTaskText]);
    const selectedProject = projects.find(p => p.id === selectedProjectId);

    const hasChanges = originalContent !== newTaskText || selectedProjectId !== props.project_id;

    const handleSubmitClick = async () => {
        setIsSubmitting(true);
        try {
            let projectId = selectedProjectId;

            // If user typed +tag syntax, handle it
            if (parsedTask.group) {
                const project = await getOrCreateProject(parsedTask.group);
                projectId = project.id;
            }

            await updateTask(props.id, {
                priority: parsedTask.priority,
                content: parsedTask.content,
                project_id: projectId,
            });
            toast({
                title: "Updated",
                description: "Task updated successfully"
            });
            onOpenChange?.(false);
        } finally {
            setIsSubmitting(false);
        }
    };

    return (
        <div className="flex flex-col space-y-4">
            <Input
                value={newTaskText}
                onChange={e => setNewTaskText(e.target.value)}
                placeholder="Task content"
            />

            <div className="space-y-2">
                <label className="text-sm text-slate-500">Project</label>
                <Select value={selectedProjectId} onValueChange={setSelectedProjectId}>
                    <SelectTrigger>
                        <SelectValue>
                            {selectedProject && (
                                <div className="flex items-center gap-2">
                                    <div
                                        className="w-3 h-3 rounded-full"
                                        style={{ backgroundColor: selectedProject.color }}
                                    />
                                    {selectedProject.name}
                                </div>
                            )}
                        </SelectValue>
                    </SelectTrigger>
                    <SelectContent>
                        {projects.map(project => (
                            <SelectItem key={project.id} value={project.id}>
                                <div className="flex items-center gap-2">
                                    <div
                                        className="w-3 h-3 rounded-full"
                                        style={{ backgroundColor: project.color }}
                                    />
                                    {project.name}
                                </div>
                            </SelectItem>
                        ))}
                    </SelectContent>
                </Select>
            </div>

            <PreviewTaskItem
                {...parsedTask}
                id="previewTask"
                project={selectedProject}
                projectName={parsedTask.group}
                archived={false}
                status={props.status}
            />

            <div className="flex justify-end space-x-2">
                <Button
                    variant="outline"
                    disabled={!hasChanges}
                    onClick={() => {
                        setNewTaskText(originalContent);
                        setSelectedProjectId(props.project_id);
                    }}
                >
                    Reset
                </Button>
                <Button
                    disabled={!hasChanges || isSubmitting}
                    onClick={handleSubmitClick}
                >
                    {isSubmitting ? "Saving..." : "Submit"}
                </Button>
            </div>
        </div>
    );
}
