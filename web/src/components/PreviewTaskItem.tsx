import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import type { Project } from "@/api/types";

interface Props {
    id: string;
    priority: number;
    content: string;
    project?: Project | null;
    projectName?: string;
    archived: boolean;
    status: string;
}

const STATUS_LABELS: Record<string, string> = {
    backlog: "Backlog",
    todo: "Todo",
    "in-progress": "In Progress",
    "in-review": "In Review",
    done: "Done",
};

const STATUS_COLORS: Record<string, string> = {
    backlog: "bg-gray-100 text-gray-700",
    todo: "bg-blue-100 text-blue-700",
    "in-progress": "bg-yellow-100 text-yellow-700",
    "in-review": "bg-purple-100 text-purple-700",
    done: "bg-green-100 text-green-700",
};

export default function PreviewTaskItem(props: Props) {
    const projectName = props.project?.name ?? props.projectName ?? "Inbox";
    const projectColor = props.project?.color ?? "#6B7280";
    const isDone = props.status === "done";

    return (
        <div className="flex items-center justify-between p-2 rounded-md hover:bg-gray-100 group">
            <div className="flex items-center gap-2">
                <div className="flex items-center gap-1.5 min-w-[5vh]">
                    <div
                        className="w-2 h-2 rounded-full"
                        style={{ backgroundColor: projectColor }}
                    />
                    <span className="text-gray-500 text-sm">{projectName}</span>
                </div>
                <span className={cn("px-2 py-0.5 rounded text-xs", STATUS_COLORS[props.status])}>
                    {STATUS_LABELS[props.status]}
                </span>

                <div className={cn("leading-7", isDone || props.archived ? "line-through text-gray-500" : "")}>
                    {props.priority > 0 && <span className="text-red-600 font-bold pr-2">{"!".repeat(props.priority)}</span>}
                    {props.content}
                </div>
                {props.archived && <Badge variant="outline">ARCHIVED</Badge>}
            </div>
        </div>
    )
}
