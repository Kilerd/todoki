import dayjs from 'dayjs';
import { findLast, sortBy } from "lodash";
import { useMemo } from "react";
import useSWR from "swr";
import NavBar from "../components/NavBar";
import TimelineTaskItem from "../components/TimelineTaskItem";
import { ItemLineItem } from "../lib/schema";
import { fetcher } from "../services/api";

export default function Timeline() {
    const {data: timelineTasks} = useSWR<ItemLineItem[]>("/timelines", fetcher);

    const dayTasks = useMemo(() => {
        let datedTasks :{ [date:string]: Set<ItemLineItem> }= {};

        (timelineTasks??[]).forEach(item => {
            item.events.forEach(event => {
                const day = dayjs(event.datetime).format("YYYY-MM-DD");
                if(day in datedTasks) {
                    datedTasks[day].add(item);
                }else {
                    datedTasks[day] = new Set();
                    datedTasks[day].add(item);
                }
            })
        });
        return sortBy(Object.keys(datedTasks), o => dayjs(o)).map(date => {
            return {
                date, tasks: sortBy(Array.from(datedTasks[date].values()), o => {
                    const event = findLast(o.events)
                    return dayjs(event?.datetime)
                }).reverse()
            }
        }).reverse()

    }, [timelineTasks])

    return (
        <div className="container mx-auto mt-12">
            <NavBar/>
            <div className="mt-4">
                {dayTasks.map(group => (
                    <div key={group.date} className="mt-4">
                        <p className="text-lg font-medium">
                            {dayjs(group.date).format("dddd,  MMM D, YYYY")}
                        </p>
                        <div>
                            {group.tasks.map(task => (
                                <TimelineTaskItem 
                                    key={task.id} 
                                    {...task} 
                                    grouped_day={dayjs(group.date)} 
                                />
                            ))}
                        </div>
                    </div>
                ))}
            </div>
        </div>
    )
}