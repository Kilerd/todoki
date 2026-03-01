import { Terminal, ChevronDown } from "lucide-react";
import { cn } from "@/lib/utils";
import { useState } from "react";
import type { SystemMessageData } from "./types";

interface SystemMessageProps {
  data: SystemMessageData[];
  timestamp?: number;
  className?: string;
}

export function SystemMessage({ data, timestamp, className }: SystemMessageProps) {
  const [expanded, setExpanded] = useState(false);

  if (data.length === 0) return null;

  // Check for available_commands type
  const commandsData = data.find((d) => d.type === "available_commands");

  return (
    <div className={cn("flex gap-3 py-2", className)}>
      {/* Avatar */}
      <div className="flex-shrink-0 w-8 h-8 rounded-full bg-slate-100 flex items-center justify-center">
        <Terminal className="h-4 w-4 text-slate-500" />
      </div>

      {/* System message */}
      <div className="flex-1 min-w-0">
        <button
          onClick={() => setExpanded(!expanded)}
          className="flex items-center gap-2 text-left hover:bg-slate-50 rounded px-1 -ml-1 transition-colors"
        >
          <span className="text-xs font-medium text-slate-500">System</span>
          {timestamp && (
            <span className="text-xs text-slate-400">
              {new Date(timestamp / 1000000).toLocaleTimeString()}
            </span>
          )}
          <ChevronDown
            className={cn(
              "h-3 w-3 text-slate-400 transition-transform",
              expanded && "rotate-180"
            )}
          />
        </button>

        {expanded && (
          <div className="mt-2 bg-slate-50 rounded-lg p-3 border border-slate-200">
            {commandsData?.commands ? (
              <div className="space-y-1">
                <div className="text-[10px] font-medium text-slate-500 mb-2">
                  Available Commands
                </div>
                {commandsData.commands.map((cmd) => (
                  <div key={cmd.name} className="flex items-start gap-2 text-xs">
                    <code className="text-purple-600 font-mono">/{cmd.name}</code>
                    <span className="text-slate-500">{cmd.description}</span>
                  </div>
                ))}
              </div>
            ) : (
              <pre className="text-[10px] text-slate-600 font-mono overflow-x-auto whitespace-pre-wrap">
                {JSON.stringify(data, null, 2)}
              </pre>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
