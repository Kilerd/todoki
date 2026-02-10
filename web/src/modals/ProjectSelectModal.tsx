import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { cn } from "@/lib/utils";
import { DialogProps } from "@radix-ui/react-dialog";
import { Check, Plus } from "lucide-react";
import { useState } from "react";
import { useProjects, createProject } from "../hooks/useProjects";
import type { Project } from "../api/types";

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

interface ProjectSelectModalProps extends DialogProps {
  mode: "select" | "select-or-create";
  suggestedName?: string;
  onSelect: (project: Project) => void;
}

export default function ProjectSelectModal({
  onOpenChange,
  mode,
  suggestedName,
  onSelect,
}: ProjectSelectModalProps) {
  const { projects, isLoading } = useProjects();
  const [showCreate, setShowCreate] = useState(mode === "select-or-create" && !!suggestedName);
  const [newName, setNewName] = useState(suggestedName || "");
  const [selectedColor, setSelectedColor] = useState(PRESET_COLORS[0]);
  const [isCreating, setIsCreating] = useState(false);

  const handleSelectProject = (project: Project) => {
    onSelect(project);
    onOpenChange?.(false);
  };

  const handleCreateProject = async () => {
    if (!newName.trim()) return;
    setIsCreating(true);
    try {
      const project = await createProject({
        name: newName.trim(),
        color: selectedColor,
      });
      onSelect(project);
      onOpenChange?.(false);
    } finally {
      setIsCreating(false);
    }
  };

  if (showCreate) {
    return (
      <div className="flex flex-col space-y-4">
        <div className="space-y-2">
          <Label htmlFor="project-name">Project Name</Label>
          <Input
            id="project-name"
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
            placeholder="Enter project name"
            autoFocus
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
                  selectedColor === color
                    ? "border-slate-900 scale-110"
                    : "border-transparent hover:scale-105"
                )}
                style={{ backgroundColor: color }}
                onClick={() => setSelectedColor(color)}
              >
                {selectedColor === color && (
                  <Check className="h-4 w-4 text-white mx-auto" />
                )}
              </button>
            ))}
          </div>
        </div>

        <div className="flex justify-between pt-2">
          <Button variant="ghost" onClick={() => setShowCreate(false)}>
            Back to list
          </Button>
          <Button
            onClick={handleCreateProject}
            disabled={!newName.trim() || isCreating}
          >
            {isCreating ? "Creating..." : "Create Project"}
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col space-y-4">
      <div className="text-sm text-slate-500 mb-2">
        Select a project for this task
      </div>

      {isLoading ? (
        <div className="text-center py-4 text-slate-400">Loading projects...</div>
      ) : (
        <div className="space-y-2 max-h-64 overflow-y-auto">
          {projects.map((project) => (
            <button
              key={project.id}
              type="button"
              className="w-full flex items-center gap-3 p-3 rounded-lg border border-slate-200 hover:border-slate-300 hover:bg-slate-50 transition-colors text-left"
              onClick={() => handleSelectProject(project)}
            >
              <div
                className="w-4 h-4 rounded-full shrink-0"
                style={{ backgroundColor: project.color }}
              />
              <span className="text-sm font-medium text-slate-700">
                {project.name}
              </span>
              {project.description && (
                <span className="text-xs text-slate-400 truncate">
                  {project.description}
                </span>
              )}
            </button>
          ))}
        </div>
      )}

      {mode === "select-or-create" && (
        <Button
          variant="outline"
          className="w-full"
          onClick={() => setShowCreate(true)}
        >
          <Plus className="h-4 w-4 mr-2" />
          Create New Project
        </Button>
      )}

      <div className="flex justify-end pt-2">
        <Button variant="ghost" onClick={() => onOpenChange?.(false)}>
          Cancel
        </Button>
      </div>
    </div>
  );
}
