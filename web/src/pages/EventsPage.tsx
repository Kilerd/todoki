import { EventTimeline } from "@/components/EventTimeline";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Activity } from "lucide-react";
import { useState } from "react";

export default function EventsPage() {
  const [filterKinds, setFilterKinds] = useState<string[]>(["*"]);
  const [filterInput, setFilterInput] = useState("*");
  const [cursor, setCursor] = useState<number | undefined>(undefined);
  const [cursorInput, setCursorInput] = useState("");

  const handleApplyKindsFilter = () => {
    const kinds = filterInput
      .split(",")
      .map((k) => k.trim())
      .filter((k) => k.length > 0);
    setFilterKinds(kinds.length > 0 ? kinds : ["*"]);
  };

  const handleApplyCursor = () => {
    const parsed = parseInt(cursorInput, 10);
    setCursor(isNaN(parsed) ? undefined : parsed);
  };

  const handleResetCursor = () => {
    setCursor(undefined);
    setCursorInput("");
  };

  const commonPatterns = [
    { label: "All Events", value: "*" },
    { label: "Task Events", value: "task.*" },
    { label: "Agent Events", value: "agent.*" },
    { label: "Artifacts", value: "artifact.*" },
    { label: "Permissions", value: "permission.*" },
    { label: "System", value: "system.*" },
  ];

  return (
    <div className="container mx-auto mt-12 max-w-5xl pb-12">
      <div className="mb-8">
        <div className="flex items-center gap-3 mb-2">
          <Activity className="h-6 w-6 text-slate-700" />
          <h1 className="text-2xl font-semibold text-slate-800">Event Stream</h1>
        </div>
        <p className="text-sm text-slate-500">
          Real-time events from the Todoki event bus
        </p>
      </div>

      {/* Filters */}
      <Card className="p-6 mb-6">
        <h2 className="text-sm font-medium text-slate-700 mb-4">Filters</h2>

        {/* Quick Patterns */}
        <div className="mb-4">
          <Label className="text-xs text-slate-500 mb-2 block">
            Quick Patterns
          </Label>
          <div className="flex flex-wrap gap-2">
            {commonPatterns.map((pattern) => (
              <Badge
                key={pattern.value}
                variant="outline"
                className="cursor-pointer hover:bg-slate-100"
                onClick={() => {
                  setFilterInput(pattern.value);
                  setFilterKinds([pattern.value]);
                }}
              >
                {pattern.label}
              </Badge>
            ))}
          </div>
        </div>

        {/* Custom Kinds Filter */}
        <div className="mb-4">
          <Label htmlFor="kinds-filter" className="text-xs text-slate-500 mb-2 block">
            Event Kinds (comma-separated, supports wildcards)
          </Label>
          <div className="flex gap-2">
            <Input
              id="kinds-filter"
              value={filterInput}
              onChange={(e) => setFilterInput(e.target.value)}
              placeholder="e.g., task.created, agent.*, system.*"
              className="flex-1"
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  handleApplyKindsFilter();
                }
              }}
            />
            <Button onClick={handleApplyKindsFilter} className="cursor-pointer">
              Apply
            </Button>
          </div>
          <p className="text-xs text-slate-400 mt-1">
            Current: {filterKinds.join(", ")}
          </p>
        </div>

        {/* Cursor Filter */}
        <div>
          <Label htmlFor="cursor-filter" className="text-xs text-slate-500 mb-2 block">
            Starting Cursor (for historical replay)
          </Label>
          <div className="flex gap-2">
            <Input
              id="cursor-filter"
              value={cursorInput}
              onChange={(e) => setCursorInput(e.target.value)}
              placeholder="e.g., 100"
              className="flex-1"
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  handleApplyCursor();
                }
              }}
            />
            <Button onClick={handleApplyCursor} className="cursor-pointer">
              Apply
            </Button>
            {cursor !== undefined && (
              <Button
                variant="outline"
                onClick={handleResetCursor}
                className="cursor-pointer"
              >
                Reset
              </Button>
            )}
          </div>
          {cursor !== undefined && (
            <p className="text-xs text-slate-400 mt-1">
              Replaying from cursor: {cursor}
            </p>
          )}
        </div>
      </Card>

      {/* Event Timeline */}
      <EventTimeline
        kinds={filterKinds}
        cursor={cursor}
        showStatus={true}
        maxEvents={100}
      />
    </div>
  );
}
