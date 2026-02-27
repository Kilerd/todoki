import { EventTimeline } from "@/components/EventTimeline";
import { Card } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Activity } from "lucide-react";
import { useState } from "react";

export default function EventTimelineDemo() {
  const [selectedTab, setSelectedTab] = useState("all");

  return (
    <div className="container mx-auto mt-12 max-w-6xl pb-12">
      <div className="mb-8">
        <div className="flex items-center gap-3 mb-2">
          <Activity className="h-6 w-6 text-slate-700" />
          <h1 className="text-2xl font-semibold text-slate-800">
            EventTimeline Component Demo
          </h1>
        </div>
        <p className="text-sm text-slate-500">
          Examples of EventTimeline component in different configurations
        </p>
      </div>

      <Tabs value={selectedTab} onValueChange={setSelectedTab} className="w-full">
        <TabsList className="grid w-full grid-cols-5">
          <TabsTrigger value="all">All Events</TabsTrigger>
          <TabsTrigger value="task">Task Events</TabsTrigger>
          <TabsTrigger value="agent">Agent Events</TabsTrigger>
          <TabsTrigger value="replay">Historical Replay</TabsTrigger>
          <TabsTrigger value="minimal">Minimal UI</TabsTrigger>
        </TabsList>

        {/* Tab 1: All Events */}
        <TabsContent value="all" className="space-y-4">
          <Card className="p-6">
            <h2 className="text-lg font-medium text-slate-800 mb-2">
              All Events (No Filter)
            </h2>
            <p className="text-sm text-slate-500 mb-4">
              Subscribe to all events with full status bar
            </p>
            <Separator className="mb-6" />

            <EventTimeline kinds={["*"]} showStatus={true} maxEvents={50} />
          </Card>

          <Card className="p-6 bg-slate-50">
            <h3 className="text-sm font-medium text-slate-700 mb-2">Usage:</h3>
            <pre className="text-xs bg-white p-4 rounded border border-slate-200 overflow-x-auto">
              {`<EventTimeline
  kinds={["*"]}
  showStatus={true}
  maxEvents={50}
/>`}
            </pre>
          </Card>
        </TabsContent>

        {/* Tab 2: Task Events */}
        <TabsContent value="task" className="space-y-4">
          <Card className="p-6">
            <h2 className="text-lg font-medium text-slate-800 mb-2">
              Task Events Only
            </h2>
            <p className="text-sm text-slate-500 mb-4">
              Filter for task-related events (task.*)
            </p>
            <Separator className="mb-6" />

            <EventTimeline kinds={["task.*"]} showStatus={true} maxEvents={30} />
          </Card>

          <Card className="p-6 bg-slate-50">
            <h3 className="text-sm font-medium text-slate-700 mb-2">Usage:</h3>
            <pre className="text-xs bg-white p-4 rounded border border-slate-200 overflow-x-auto">
              {`<EventTimeline
  kinds={["task.*"]}
  showStatus={true}
  maxEvents={30}
/>`}
            </pre>
            <p className="text-xs text-slate-500 mt-2">
              Matches: task.created, task.updated, task.completed, task.failed, etc.
            </p>
          </Card>
        </TabsContent>

        {/* Tab 3: Agent Events */}
        <TabsContent value="agent" className="space-y-4">
          <Card className="p-6">
            <h2 className="text-lg font-medium text-slate-800 mb-2">
              Agent Events Only
            </h2>
            <p className="text-sm text-slate-500 mb-4">
              Monitor agent lifecycle and analysis events
            </p>
            <Separator className="mb-6" />

            <EventTimeline
              kinds={["agent.*"]}
              showStatus={true}
              maxEvents={40}
            />
          </Card>

          <Card className="p-6 bg-slate-50">
            <h3 className="text-sm font-medium text-slate-700 mb-2">Usage:</h3>
            <pre className="text-xs bg-white p-4 rounded border border-slate-200 overflow-x-auto">
              {`<EventTimeline
  kinds={["agent.*"]}
  showStatus={true}
  maxEvents={40}
/>`}
            </pre>
            <p className="text-xs text-slate-500 mt-2">
              Matches: agent.started, agent.stopped, agent.requirement_analyzed, etc.
            </p>
          </Card>
        </TabsContent>

        {/* Tab 4: Historical Replay */}
        <TabsContent value="replay" className="space-y-4">
          <Card className="p-6">
            <h2 className="text-lg font-medium text-slate-800 mb-2">
              Historical Replay
            </h2>
            <p className="text-sm text-slate-500 mb-4">
              Replay events from cursor 0 (all historical events)
            </p>
            <Separator className="mb-6" />

            <EventTimeline
              kinds={["task.*", "agent.*"]}
              cursor={0}
              showStatus={true}
              maxEvents={100}
            />
          </Card>

          <Card className="p-6 bg-slate-50">
            <h3 className="text-sm font-medium text-slate-700 mb-2">Usage:</h3>
            <pre className="text-xs bg-white p-4 rounded border border-slate-200 overflow-x-auto">
              {`<EventTimeline
  kinds={["task.*", "agent.*"]}
  cursor={0}
  showStatus={true}
  maxEvents={100}
/>`}
            </pre>
            <p className="text-xs text-slate-500 mt-2">
              Server replays up to 1000 historical events, then switches to
              real-time streaming
            </p>
          </Card>
        </TabsContent>

        {/* Tab 5: Minimal UI */}
        <TabsContent value="minimal" className="space-y-4">
          <Card className="p-6">
            <h2 className="text-lg font-medium text-slate-800 mb-2">
              Minimal UI (No Status Bar)
            </h2>
            <p className="text-sm text-slate-500 mb-4">
              Compact view without connection status - ideal for embedding in
              other pages
            </p>
            <Separator className="mb-6" />

            <EventTimeline
              kinds={["task.*", "agent.*"]}
              showStatus={false}
              maxEvents={15}
            />
          </Card>

          <Card className="p-6 bg-slate-50">
            <h3 className="text-sm font-medium text-slate-700 mb-2">Usage:</h3>
            <pre className="text-xs bg-white p-4 rounded border border-slate-200 overflow-x-auto">
              {`<EventTimeline
  kinds={["task.*", "agent.*"]}
  showStatus={false}
  maxEvents={15}
/>`}
            </pre>
            <p className="text-xs text-slate-500 mt-2">
              Perfect for TaskDetail page or agent detail sidebar
            </p>
          </Card>
        </TabsContent>
      </Tabs>

      {/* Additional Examples */}
      <div className="mt-8 space-y-6">
        <h2 className="text-lg font-semibold text-slate-800">
          Additional Integration Examples
        </h2>

        {/* Example 1: Task-specific */}
        <Card className="p-6">
          <h3 className="text-md font-medium text-slate-700 mb-2">
            Task-specific Events
          </h3>
          <p className="text-sm text-slate-500 mb-4">
            Show events for a specific task (simulated with empty taskId)
          </p>
          <div className="bg-slate-50 p-4 rounded border border-slate-200 mb-4">
            <pre className="text-xs overflow-x-auto">
              {`// In TaskDetail.tsx
const { id } = useParams();

<EventTimeline
  kinds={['task.*', 'agent.*']}
  taskId={id}  // Filter by current task
  showStatus={false}
  maxEvents={20}
/>`}
            </pre>
          </div>
          <p className="text-xs text-slate-500">
            Only shows events where event.task_id matches the specified task ID
          </p>
        </Card>

        {/* Example 2: Agent-specific */}
        <Card className="p-6">
          <h3 className="text-md font-medium text-slate-700 mb-2">
            Agent-specific Events
          </h3>
          <p className="text-sm text-slate-500 mb-4">
            Monitor a specific agent's activity
          </p>
          <div className="bg-slate-50 p-4 rounded border border-slate-200 mb-4">
            <pre className="text-xs overflow-x-auto">
              {`// In AgentDetail.tsx
const { agentId } = useParams();

<EventTimeline
  kinds={['agent.*', 'task.*']}
  agentId={agentId}  // Filter by agent
  showStatus={true}
  maxEvents={30}
/>`}
            </pre>
          </div>
          <p className="text-xs text-slate-500">
            Only shows events where event.agent_id matches the specified agent
          </p>
        </Card>

        {/* Example 3: Custom token */}
        <Card className="p-6">
          <h3 className="text-md font-medium text-slate-700 mb-2">
            Custom Authentication Token
          </h3>
          <p className="text-sm text-slate-500 mb-4">
            Override default token from localStorage
          </p>
          <div className="bg-slate-50 p-4 rounded border border-slate-200 mb-4">
            <pre className="text-xs overflow-x-auto">
              {`const customToken = getCustomToken();

<EventTimeline
  kinds={['*']}
  token={customToken}  // Custom token
  showStatus={true}
  maxEvents={50}
/>`}
            </pre>
          </div>
          <p className="text-xs text-slate-500">
            By default, uses localStorage.getItem('token')
          </p>
        </Card>
      </div>
    </div>
  );
}
