import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Checkbox } from "@/components/ui/checkbox";

interface Props {
    id: string;
    priority: number;
    content: string;
    group?: string;
    task_type: "Todo" | "Stateful";
    done: boolean;
    archived: boolean;
    current_state?: string | null;
    states?: string[] | null;
}

export default function PreviewTaskItem(props: Props) {
    const [checked] = useState(props.done);

    const current_index = (props.states ?? []).findIndex(it => it === props.current_state)
    const prevState = props.states?.[current_index - 1]
    const nextState = props.states?.[current_index + 1]

    const group = props.group ?? "default";

    return (
        <div className="flex items-center justify-between p-2 rounded-md hover:bg-gray-100 group">
            <div className="flex items-center gap-2">
                <span className="text-gray-500 min-w-[5vh]">{group}</span>
                {props.task_type === "Todo" && 
                    <Checkbox disabled={props.archived} checked={checked} />}
                {props.task_type === "Stateful" &&
                    <Badge variant="secondary">{props.current_state}</Badge>}
                
                <div className={`leading-7 ${props.done || props.archived ? 'line-through text-gray-500' : ''}`}>
                    {props.priority > 0 && <span className="text-red-600 font-bold pr-2">{"!".repeat(props.priority)}</span>}
                    {props.content}
                </div>
                {props.archived && <Badge variant="outline">ARCHIVED</Badge>}
            </div>

            <div className="hidden group-hover:flex items-center gap-2">
                {(!props.archived && prevState !== undefined) &&
                    <Button variant="secondary" size="sm">
                        Back to {prevState}
                    </Button>
                }
                {(!props.archived && nextState !== undefined) &&
                    <Button size="sm">
                        Goto {nextState}
                    </Button>
                }
            </div>
        </div>
    )
}