import { ClipboardList, CheckCircle2, Circle } from "lucide-react";
import { cn } from "@/lib/utils";
import type { PlanEntry } from "./types";

interface PlanMessageProps {
  entries: PlanEntry[];
  timestamp?: number;
  className?: string;
}

export function PlanMessage({ entries, timestamp, className }: PlanMessageProps) {
  if (entries.length === 0) return null;

  const completedCount = entries.filter((e) => e.status === "completed").length;

  return (
    <div className={cn("flex gap-3 py-3", className)}>
      {/* Avatar */}
      <div className="flex-shrink-0 w-8 h-8 rounded-full bg-purple-100 flex items-center justify-center">
        <ClipboardList className="h-4 w-4 text-purple-600" />
      </div>

      {/* Plan content */}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 mb-2">
          <span className="text-sm font-medium text-slate-700">Plan</span>
          <span className="text-xs text-slate-400">
            {completedCount}/{entries.length} completed
          </span>
          {timestamp && (
            <span className="text-xs text-slate-400">
              {new Date(timestamp / 1000000).toLocaleTimeString()}
            </span>
          )}
        </div>

        <div className="bg-purple-50 rounded-lg p-3 border border-purple-100">
          <div className="space-y-2">
            {entries.map((entry, idx) => (
              <div
                key={idx}
                className={cn(
                  "flex items-start gap-2 text-sm",
                  entry.status === "completed"
                    ? "text-green-700"
                    : "text-slate-600"
                )}
              >
                {entry.status === "completed" ? (
                  <CheckCircle2 className="h-4 w-4 flex-shrink-0 mt-0.5" />
                ) : (
                  <Circle className="h-4 w-4 flex-shrink-0 mt-0.5" />
                )}
                <span
                  className={cn(
                    entry.status === "completed" && "line-through opacity-70"
                  )}
                >
                  {entry.content}
                </span>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
