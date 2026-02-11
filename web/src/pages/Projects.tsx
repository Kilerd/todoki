import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { Textarea } from "@/components/ui/textarea";
import { useToast } from "@/hooks/use-toast";
import { cn } from "@/lib/utils";
import {
  Archive,
  ArchiveRestore,
  Check,
  FileCode,
  MoreHorizontal,
  Pencil,
  Plus,
  Trash2,
} from "lucide-react";
import { useState } from "react";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import NavBar from "../components/NavBar";
import {
  useProjects,
  createProject,
  updateProject,
  deleteProject,
  type Project,
} from "../hooks/useProjects";

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

interface ProjectFormData {
  name: string;
  description: string;
  color: string;
}

type ProjectTemplates = Pick<Project, 'general_template' | 'business_template' | 'coding_template' | 'qa_template'>;

const DEFAULT_TEMPLATE = `# Task Execution

## Project: {{project_name}}
{{project_description}}

## Task
{{task_content}}

## Acceptance Criteria
- Complete the task as described
- Follow project conventions
- Test your changes before completion
`;

function ProjectForm({
  initialData,
  onSubmit,
  onCancel,
  submitLabel,
}: {
  initialData?: Partial<ProjectFormData>;
  onSubmit: (data: ProjectFormData) => Promise<void>;
  onCancel: () => void;
  submitLabel: string;
}) {
  const [name, setName] = useState(initialData?.name || "");
  const [description, setDescription] = useState(initialData?.description || "");
  const [color, setColor] = useState(initialData?.color || PRESET_COLORS[0]);
  const [isSubmitting, setIsSubmitting] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) return;
    setIsSubmitting(true);
    try {
      await onSubmit({ name: name.trim(), description: description.trim(), color });
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-4">
      <div className="space-y-2">
        <Label htmlFor="name">Name</Label>
        <Input
          id="name"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="Project name"
          autoFocus
        />
      </div>

      <div className="space-y-2">
        <Label htmlFor="description">Description (optional)</Label>
        <Input
          id="description"
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          placeholder="Brief description"
        />
      </div>

      <div className="space-y-2">
        <Label>Color</Label>
        <div className="flex flex-wrap gap-2">
          {PRESET_COLORS.map((c) => (
            <button
              key={c}
              type="button"
              className={cn(
                "w-8 h-8 rounded-full border-2 transition-all",
                color === c
                  ? "border-slate-900 scale-110"
                  : "border-transparent hover:scale-105"
              )}
              style={{ backgroundColor: c }}
              onClick={() => setColor(c)}
            >
              {color === c && <Check className="h-4 w-4 text-white mx-auto" />}
            </button>
          ))}
        </div>
      </div>

      <div className="flex justify-end gap-2 pt-2">
        <Button type="button" variant="ghost" onClick={onCancel}>
          Cancel
        </Button>
        <Button type="submit" disabled={!name.trim() || isSubmitting}>
          {isSubmitting ? "Saving..." : submitLabel}
        </Button>
      </div>
    </form>
  );
}

type TemplateTab = "general" | "business" | "coding" | "qa";

