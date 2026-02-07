import { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from "@/components/ui/accordion";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Calendar } from "@/components/ui/calendar";
import dayjs from "dayjs";
import useSWR, { mutate } from "swr";
import { disableHabit, enableHabit, fetcher } from "../services/api";

interface Props {
    id: string,
    name: string,
    trigger_days: string[]
    enabled: boolean
}

export default function HabitItem(props: Props) {
    const { data: calenderDates } = useSWR<string[]>(`/habits/${props.id}/calendar`, fetcher);
    const minDate = dayjs().subtract(2, "month").toDate();

    const handleSubmit = async () => {
        if (props.enabled) {
            await disableHabit(props.id);
        } else {
            await enableHabit(props.id);
        }
        await mutate("/habits")
    }

    return (
        <Accordion type="single" collapsible>
            <AccordionItem value="calendar">
                <AccordionTrigger>
                    <div className="flex items-center justify-between w-full">
                        <div className="flex items-center gap-2">
                            <Badge variant={props.enabled ? "default" : "secondary"}>
                                {props.enabled ? "有效" : "暂停"}
                            </Badge>
                            <span>{props.name}</span>
                        </div>

                        <div className="flex gap-1">
                            {[
                                { day: "Sunday", label: "日" },
                                { day: "Monday", label: "一" },
                                { day: "Tuesday", label: "二" },
                                { day: "Wednesday", label: "三" },
                                { day: "Thursday", label: "四" },
                                { day: "Friday", label: "五" },
                                { day: "Saturday", label: "六" }
                            ].map(({ day, label }) => (
                                <span key={day} 
                                    className={`inline-flex items-center justify-center w-6 h-6 text-xs rounded-full
                                        ${props.trigger_days.includes(day) 
                                            ? 'bg-primary text-primary-foreground' 
                                            : 'bg-secondary text-secondary-foreground'}`}>
                                    {label}
                                </span>
                            ))}
                        </div>
                    </div>
                </AccordionTrigger>
                <AccordionContent>
                    <div className="flex flex-col items-center gap-4">
                        <Button onClick={handleSubmit}>
                            {props.enabled ? "暂停" : "开启"}
                        </Button>

                        <div className="grid grid-cols-3 gap-4">
                            {Array.from({ length: 3 }).map((_, i) => (
                                <Calendar
                                    key={i}
                                    mode="single"
                                    defaultMonth={dayjs(minDate).add(i, 'month').toDate()}
                                    selected={calenderDates?.length ? new Date(calenderDates[0]) : undefined}
                                    className="rounded-md border"
                                    disabled={(date) => dayjs(date).isAfter(dayjs())}
                                    modifiers={{
                                        completed: (date) => (calenderDates ?? []).some(
                                            one => dayjs(one).format("YYYY-MM-DD") === dayjs(date).format("YYYY-MM-DD")
                                        )
                                    }}
                                    modifiersClassNames={{
                                        completed: "bg-primary text-primary-foreground rounded-full"
                                    }}
                                />
                            ))}
                        </div>
                    </div>
                </AccordionContent>
            </AccordionItem>
        </Accordion>
    )
}