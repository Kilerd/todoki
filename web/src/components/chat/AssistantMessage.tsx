import { Bot } from "lucide-react";
import { cn } from "@/lib/utils";

interface AssistantMessageProps {
  text: string;
  timestamp?: number;
  className?: string;
}

export function AssistantMessage({ text, timestamp, className }: AssistantMessageProps) {
  if (!text.trim()) return null;

  return (
    <div className={cn("flex gap-3 py-3", className)}>
      {/* Avatar */}
      <div className="flex-shrink-0 w-8 h-8 rounded-full bg-gradient-to-br from-blue-500 to-purple-600 flex items-center justify-center">
        <Bot className="h-4 w-4 text-white" />
      </div>

      {/* Message content */}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 mb-1">
          <span className="text-sm font-medium text-slate-800">Assistant</span>
          {timestamp && (
            <span className="text-xs text-slate-400">
              {new Date(timestamp / 1000000).toLocaleTimeString()}
            </span>
          )}
        </div>
        <div className="text-sm text-slate-700 whitespace-pre-wrap break-words leading-relaxed">
          {text}
        </div>
      </div>
    </div>
  );
}