function TemplateEditor({
  templates,
  onSave,
  onCancel,
}: {
  templates: ProjectTemplates;
  onSave: (templates: ProjectTemplates) => Promise<void>;
  onCancel: () => void;
}) {
  const [activeTab, setActiveTab] = useState<TemplateTab>("coding");
  const [general, setGeneral] = useState(templates.general_template || "");
  const [business, setBusiness] = useState(templates.business_template || "");
  const [coding, setCoding] = useState(templates.coding_template || "");
  const [qa, setQa] = useState(templates.qa_template || "");
  const [isSubmitting, setIsSubmitting] = useState(false);

  const handleSave = async () => {
    setIsSubmitting(true);
    try {
      await onSave({
        general_template: general || undefined,
        business_template: business || undefined,
        coding_template: coding || undefined,
        qa_template: qa || undefined,
      });
    } finally {
      setIsSubmitting(false);
    }
  };

  const tabs: { key: TemplateTab; label: string }[] = [
    { key: "general", label: "General" },
    { key: "business", label: "Business" },
    { key: "coding", label: "Coding" },
    { key: "qa", label: "QA" },
  ];

  const getValue = (tab: TemplateTab) => {
    switch (tab) {
      case "general": return general;
      case "business": return business;
      case "coding": return coding;
      case "qa": return qa;
    }
  };

  const setValue = (tab: TemplateTab, value: string) => {
    switch (tab) {
      case "general": setGeneral(value); break;
      case "business": setBusiness(value); break;
      case "coding": setCoding(value); break;
      case "qa": setQa(value); break;
    }
  };

  return (
    <div className="space-y-4">
      <p className="text-sm text-slate-500">
        Templates are used when executing tasks on a relay. Available variables:{" "}
        <code className="text-xs bg-slate-100 px-1 rounded">{"{{task_content}}"}</code>,{" "}
        <code className="text-xs bg-slate-100 px-1 rounded">{"{{project_name}}"}</code>,{" "}
        <code className="text-xs bg-slate-100 px-1 rounded">{"{{project_description}}"}</code>
      </p>

      {/* Tab buttons */}
      <div className="flex gap-1 p-1 bg-slate-100 rounded-lg">
        {tabs.map((tab) => (
          <button
            key={tab.key}
            type="button"
            onClick={() => setActiveTab(tab.key)}
            className={cn(
              "flex-1 px-3 py-1.5 text-sm font-medium rounded-md transition-colors",
              activeTab === tab.key
                ? "bg-white text-slate-900 shadow-sm"
                : "text-slate-600 hover:text-slate-900"
            )}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {/* Template textarea */}
      <Textarea
        value={getValue(activeTab)}
        onChange={(e) => setValue(activeTab, e.target.value)}
        placeholder={DEFAULT_TEMPLATE}
        className="font-mono text-sm min-h-[300px]"
      />

      <div className="flex justify-end gap-2 pt-2">
        <Button type="button" variant="ghost" onClick={onCancel}>
          Cancel
        </Button>
        <Button onClick={handleSave} disabled={isSubmitting}>
          {isSubmitting ? "Saving..." : "Save Templates"}
        </Button>
      </div>
    </div>
  );
}

function ProjectCard({
  project,
  onEdit,
  onEditTemplates,
  onArchive,
  onDelete,
}: {
  project: Project;
  onEdit: () => void;
  onEditTemplates: () => void;
  onArchive: () => void;
  onDelete: () => void;
}) {
  return (
    <div
      className={cn(
        "flex items-center justify-between p-4 rounded-lg border border-slate-200 hover:border-slate-300 bg-white group transition-colors",
        project.archived && "opacity-60"
      )}
    >
      <div className="flex items-center gap-3 min-w-0">
        <div
          className="w-4 h-4 rounded-full shrink-0"
          style={{ backgroundColor: project.color }}
        />
        <div className="flex flex-col min-w-0">
          <span
            className={cn(
              "text-sm font-medium text-slate-700",
              project.archived && "line-through text-slate-400"
            )}
          >
            {project.name}
          </span>
          {project.description && (
            <span className="text-xs text-slate-400 truncate">
              {project.description}
            </span>
          )}
        </div>
      </div>

      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button
            variant="ghost"
            size="icon"
            className="h-8 w-8 opacity-0 group-hover:opacity-100 transition-opacity"
          >
            <MoreHorizontal className="h-4 w-4" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          <DropdownMenuItem onClick={onEdit}>
            <Pencil className="h-4 w-4 mr-2" />
            Edit
          </DropdownMenuItem>
          <DropdownMenuItem onClick={onEditTemplates}>
            <FileCode className="h-4 w-4 mr-2" />
            Edit Templates
          </DropdownMenuItem>
          <DropdownMenuItem onClick={onArchive}>
            {project.archived ? (
              <>
                <ArchiveRestore className="h-4 w-4 mr-2" />
                Unarchive
              </>
            ) : (
              <>
                <Archive className="h-4 w-4 mr-2" />
                Archive
              </>
            )}
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem onClick={onDelete} className="text-red-600">
            <Trash2 className="h-4 w-4 mr-2" />
            Delete
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );
}

function Projects() {
  const { toast } = useToast();
  const { projects, isLoading, refresh } = useProjects();
  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [editingProject, setEditingProject] = useState<Project | null>(null);
  const [editingTemplatesProject, setEditingTemplatesProject] = useState<Project | null>(null);
  const [showArchived, setShowArchived] = useState(false);

  const activeProjects = projects.filter((p) => !p.archived);
  const archivedProjects = projects.filter((p) => p.archived);

  const handleCreate = async (data: ProjectFormData) => {
    await createProject({
      name: data.name,
      description: data.description || undefined,
      color: data.color,
    });
    toast({ title: "Project created" });
    setShowCreateDialog(false);
    refresh();
  };

  const handleUpdate = async (data: ProjectFormData) => {
    if (!editingProject) return;
    await updateProject(editingProject.id, {
      name: data.name,
      description: data.description || undefined,
      color: data.color,
    });
    toast({ title: "Project updated" });
    setEditingProject(null);
    refresh();
  };

  const handleArchive = async (project: Project) => {
    await updateProject(project.id, { archived: !project.archived });
    toast({ title: project.archived ? "Project unarchived" : "Project archived" });
    refresh();
  };

  const handleUpdateTemplates = async (templates: ProjectTemplates) => {
    if (!editingTemplatesProject) return;
    await updateProject(editingTemplatesProject.id, templates);
    toast({ title: "Templates updated" });
    setEditingTemplatesProject(null);
    refresh();
  };

  const handleDelete = async (project: Project) => {
    try {
      await deleteProject(project.id);
      toast({ title: "Project deleted" });
      refresh();
    } catch {
      toast({
        title: "Cannot delete project",
        description: "This project has tasks. Archive it instead or move tasks first.",
        variant: "destructive",
      });
    }
  };

  return (
    <div className="container mx-auto mt-12 max-w-3xl">
      <NavBar />

      <div className="mt-8 space-y-6">
        <div className="flex items-center justify-between">
          <h2 className="text-lg font-medium text-slate-700">Projects</h2>
          <Button onClick={() => setShowCreateDialog(true)}>
            <Plus className="h-4 w-4 mr-2" />
            New Project
          </Button>
        </div>

        {isLoading ? (
          <div className="space-y-3">
            <Skeleton className="h-16 w-full" />
            <Skeleton className="h-16 w-full" />
            <Skeleton className="h-16 w-full" />
          </div>
        ) : (
          <>
            <div className="space-y-2">
              {activeProjects.map((project) => (
                <ProjectCard
                  key={project.id}
                  project={project}
                  onEdit={() => setEditingProject(project)}
                  onEditTemplates={() => setEditingTemplatesProject(project)}
                  onArchive={() => handleArchive(project)}
                  onDelete={() => handleDelete(project)}
                />
              ))}
              {activeProjects.length === 0 && (
                <div className="text-center py-8 text-slate-400">
                  No projects yet. Create one to get started.
                </div>
              )}
            </div>

            {archivedProjects.length > 0 && (
              <div className="space-y-2">
                <button
                  type="button"
                  className="text-sm text-slate-500 hover:text-slate-700"
                  onClick={() => setShowArchived(!showArchived)}
                >
                  {showArchived ? "Hide" : "Show"} archived ({archivedProjects.length})
                </button>
                {showArchived &&
                  archivedProjects.map((project) => (
                    <ProjectCard
                      key={project.id}
                      project={project}
                      onEdit={() => setEditingProject(project)}
                      onEditTemplates={() => setEditingTemplatesProject(project)}
                      onArchive={() => handleArchive(project)}
                      onDelete={() => handleDelete(project)}
                    />
                  ))}
              </div>
            )}
          </>
        )}
      </div>

      {/* Create Dialog */}
      <Dialog open={showCreateDialog} onOpenChange={setShowCreateDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Create Project</DialogTitle>
          </DialogHeader>
          <ProjectForm
            onSubmit={handleCreate}
            onCancel={() => setShowCreateDialog(false)}
            submitLabel="Create"
          />
        </DialogContent>
      </Dialog>

      {/* Edit Dialog */}
      <Dialog
        open={!!editingProject}
        onOpenChange={(open) => !open && setEditingProject(null)}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Edit Project</DialogTitle>
          </DialogHeader>
          {editingProject && (
            <ProjectForm
              initialData={{
                name: editingProject.name,
                description: editingProject.description || "",
                color: editingProject.color,
              }}
              onSubmit={handleUpdate}
              onCancel={() => setEditingProject(null)}
              submitLabel="Save"
            />
          )}
        </DialogContent>
      </Dialog>

      {/* Templates Dialog */}
      <Dialog
        open={!!editingTemplatesProject}
        onOpenChange={(open) => !open && setEditingTemplatesProject(null)}
      >
        <DialogContent className="max-w-3xl">
          <DialogHeader>
            <DialogTitle>
              Edit Templates - {editingTemplatesProject?.name}
            </DialogTitle>
          </DialogHeader>
          {editingTemplatesProject && (
            <TemplateEditor
              templates={{
                general_template: editingTemplatesProject.general_template,
                business_template: editingTemplatesProject.business_template,
                coding_template: editingTemplatesProject.coding_template,
                qa_template: editingTemplatesProject.qa_template,
              }}
              onSave={handleUpdateTemplates}
              onCancel={() => setEditingTemplatesProject(null)}
            />
          )}
        </DialogContent>
      </Dialog>
    </div>
  );
}

export default Projects;
