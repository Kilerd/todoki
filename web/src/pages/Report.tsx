import dayjs from "dayjs";
import { groupBy, sortBy } from "lodash";
import { useCallback, useEffect, useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { Skeleton } from "@/components/ui/skeleton";
import { cn } from "@/lib/utils";
import NavBar from "../components/NavBar";
import type { TaskResponse, TaskEventResponse } from "../api/schema";
import { useTasks, useTodayDoneTasks } from "../hooks/useTasks";
import { fetchReport } from "../api/tasks";

interface ReportSummary {
  period: "today" | "week" | "month";
  created_count: number;
  done_count: number;
  archived_count: number;
  state_changes_count: number;
  comments_count: number;
}

const STATUS_LABELS: Record<string, string> = {
  backlog: "Backlog",
  todo: "Todo",
  "in-progress": "In Progress",
  "in-review": "In Review",
  done: "Done",
};

function StatItem({
  value,
  label,
  isLoading,
  accent = false,
}: {
  value: number;
  label: string;
  isLoading: boolean;
  accent?: boolean;
}) {
  return (
    <div className="flex flex-col items-center py-6 px-4">
      {isLoading ? (
        <Skeleton className="h-12 w-12 rounded-full mb-2" />
      ) : (
        <span
          className={cn(
            "text-4xl font-semibold tabular-nums",
            accent ? "text-teal-600" : "text-slate-800"
          )}
        >
          {value}
        </span>
      )}
      <span className="text-sm text-slate-500 mt-1">{label}</span>
    </div>
  );
}

function formatEventDescription(event: TaskEventResponse): string {
  switch (event.event_type) {
    case "Create":
      return "Created";
    case "StatusChange": {
      const fromLabel = STATUS_LABELS[event.from_state ?? ""] ?? event.from_state;
      const toLabel = STATUS_LABELS[event.state ?? ""] ?? event.state;
      if (fromLabel && toLabel) {
        return `${fromLabel} → ${toLabel}`;
      }
      return `→ ${toLabel}`;
    }
    case "Archived":
      return "Archived";
    case "Unarchived":
      return "Restored";
    case "CreateComment":
      return "Commented";
    default:
      return event.event_type;
  }
}

interface ActivityEvent {
  task: TaskResponse;
  event: TaskEventResponse;
}

function useReport() {
  const [report, setReport] = useState<ReportSummary | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  const refresh = useCallback(async () => {
    setIsLoading(true);
    try {
      const data = await fetchReport("today");
      setReport(data);
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return { report, isLoading, refresh };
}

export default function Report() {
  const { tasks } = useTasks();
  const { tasks: doneTasks } = useTodayDoneTasks();
  const { report, isLoading: isReportLoading } = useReport();

  const activityByDay = useMemo(() => {
    const allEvents: ActivityEvent[] = [];
    const seenTaskIds = new Set<string>();

    // Combine inbox tasks and done tasks, avoiding duplicates
    const allTasks = [...(tasks ?? []), ...(doneTasks ?? [])];
    allTasks.forEach((task) => {
      if (seenTaskIds.has(task.id)) return;
      seenTaskIds.add(task.id);
      task.events.forEach((event) => {
        allEvents.push({ task, event });
      });
    });

    const sorted = sortBy(allEvents, (e) => e.event.datetime).reverse();

    const grouped = groupBy(sorted, (e) =>
      dayjs(e.event.datetime).format("YYYY-MM-DD")
    );

    return sortBy(Object.keys(grouped), (d) => d)
      .reverse()
      .map((date) => ({
        date,
        events: grouped[date],
      }));
  }, [tasks, doneTasks]);

  return (
    <div className="container mx-auto mt-12 max-w-3xl">
      <NavBar />

      <div className="mt-8 space-y-8">
        {/* Stats Summary */}
        <section>
          <h2 className="text-xs font-medium text-slate-400 uppercase tracking-wider mb-4">
            Today's Summary
          </h2>
          <div className="grid grid-cols-4 divide-x divide-slate-100 border border-slate-100 rounded-lg bg-white">
            <StatItem
              value={report?.created_count ?? 0}
              label="Created"
              isLoading={isReportLoading}
            />
            <StatItem
              value={report?.done_count ?? 0}
              label="Done"
              isLoading={isReportLoading}
              accent
            />
            <StatItem
              value={report?.archived_count ?? 0}
              label="Archived"
              isLoading={isReportLoading}
            />
            <StatItem
              value={report?.state_changes_count ?? 0}
              label="Changes"
              isLoading={isReportLoading}
            />
          </div>
        </section>

        {/* Activity Log */}
        <section>
          <h2 className="text-xs font-medium text-slate-400 uppercase tracking-wider mb-4">
            Activity
          </h2>

          <div className="space-y-6">
            {activityByDay.map((group) => (
              <div key={group.date}>
                <div className="flex items-center gap-3 mb-3">
                  <span className="text-sm font-medium text-slate-700">
                    {dayjs(group.date).format("ddd, MMM D")}
                  </span>
                  <div className="flex-1 h-px bg-slate-100" />
                  <span className="text-xs text-slate-400">
                    {group.events.length} events
                  </span>
                </div>

                <div className="space-y-1 pl-1">
                  {group.events.map((item, idx) => (
                    <div
                      key={`${item.event.id}-${idx}`}
                      className="flex items-baseline gap-3 py-1.5 group"
                    >
                      <span className="text-xs text-slate-400 font-mono w-14 shrink-0">
                        {dayjs(item.event.datetime).format("HH:mm")}
                      </span>
                      <Link
                        to={`/inbox/${item.task.id}`}
                        className="text-sm text-slate-700 hover:text-teal-600 transition-colors duration-150 truncate cursor-pointer"
                      >
                        {item.task.content}
                      </Link>
                      <span className="text-xs text-slate-400 shrink-0">
                        {formatEventDescription(item.event)}
                      </span>
                    </div>
                  ))}
                </div>
              </div>
            ))}

            {activityByDay.length === 0 && (
              <div className="text-center py-12 text-slate-400">
                No activity yet
              </div>
            )}
          </div>
        </section>
      </div>
    </div>
  );
}
