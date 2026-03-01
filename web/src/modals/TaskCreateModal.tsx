import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { cn } from "@/lib/utils";
import { DialogProps } from "@radix-ui/react-dialog";
import { Check, Plus } from "lucide-react";
import { useState, useEffect } from "react";
import { useProjects, createProject } from "../hooks/useProjects";
import { createTask } from "../hooks/useTasks";

const PRESET_COLORS = [
  "#3B82F6", // blue
  "#10B981", // emerald
  "#F59E0B", // amber
  "#EF4444", // red
  "#8B5CF6", // violet
  "#EC4899", // pink
  "#6B7280", // gray
  "#14B8A6", // teal
];

const PRIORITY_OPTIONS = [
  { value: 0, label: "Low", emoji: "⬇️" },
  { value: 1, label: "Normal", emoji: "➡️" },
  { value: 2, label: "High", emoji: "⬆️" },
  { value: 3, label: "Urgent", emoji: "🔥" },
];

interface TaskCreateModalProps extends DialogProps {
  projectId?: string;
  onSuccess?: () => void;
}

export default function TaskCreateModal({
  projectId,
  onOpenChange,
  onSuccess,
}: TaskCreateModalProps) {
  const { projects, isLoading: isProjectsLoading } = useProjects();
  const [showCreateProject, setShowCreateProject] = useState(false);

  // Task form state
  const [taskContent, setTaskContent] = useState("");
  const [taskPriority, setTaskPriority] = useState(1);
  const [selectedProjectId, setSelectedProjectId] = useState(projectId || "");
  const [isSubmitting, setIsSubmitting] = useState(false);

  // Project creation state
  const [newProjectName, setNewProjectName] = useState("");
  const [newProjectColor, setNewProjectColor] = useState(PRESET_COLORS[0]);
  const [isCreatingProject, setIsCreatingProject] = useState(false);

  // Set initial project if provided
  useEffect(() => {
    if (projectId) {
      setSelectedProjectId(projectId);
    }
  }, [projectId]);

  const handleCreateTask = async () => {
    if (!taskContent.trim() || !selectedProjectId) return;

    setIsSubmitting(true);
    try {
      await createTask({
        content: taskContent.trim(),
        priority: taskPriority,
        project_id: selectedProjectId,
        status: "todo",
      });

      // Reset form
      setTaskContent("");
      setTaskPriority(1);
      if (!projectId) {
        setSelectedProjectId("");
      }

      onSuccess?.();
      onOpenChange?.(false);
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleCreateProject = async () => {
    if (!newProjectName.trim()) return;

    setIsCreatingProject(true);
    try {
      const project = await createProject({
        name: newProjectName.trim(),
        color: newProjectColor,
      });

      setSelectedProjectId(project.id);
      setShowCreateProject(false);
      setNewProjectName("");
      setNewProjectColor(PRESET_COLORS[0]);
    } finally {
      setIsCreatingProject(false);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
      e.preventDefault();
      handleCreateTask();
    }
  };

  if (showCreateProject) {
    return (
      <div className="flex flex-col space-y-4">
        <div className="space-y-2">
          <Label htmlFor="new-project-name">Project Name</Label>
          <Input
            id="new-project-name"
            value={newProjectName}
            onChange={(e) => setNewProjectName(e.target.value)}
            placeholder="Enter project name"
            autoFocus
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                handleCreateProject();
              }
            }}
          />
        </div>

        <div className="space-y-2">
          <Label>Color</Label>
          <div className="flex flex-wrap gap-2">
            {PRESET_COLORS.map((color) => (
              <button
                key={color}
                type="button"
                className={cn(
                  "w-8 h-8 rounded-full border-2 transition-all",
                  newProjectColor === color
                    ? "border-slate-900 scale-110"
                    : "border-transparent hover:scale-105"
                )}
                style={{ backgroundColor: color }}
                onClick={() => setNewProjectColor(color)}
              >
                {newProjectColor === color && (
                  <Check className="h-4 w-4 text-white mx-auto" />
                )}
              </button>
            ))}
          </div>
        </div>

        <div className="flex justify-between pt-2">
          <Button
            variant="ghost"
            onClick={() => {
              setShowCreateProject(false);
              setNewProjectName("");
            }}
          >
            Back
          </Button>
          <Button
            onClick={handleCreateProject}
            disabled={!newProjectName.trim() || isCreatingProject}
          >
            {isCreatingProject ? "Creating..." : "Create Project"}
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col space-y-4">
      <div className="space-y-2">
        <Label htmlFor="task-content">Task Content *</Label>
        <Textarea
          id="task-content"
          value={taskContent}
          onChange={(e) => setTaskContent(e.target.value)}
          placeholder="What needs to be done?"
          autoFocus
          rows={3}
          onKeyDown={handleKeyDown}
          className="resize-none"
        />
        <p className="text-xs text-slate-500">
          Press {navigator.platform.includes("Mac") ? "⌘" : "Ctrl"}+Enter to
          create
        </p>
      </div>

      <div className="space-y-2">
        <Label htmlFor="task-priority">Priority</Label>
        <Select
          value={taskPriority.toString()}
          onValueChange={(value) => setTaskPriority(parseInt(value))}
        >
          <SelectTrigger id="task-priority">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {PRIORITY_OPTIONS.map((option) => (
              <SelectItem key={option.value} value={option.value.toString()}>
                <span className="flex items-center gap-2">
                  <span>{option.emoji}</span>
                  <span>{option.label}</span>
                </span>
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      <div className="space-y-2">
        <Label htmlFor="task-project">Project *</Label>
        {isProjectsLoading ? (
          <div className="text-sm text-slate-400">Loading projects...</div>
        ) : (
          <div className="space-y-2">
            <Select
              value={selectedProjectId}
              onValueChange={setSelectedProjectId}
            >
              <SelectTrigger id="task-project" disabled={!!projectId}>
                <SelectValue placeholder="Select a project" />
              </SelectTrigger>
              <SelectContent>
                {projects.map((project) => (
                  <SelectItem key={project.id} value={project.id}>
                    <div className="flex items-center gap-2">
                      <div
                        className="w-3 h-3 rounded-full shrink-0"
                        style={{ backgroundColor: project.color }}
                      />
                      <span>{project.name}</span>
                    </div>
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            {!projectId && (
              <Button
                variant="outline"
                size="sm"
                className="w-full"
                onClick={() => setShowCreateProject(true)}
              >
                <Plus className="h-3.5 w-3.5 mr-2" />
                Create New Project
              </Button>
            )}
          </div>
        )}
      </div>

      <div className="flex justify-end gap-2 pt-2">
        <Button variant="ghost" onClick={() => onOpenChange?.(false)}>
          Cancel
        </Button>
        <Button
          onClick={handleCreateTask}
          disabled={!taskContent.trim() || !selectedProjectId || isSubmitting}
        >
          {isSubmitting ? "Creating..." : "Create Task"}
        </Button>
      </div>
    </div>
  );
}
