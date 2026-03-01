import { Code, ChevronDown, FileText } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import { useState } from "react";
import type { ToolCallMessage as ToolCallData } from "./types";

interface ToolResultMessageProps {
  results: ToolCallData[];
  timestamp?: number;
  className?: string;
}

function ToolResultItem({ result }: { result: ToolCallData }) {
  const [expanded, setExpanded] = useState(false);

  const toolName = result.meta?.claudeCode?.toolName || result.kind;
  const hasResponse = result.meta?.claudeCode?.toolResponse;

  return (
    <div className="border border-slate-200 rounded-lg overflow-hidden bg-white">
      <button
        onClick={() => setExpanded(!expanded)}
        className={cn(
          "w-full flex items-center gap-2 px-3 py-2 text-left hover:bg-slate-50 transition-colors",
          expanded && "border-b border-slate-100"
        )}
      >
        <FileText className="h-3.5 w-3.5 text-green-500 flex-shrink-0" />
        <span className="text-xs font-medium text-slate-700 flex-1 truncate">
          {toolName}
        </span>
        <Badge
          variant="outline"
          className="text-[10px] h-4 text-green-600 border-green-300"
        >
          completed
        </Badge>
        <ChevronDown
          className={cn(
            "h-3 w-3 text-slate-400 transition-transform",
            expanded && "rotate-180"
          )}
        />
      </button>

      {expanded && !!hasResponse && (
        <div className="px-3 py-2 bg-slate-50 max-h-48 overflow-y-auto">
          <div className="text-[10px] font-medium text-slate-500 mb-1">Response</div>
          <pre className="text-[10px] text-slate-600 font-mono overflow-x-auto whitespace-pre-wrap break-all">
            {JSON.stringify(result.meta?.claudeCode?.toolResponse, null, 2)}
          </pre>
        </div>
      )}
    </div>
  );
}

export function ToolResultMessage({ results, timestamp, className }: ToolResultMessageProps) {
  const completedResults = results.filter((r) => r.status === "completed");

  if (completedResults.length === 0) return null;

  return (
    <div className={cn("flex gap-3 py-3", className)}>
      {/* Avatar */}
      <div className="flex-shrink-0 w-8 h-8 rounded-full bg-green-100 flex items-center justify-center">
        <Code className="h-4 w-4 text-green-600" />
      </div>

      {/* Results */}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 mb-2">
          <span className="text-sm font-medium text-slate-600">Tool Results</span>
          <Badge variant="outline" className="text-[10px] h-4 text-green-600 border-green-300">
            {completedResults.length}
          </Badge>
          {timestamp && (
            <span className="text-xs text-slate-400">
              {new Date(timestamp / 1000000).toLocaleTimeString()}
            </span>
          )}
        </div>
        <div className="space-y-2">
          {completedResults.map((result) => (
            <ToolResultItem key={result.id} result={result} />
          ))}
        </div>
      </div>
    </div>
  );
}
