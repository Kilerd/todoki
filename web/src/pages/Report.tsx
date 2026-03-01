import dayjs from "dayjs";
import { groupBy, sortBy } from "lodash";
import { useCallback, useEffect, useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { Skeleton } from "@/components/ui/skeleton";
import { cn } from "@/lib/utils";
import NavBar from "../components/NavBar";
import type { TaskResponse, TaskEvent } from "../api/types";
import { useTasks, useTodayDoneTasks } from "../hooks/useTasks";
import { getProjectById } from "../hooks/useProjects";
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

interface TaskActivity {
  task: TaskResponse;
  events: TaskEvent[];
  summary: string;
  lastEventTime: string;
}

function summarizeTaskActivity(events: TaskEvent[]): string {
  const sorted = sortBy(events, (e) => e.datetime);
  const parts: string[] = [];

  const created = sorted.some((e) => e.event_type === "Create");
  const archived = sorted.some((e) => e.event_type === "Archived");
  const unarchived = sorted.some((e) => e.event_type === "Unarchived");
  const commentCount = sorted.filter((e) => e.event_type === "CreateComment").length;

  // Build status flow from StatusChange events
  const statusChanges = sorted.filter((e) => e.event_type === "StatusChange");
  const statusFlow: string[] = [];

  if (created) {
    statusFlow.push("Created");
  }

  statusChanges.forEach((e) => {
    const toLabel = STATUS_LABELS[e.state ?? ""] ?? e.state;
    if (toLabel && statusFlow[statusFlow.length - 1] !== toLabel) {
      statusFlow.push(toLabel);
    }
  });

  if (statusFlow.length > 0) {
    parts.push(statusFlow.join(" → "));
  }

  if (archived) {
    parts.push("Archived");
  }
  if (unarchived) {
    parts.push("Restored");
  }
  if (commentCount > 0) {
    parts.push(`${commentCount} comment${commentCount > 1 ? "s" : ""}`);
  }

  return parts.join(" · ") || "Activity";
}

function useReport() {
  const [report, setReport] = useState<ReportSummary | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  const refresh = useCallback(async () => {
    setIsLoading(true);
    try {
      const { data } = await fetchReport({ period: "today" });
      setReport(data as ReportSummary);
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

  const todayActivity = useMemo(() => {
    const seenTaskIds = new Set<string>();
    const today = dayjs().format("YYYY-MM-DD");

    // Combine inbox tasks and done tasks, avoiding duplicates
    const allTasks = [...(tasks ?? []), ...(doneTasks ?? [])].filter((task) => {
      if (seenTaskIds.has(task.id)) return false;
      seenTaskIds.add(task.id);
      return true;
    });

    // Get today's events grouped by task
    const taskEventsMap: Record<string, { task: TaskResponse; events: TaskEvent[] }> = {};

    allTasks.forEach((task) => {
      const todayEvents = task.events.filter(
        (event) => dayjs(event.datetime).format("YYYY-MM-DD") === today
      );
      if (todayEvents.length > 0) {
        taskEventsMap[task.id] = { task, events: todayEvents };
      }
    });

    // Convert to activities with summaries
    const taskActivities: TaskActivity[] = Object.values(taskEventsMap).map(
      ({ task, events }) => {
        const sorted = sortBy(events, (e) => e.datetime);
        return {
          task,
          events,
          summary: summarizeTaskActivity(events),
          lastEventTime: sorted[sorted.length - 1]?.datetime ?? "",
        };
      }
    );

    // Group by task project
    const byGroup = groupBy(taskActivities, (a) => {
      const project = a.task.project_id ? getProjectById(a.task.project_id) : null;
      return project?.name || "Inbox";
    });
    return sortBy(Object.keys(byGroup)).map((groupName) => ({
      groupName,
      activities: sortBy(byGroup[groupName], (a) => a.lastEventTime).reverse(),
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
        {todayActivity.map((group) => (
          <section key={group.groupName}>
            <h2 className="text-xs font-medium text-slate-400 uppercase tracking-wider mb-4">
              {group.groupName} activity
            </h2>
            <div className="space-y-1">
              {group.activities.map((activity) => (
                <div
                  key={activity.task.id}
                  className="flex items-center gap-3 py-1.5 rounded hover:bg-slate-50 transition-colors"
                >
                  <Link
                    to={`/inbox/${activity.task.id}`}
                    className="text-sm text-slate-700 hover:text-teal-600 transition-colors duration-150 truncate flex-1 cursor-pointer"
                  >
                    {activity.task.content}
                  </Link>
                  <span className="text-xs text-slate-500 shrink-0">
                    {activity.summary}
                  </span>
                </div>
              ))}
            </div>
          </section>
        ))}

        {todayActivity.length === 0 && (
          <section>
            <div className="text-center py-12 text-slate-400">
              No activity today
            </div>
          </section>
        )}
      </div>
    </div>
  );
}
