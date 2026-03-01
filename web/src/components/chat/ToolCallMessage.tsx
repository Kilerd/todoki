import { Wrench, CheckCircle2, Clock, XCircle, ChevronDown } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import { useState } from "react";
import type { ToolCallMessage as ToolCallData } from "./types";

interface ToolCallMessageProps {
  tools: ToolCallData[];
  timestamp?: number;
  className?: string;
}

function ToolCallItem({ tool }: { tool: ToolCallData }) {
  const [expanded, setExpanded] = useState(false);

  const hasInput = tool.raw_input && Object.keys(tool.raw_input).length > 0;
  const statusIcon = {
    completed: <CheckCircle2 className="h-3 w-3 text-green-500" />,
    pending: <Clock className="h-3 w-3 text-amber-500 animate-pulse" />,
    error: <XCircle className="h-3 w-3 text-red-500" />,
  }[tool.status];

  return (
    <div className="border border-slate-200 rounded-lg overflow-hidden bg-white">
      <button
        onClick={() => setExpanded(!expanded)}
        className={cn(
          "w-full flex items-center gap-2 px-3 py-2 text-left hover:bg-slate-50 transition-colors",
          expanded && "border-b border-slate-100"
        )}
      >
        <Wrench className="h-3.5 w-3.5 text-orange-500 flex-shrink-0" />
        <span className="text-xs font-medium text-slate-700 flex-1 truncate">
          {tool.title || tool.kind}
        </span>
        {statusIcon}
        <ChevronDown
          className={cn(
            "h-3 w-3 text-slate-400 transition-transform",
            expanded && "rotate-180"
          )}
        />
      </button>

      {expanded && hasInput && (
        <div className="px-3 py-2 bg-slate-50">
          <div className="text-[10px] font-medium text-slate-500 mb-1">Input</div>
          <pre className="text-[10px] text-slate-600 font-mono overflow-x-auto whitespace-pre-wrap break-all">
            {JSON.stringify(tool.raw_input, null, 2)}
          </pre>
        </div>
      )}
    </div>
  );
}

export function ToolCallMessage({ tools, timestamp, className }: ToolCallMessageProps) {
  if (tools.length === 0) return null;

  // Dedupe tools by id, keep latest state
  const uniqueTools = new Map<string, ToolCallData>();
  for (const tool of tools) {
    uniqueTools.set(tool.id, tool);
  }

  const toolList = Array.from(uniqueTools.values());

  return (
    <div className={cn("flex gap-3 py-3", className)}>
      {/* Avatar placeholder for alignment */}
      <div className="flex-shrink-0 w-8 h-8 rounded-full bg-orange-100 flex items-center justify-center">
        <Wrench className="h-4 w-4 text-orange-600" />
      </div>

      {/* Tool calls */}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 mb-2">
          <span className="text-sm font-medium text-slate-600">Tool Calls</span>
          <Badge variant="outline" className="text-[10px] h-4 text-orange-600 border-orange-300">
            {toolList.length}
          </Badge>
          {timestamp && (
            <span className="text-xs text-slate-400">
              {new Date(timestamp / 1000000).toLocaleTimeString()}
            </span>
          )}
        </div>
        <div className="space-y-2">
          {toolList.map((tool) => (
            <ToolCallItem key={tool.id} tool={tool} />
          ))}
        </div>
      </div>
    </div>
  );
}
