import dayjs from "dayjs";
import { findLast, sortBy } from "lodash";
import { useMemo } from "react";
import NavBar from "../components/NavBar";
import TimelineTaskItem from "../components/TimelineTaskItem";
import type { TaskResponse } from "../api/schema";
import { useTasks } from "../hooks/useTasks";

export default function Timeline() {
  const { tasks: timelineTasks } = useTasks();

  const dayTasks = useMemo(() => {
    const datedTasks: { [date: string]: Set<TaskResponse> } = {};

    (timelineTasks ?? []).forEach((item) => {
      item.events.forEach((event) => {
        const day = dayjs(event.datetime).format("YYYY-MM-DD");
        if (day in datedTasks) {
          datedTasks[day].add(item);
        } else {
          datedTasks[day] = new Set();
          datedTasks[day].add(item);
        }
      });
    });
    return sortBy(Object.keys(datedTasks), (o) => dayjs(o))
      .map((date) => {
        return {
          date,
          tasks: sortBy(Array.from(datedTasks[date].values()), (o) => {
            const event = findLast(o.events);
            return dayjs(event?.datetime);
          }).reverse(),
        };
      })
      .reverse();
  }, [timelineTasks]);

  return (
    <div className="container mx-auto mt-12">
      <NavBar />
      <div className="mt-4">
        {dayTasks.map((group) => (
          <div key={group.date} className="mt-4">
            <p className="text-lg font-medium">
              {dayjs(group.date).format("dddd,  MMM D, YYYY")}
            </p>
            <div>
              {group.tasks.map((task) => (
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
  );
}
