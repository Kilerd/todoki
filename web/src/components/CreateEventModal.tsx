import { useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
import { emitEvent } from "@/api/eventBus";
import { Plus, Loader2, CheckCircle2 } from "lucide-react";
import { Alert, AlertDescription } from "@/components/ui/alert";

interface CreateEventModalProps {
  /** Optional callback after event is created */
  onEventCreated?: (cursor: number) => void;
}

const EVENT_KINDS = [
  { category: "Task Events", kinds: ["task.created", "task.updated", "task.completed", "task.failed"] },
  { category: "Agent Events", kinds: ["agent.started", "agent.stopped", "agent.requirement_analyzed"] },
  { category: "Artifact Events", kinds: ["artifact.created", "artifact.github_pr_opened"] },
  { category: "Permission Events", kinds: ["permission.requested", "permission.approved"] },
  { category: "System Events", kinds: ["system.relay_connected", "system.relay_disconnected"] },
];

export function CreateEventModal({ onEventCreated }: CreateEventModalProps) {
  const [open, setOpen] = useState(false);
  const [loading, setLoading] = useState(false);
  const [success, setSuccess] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Form state
  const [kind, setKind] = useState<string>("task.created");
  const [taskId, setTaskId] = useState<string>("");
  const [sessionId, setSessionId] = useState<string>("");
  const [dataJson, setDataJson] = useState<string>('{\n  "content": "Example event data"\n}');

  const resetForm = () => {
    setKind("task.created");
    setTaskId("");
    setSessionId("");
    setDataJson('{\n  "content": "Example event data"\n}');
    setError(null);
    setSuccess(false);
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError(null);
    setSuccess(false);

    try {
      // Parse JSON data
      let data: Record<string, any>;
      try {
        data = JSON.parse(dataJson);
      } catch (err) {
        throw new Error("Invalid JSON in data field");
      }

      // Emit event
      const result = await emitEvent({
        kind,
        data,
        task_id: taskId || undefined,
        session_id: sessionId || undefined,
      });

      setSuccess(true);
      onEventCreated?.(result.cursor);

      // Close modal after 1 second
      setTimeout(() => {
        setOpen(false);
        resetForm();
      }, 1000);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to create event");
    } finally {
      setLoading(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button className="gap-2">
          <Plus className="h-4 w-4" />
          Create Event
        </Button>
      </DialogTrigger>
      <DialogContent className="max-w-2xl max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>Create New Event</DialogTitle>
          <DialogDescription>
            Emit a new event to the event bus for testing or manual triggering
          </DialogDescription>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="space-y-4">
          {/* Event Kind */}
          <div className="space-y-2">
            <Label htmlFor="kind">Event Kind *</Label>
            <Select value={kind} onValueChange={setKind}>
              <SelectTrigger id="kind">
                <SelectValue placeholder="Select event kind" />
              </SelectTrigger>
              <SelectContent>
                {EVENT_KINDS.map((category) => (
                  <div key={category.category}>
                    <div className="px-2 py-1.5 text-xs font-semibold text-slate-500">
                      {category.category}
                    </div>
                    {category.kinds.map((k) => (
                      <SelectItem key={k} value={k}>
                        {k}
                      </SelectItem>
                    ))}
                  </div>
                ))}
              </SelectContent>
            </Select>
            <p className="text-xs text-slate-500">
              Choose from predefined event kinds or enter custom below
            </p>
          </div>

          {/* Custom Event Kind */}
          <div className="space-y-2">
            <Label htmlFor="custom-kind">Or Custom Event Kind</Label>
            <Input
              id="custom-kind"
              placeholder="e.g., custom.event.type"
              value={kind}
              onChange={(e) => setKind(e.target.value)}
            />
          </div>

          {/* Task ID (Optional) */}
          <div className="space-y-2">
            <Label htmlFor="task-id">Task ID (Optional)</Label>
            <Input
              id="task-id"
              placeholder="e.g., 550e8400-e29b-41d4-a716-446655440000"
              value={taskId}
              onChange={(e) => setTaskId(e.target.value)}
            />
            <p className="text-xs text-slate-500">
              Associate this event with a specific task
            </p>
          </div>

          {/* Session ID (Optional) */}
          <div className="space-y-2">
            <Label htmlFor="session-id">Session ID (Optional)</Label>
            <Input
              id="session-id"
              placeholder="e.g., 550e8400-e29b-41d4-a716-446655440001"
              value={sessionId}
              onChange={(e) => setSessionId(e.target.value)}
            />
            <p className="text-xs text-slate-500">
              Associate this event with a specific session
            </p>
          </div>

          {/* Event Data (JSON) */}
          <div className="space-y-2">
            <Label htmlFor="data">Event Data (JSON) *</Label>
            <Textarea
              id="data"
              placeholder='{"key": "value"}'
              value={dataJson}
              onChange={(e) => setDataJson(e.target.value)}
              rows={8}
              className="font-mono text-sm"
            />
            <p className="text-xs text-slate-500">
              JSON object containing event-specific data
            </p>
          </div>

          {/* Success Message */}
          {success && (
            <Alert className="border-green-200 bg-green-50">
              <CheckCircle2 className="h-4 w-4 text-green-600" />
              <AlertDescription className="text-green-800">
                Event created successfully! Check the event stream.
              </AlertDescription>
            </Alert>
          )}

          {/* Error Message */}
          {error && (
            <Alert variant="destructive">
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          )}

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => {
                setOpen(false);
                resetForm();
              }}
              disabled={loading}
            >
              Cancel
            </Button>
            <Button type="submit" disabled={loading || success}>
              {loading && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              {success ? "Created!" : "Create Event"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
