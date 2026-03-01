import { useState, useEffect, useMemo } from "react";
import { useSearchParams } from "react-router-dom";
import { X, ExternalLink, AlertCircle } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { useEventStream } from "../hooks/useEventStream";

interface Artifact {
  id: string;
  type: string;
  url?: string;
  title?: string;
  data?: any;
}

export default function ArtifactPreview() {
  const [searchParams, setSearchParams] = useSearchParams();
  const selectedTaskId = searchParams.get("task");
  const [artifacts, setArtifacts] = useState<Artifact[]>([]);
  const [selectedArtifact, setSelectedArtifact] = useState<Artifact | null>(
    null
  );
  const [iframeError, setIframeError] = useState(false);

  // Subscribe to artifact events for the selected task
  const { events, isConnected } = useEventStream({
    kinds: ["artifact.*"],
    taskId: selectedTaskId || undefined,
    enabled: !!selectedTaskId,
  });

  // Extract artifacts from events
  useEffect(() => {
    if (!events || events.length === 0) return;

    const newArtifacts = events
      .filter((event) => event.kind.startsWith("artifact."))
      .map((event, index) => ({
        id: String(event.cursor) || `artifact-${index}`,
        type: event.data?.type || "unknown",
        url: event.data?.url,
        title: event.data?.title || "Untitled Artifact",
        data: event.data?.data,
      }));

    setArtifacts(newArtifacts);

    // Auto-select the latest artifact
    if (newArtifacts.length > 0 && !selectedArtifact) {
      setSelectedArtifact(newArtifacts[newArtifacts.length - 1]);
    }
  }, [events]);

  const handleClose = () => {
    setSelectedArtifact(null);
  };

  const renderArtifactContent = () => {
    if (!selectedArtifact) return null;

    // GitHub PR - try iframe first
    if (
      selectedArtifact.type === "github_pr" &&
      selectedArtifact.url &&
      !iframeError
    ) {
      return (
        <div className="relative w-full h-full">
          <iframe
            src={selectedArtifact.url}
            className="w-full h-full border-0"
            sandbox="allow-same-origin allow-scripts allow-popups allow-forms"
            title={selectedArtifact.title}
            onError={() => setIframeError(true)}
          />
        </div>
      );
    }

    // Fallback for iframe errors or custom rendering
    if (iframeError || selectedArtifact.type === "github_pr") {
      return (
        <div className="p-6 space-y-4">
          <div className="flex items-center gap-2 text-amber-600">
            <AlertCircle className="h-5 w-5" />
            <span className="text-sm font-medium">
              Unable to load iframe preview
            </span>
          </div>
          <p className="text-sm text-slate-600">
            The artifact cannot be displayed in an embedded frame. You can open
            it directly:
          </p>
          {selectedArtifact.url && (
            <Button
              variant="outline"
              size="sm"
              onClick={() => window.open(selectedArtifact.url, "_blank")}
              className="gap-2"
            >
              <ExternalLink className="h-4 w-4" />
              Open in new tab
            </Button>
          )}
        </div>
      );
    }

    // Generic artifact rendering
    return (
      <div className="p-6">
        <pre className="text-xs text-slate-600 bg-slate-50 p-4 rounded overflow-auto">
          {JSON.stringify(selectedArtifact, null, 2)}
        </pre>
      </div>
    );
  };

  if (!selectedTaskId) {
    return null;
  }

  if (!selectedArtifact) {
    return (
      <div className="h-full flex items-center justify-center text-slate-400 text-sm">
        No artifacts yet
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col bg-white">
      {/* Header */}
      <div className="flex items-center justify-between p-4 border-b border-slate-200">
        <div className="flex items-center gap-3 flex-1 min-w-0">
          <Badge variant="outline" className="flex-shrink-0">
            {selectedArtifact.type}
          </Badge>
          <h3 className="text-sm font-medium text-slate-700 truncate">
            {selectedArtifact.title}
          </h3>
        </div>
        <div className="flex items-center gap-1 flex-shrink-0">
          {selectedArtifact.url && (
            <Button
              variant="ghost"
              size="icon"
              onClick={() => window.open(selectedArtifact.url, "_blank")}
            >
              <ExternalLink className="h-4 w-4" />
            </Button>
          )}
          <Button variant="ghost" size="icon" onClick={handleClose}>
            <X className="h-4 w-4" />
          </Button>
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto">{renderArtifactContent()}</div>

      {/* Artifact List (if multiple) */}
      {artifacts.length > 1 && (
        <div className="border-t border-slate-200 p-3">
          <div className="text-xs text-slate-500 mb-2">
            {artifacts.length} artifacts
          </div>
          <div className="flex gap-2 overflow-x-auto">
            {artifacts.map((artifact) => (
              <Button
                key={artifact.id}
                variant="outline"
                size="sm"
                className={
                  selectedArtifact?.id === artifact.id
                    ? "bg-teal-50 border-teal-500"
                    : ""
                }
                onClick={() => {
                  setSelectedArtifact(artifact);
                  setIframeError(false);
                }}
              >
                <span className="text-xs truncate max-w-[150px]">
                  {artifact.title}
                </span>
              </Button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
