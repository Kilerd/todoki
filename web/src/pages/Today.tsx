import { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from "@/components/ui/accordion";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Separator } from "@/components/ui/separator";
import { Skeleton } from "@/components/ui/skeleton";
import { useOs } from "@mantine/hooks";
import { filter, orderBy } from "lodash";
import { Archive, ArrowRight, Eye } from "lucide-react";
import { useMemo, useState } from "react";
import useSWR, { useSWRConfig } from "swr";
import ArchivedTaskItem from "../components/ArchivedTaskItem";
import NavBar from "../components/NavBar";
import PreviewTaskItem from "../components/PreviewTaskItem";
import TaskItem from "../components/TaskItem";
import { api, fetcher } from "../services/api";
import { parseTask } from "../utils/taskParser";

function Kbd({ children }: { children: React.ReactNode }) {
    return (
        <kbd className="pointer-events-none inline-flex h-5 select-none items-center gap-1 rounded border bg-muted px-1.5 font-mono text-[10px] font-medium text-muted-foreground opacity-100">
            {children}
        </kbd>
    );
}


function Today() {
    const os = useOs();
    const { mutate } = useSWRConfig()
    const [newTaskText, setNewTaskText] = useState("");

    const parsedTask = useMemo(() => parseTask(newTaskText), [newTaskText])

    const { data: tasks, isLoading } = useSWR("/tasks", fetcher);

    console.log("tasks", tasks);

    const openedTasks = useMemo(() => {
        return filter((tasks ?? []), item => item.archived === false && item.done === false)
    }, [tasks]);
    const doneTasks = useMemo(() => {
        return filter((tasks ?? []), item => item.archived === false && item.done === true)
    }, [tasks]);

    const archivedTasks: any[] = useMemo(() => {
        return (tasks ?? []).filter((item: any) => item.archived === true);
    }, [tasks]);

    const handleNewTask = async () => {
        await api.post("/tasks", parsedTask);
        setNewTaskText("")
        await mutate("/tasks")
    }

    const handleKeyDown = (e: React.KeyboardEvent) => {
        if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
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
                        onChange={e => setNewTaskText(e.target.value)}
                        placeholder="输入新任务"
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
                                        预览模式 (点击展开)
                                    </div>
                                </AccordionTrigger>
                                <AccordionContent>
                                    {newTaskText.trim() !== "" &&
                                        <PreviewTaskItem {...parsedTask} id={"previewTask"} done={false}
                                            archived={false} current_state={parsedTask.states?.[0]} />
                                    }
                                </AccordionContent>
                            </AccordionItem>
                        </Accordion>

                        <div className="flex flex-wrap gap-4">
                            <div className="flex items-center gap-2">
                                加入分组: <Kbd>+Group</Kbd>
                            </div>
                            <div className="flex items-center gap-2">
                                状态机流转: <Kbd>[[打开 &gt; 等待回复 &gt; 完成]]</Kbd>
                            </div>
                            <div className="flex items-center gap-2">
                                优先级设定: <Kbd>!!!</Kbd>
                            </div>
                            <div className="flex items-center gap-2">
                                新建快捷键: <Kbd>{os === "macos" ? "Command" : "Control"} + Enter</Kbd>
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
                <>
                    <div className="space-y-2 mt-4">
                        {orderBy(openedTasks, ["priority", "create_at"], ["desc", "asc"]).map(task => (
                            <TaskItem key={task.id} {...task}></TaskItem>
                        ))}
                        <Separator className="my-4" />
                        {orderBy(doneTasks, ["priority", "create_at"], ["desc", "asc"]).map(task => (
                            <TaskItem key={task.id} {...task}></TaskItem>
                        ))}
                    </div>

                    {archivedTasks.length > 0 && (
                        <>
                            <Separator className="my-4" />
                            <Accordion type="single" collapsible>
                                <AccordionItem value="archived">
                                    <AccordionTrigger>
                                        <div className="flex items-center gap-2">
                                            <Archive className="h-5 w-5" />
                                            Archived Tasks
                                        </div>
                                    </AccordionTrigger>
                                    <AccordionContent>
                                        <div>
                                            {archivedTasks.map(task => (
                                                <ArchivedTaskItem key={task.id} {...task}></ArchivedTaskItem>
                                            ))}
                                        </div>
                                    </AccordionContent>
                                </AccordionItem>
                            </Accordion>
                        </>
                    )}
                </>
            )}
        </div>
    )
}

export default Today
