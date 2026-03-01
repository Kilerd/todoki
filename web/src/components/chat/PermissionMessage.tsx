import { Shield, CheckCircle2, XCircle } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import { useState } from "react";
import { emitEvent } from "@/api/eventBus";

interface PermissionMessageProps {
  requestId: string;
  sessionId: string;
  agentId: string;
  taskId?: string | null;
  toolCall?: {
    title?: string;
    kind?: string;
  };
  outcome?: {
    selected?: { option_id: string };
    cancelled?: boolean;
  };
  timestamp?: number;
  className?: string;
}

export function PermissionMessage({
  requestId,
  sessionId,
  agentId,
  taskId,
  toolCall,
  outcome,
  timestamp,
  className,
}: PermissionMessageProps) {
  const [isLoading, setIsLoading] = useState(false);
  const [localOutcome, setLocalOutcome] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Use outcome from props (merged state) or local state
  const isResponded = !!outcome || !!localOutcome;
  const responseLabel = outcome?.cancelled
    ? "Rejected"
    : outcome?.selected?.option_id === "allow_always"
      ? "Always Allowed"
      : outcome?.selected?.option_id === "allow"
        ? "Allowed"
        : localOutcome;

  const handleRespond = async (action: "allow" | "allow_always" | "reject") => {
    setIsLoading(true);
    setError(null);

    try {
      await emitEvent({
        agent_id: agentId,
        session_id: sessionId,
        task_id: taskId,
        kind: "permission.responded",
        data: {
          relay_id: "",
          request_id: requestId,
          session_id: sessionId,
          outcome: action === "reject" ? { cancelled: true } : { selected: { option_id: action } },
        },
      });

      setLocalOutcome(action === "reject" ? "Rejected" : action === "allow_always" ? "Always Allowed" : "Allowed");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to respond");
    } finally {
      setIsLoading(false);
    }
  };

  const isRejected = outcome?.cancelled || localOutcome === "Rejected";

  return (
    <div className={cn("flex gap-3 py-3", className)}>
      {/* Avatar */}
      <div className={cn(
        "flex-shrink-0 w-8 h-8 rounded-full flex items-center justify-center",
        isResponded
          ? isRejected
            ? "bg-red-100"
            : "bg-green-100"
          : "bg-orange-100"
      )}>
        <Shield className={cn(
          "h-4 w-4",
          isResponded
            ? isRejected
              ? "text-red-600"
              : "text-green-600"
            : "text-orange-600"
        )} />
      </div>

      {/* Content */}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 mb-2">
          <span className={cn(
            "text-sm font-medium",
            isResponded
              ? isRejected
                ? "text-red-600"
                : "text-green-600"
              : "text-orange-600"
          )}>
            Permission Request
          </span>
          {timestamp && (
            <span className="text-xs text-slate-400">
              {new Date(timestamp / 1000000).toLocaleTimeString()}
            </span>
          )}
        </div>

        {/* Tool call info */}
        {toolCall?.title && (
          <div className="text-xs text-slate-700 bg-slate-50 p-2 rounded mb-3 font-mono border border-slate-200">
            {toolCall.title}
          </div>
        )}

        {/* Action buttons or responded state */}
        {isResponded ? (
          <Badge
            variant="outline"
            className={cn(
              isRejected
                ? "text-red-600 border-red-300"
                : "text-green-600 border-green-300"
            )}
          >
            {isRejected ? (
              <XCircle className="h-3 w-3 mr-1" />
            ) : (
              <CheckCircle2 className="h-3 w-3 mr-1" />
            )}
            {responseLabel}
          </Badge>
        ) : (
          <div className="flex items-center gap-2">
            <Button
              size="sm"
              variant="outline"
              className="h-7 text-xs cursor-pointer text-green-600 border-green-300 hover:bg-green-50"
              onClick={() => handleRespond("allow")}
              disabled={isLoading}
            >
              Allow
            </Button>
            <Button
              size="sm"
              variant="outline"
              className="h-7 text-xs cursor-pointer text-blue-600 border-blue-300 hover:bg-blue-50"
              onClick={() => handleRespond("allow_always")}
              disabled={isLoading}
            >
              Always Allow
            </Button>
            <Button
              size="sm"
              variant="outline"
              className="h-7 text-xs cursor-pointer text-red-600 border-red-300 hover:bg-red-50"
              onClick={() => handleRespond("reject")}
              disabled={isLoading}
            >
              Reject
            </Button>
            {error && <span className="text-xs text-red-500">{error}</span>}
          </div>
        )}
      </div>
    </div>
  );
}
