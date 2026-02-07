import NavBar from "../components/NavBar";
import { useState } from "react";
import { parseTask } from "../utils/taskParser";
import { api, fetcher } from "../services/api";
import useSWR, { mutate } from "swr";
import HabitItem from "../components/HabitItem";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { Separator } from "@/components/ui/separator";

export default function Habits() {
    const [name, setName] = useState("");
    const [newTaskText, setNewTaskText] = useState("");
    const [triggerDays, setTriggerDays] = useState<string[]>([]);

    const {data: habits, isLoading} = useSWR<any[]>("/habits", fetcher);
    
    const handleSubmit = async () => {
        const data = parseTask(newTaskText);
        await api.post("/habits", {
            trigger_days: triggerDays,
            name: name.trim(),
            ...data
        });
        setNewTaskText("")
        setName("")
        await mutate("/habits")
    }

    return (
        <div className="container mx-auto mt-12">
            <NavBar/>

            <Accordion type="single" collapsible className="mt-4">
                <AccordionItem value="new-habit">
                    <AccordionTrigger>新增习惯</AccordionTrigger>
                    <AccordionContent>
                        <div className="mt-4 space-y-4">
                            <div>
                                <Input
                                    value={name}
                                    onChange={(e) => setName(e.target.value)}
                                    placeholder="habit name"
                                />
                            </div>
                            <div>
                                <Input
                                    value={newTaskText}
                                    onChange={(e) => setNewTaskText(e.target.value)}
                                    placeholder="Input a new habit"
                                />
                            </div>
                            <div>
                                <ToggleGroup 
                                    type="multiple" 
                                    value={triggerDays}
                                    onValueChange={setTriggerDays}
                                    className="flex flex-wrap gap-2"
                                >
                                    <ToggleGroupItem value="Sunday">日</ToggleGroupItem>
                                    <ToggleGroupItem value="Monday">一</ToggleGroupItem>
                                    <ToggleGroupItem value="Tuesday">二</ToggleGroupItem>
                                    <ToggleGroupItem value="Wednesday">三</ToggleGroupItem>
                                    <ToggleGroupItem value="Thursday">四</ToggleGroupItem>
                                    <ToggleGroupItem value="Friday">五</ToggleGroupItem>
                                    <ToggleGroupItem value="Saturday">六</ToggleGroupItem>
                                </ToggleGroup>
                            </div>
                            <div>
                                <Button
                                    disabled={name.trim().length === 0 || newTaskText.trim().length === 0 || triggerDays.length === 0}
                                    onClick={handleSubmit}
                                >
                                    创建
                                </Button>
                            </div>
                        </div>
                    </AccordionContent>
                </AccordionItem>
            </Accordion>

            <Separator className="my-4" />

            {!isLoading && (habits ?? []).map(habit => (
                <div key={habit.id} className="mt-4">
                    <HabitItem {...habit}/>
                </div>
            ))}
        </div>
    )
}