import { useState, useMemo } from "react";
import { ExternalLink, AlertCircle, Github } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Card } from "@/components/ui/card";
import type { Event } from "../hooks/useEventStream";

interface Artifact {
  id: string;
  type: string;
  url?: string;
  title?: string;
  data?: any;
}

interface ArtifactPreviewProps {
  events: Event[];
  onClose?: () => void;
}

export default function ArtifactPreview({ events }: ArtifactPreviewProps) {
  const [expandedArtifactIds, setExpandedArtifactIds] = useState<Set<string>>(
    new Set()
  );
  const [iframeErrors, setIframeErrors] = useState<Set<string>>(new Set());

  // Extract artifacts from events
  const artifacts = useMemo(() => {
    return events
      .filter((event) => event.kind.startsWith("artifact."))
      .map((event, index) => ({
        id: String(event.cursor) || `artifact-${index}`,
        type: (event.data as any)?.type || "unknown",
        url: (event.data as any)?.url,
        title: (event.data as any)?.title || "Untitled Artifact",
        data: (event.data as any)?.data,
      }));
  }, [events]);

  const toggleArtifactExpanded = (artifactId: string) => {
    setExpandedArtifactIds((prev) => {
      const newSet = new Set(prev);
      if (newSet.has(artifactId)) {
        newSet.delete(artifactId);
      } else {
        newSet.add(artifactId);
      }
      return newSet;
    });
  };

  const handleIframeError = (artifactId: string) => {
    setIframeErrors((prev) => new Set(prev).add(artifactId));
  };

  const renderArtifactContent = (artifact: Artifact, isExpanded: boolean) => {
    if (!isExpanded) return null;

    const hasError = iframeErrors.has(artifact.id);

    // GitHub PR - try iframe first
    if (artifact.type === "github_pr" && artifact.url && !hasError) {
      return (
        <div className="relative w-full h-[600px] mt-3">
          <iframe
            src={artifact.url}
            className="w-full h-full border-0 rounded"
            sandbox="allow-same-origin allow-scripts allow-popups allow-forms"
            title={artifact.title}
            onError={() => handleIframeError(artifact.id)}
          />
        </div>
      );
    }

    // Fallback for iframe errors or custom rendering
    if (hasError || artifact.type === "github_pr") {
      return (
        <div className="p-4 space-y-3 mt-3 bg-amber-50 rounded-lg border border-amber-200">
          <div className="flex items-center gap-2 text-amber-700">
            <AlertCircle className="h-4 w-4" />
            <span className="text-sm font-medium">
              Unable to load iframe preview
            </span>
          </div>
          <p className="text-sm text-amber-600">
            The artifact cannot be displayed in an embedded frame due to security restrictions.
          </p>
        </div>
      );
    }

    // Generic artifact rendering
    return (
      <div className="mt-3">
        <pre className="text-xs text-slate-600 bg-slate-50 p-4 rounded overflow-auto max-h-[400px]">
          {JSON.stringify(artifact.data || artifact, null, 2)}
        </pre>
      </div>
    );
  };

  if (artifacts.length === 0) {
    return (
      <div className="h-full flex items-center justify-center text-slate-400 text-sm p-6">
        <div className="text-center">
          <Github className="h-12 w-12 mx-auto mb-3 text-slate-300" />
          <p>No artifacts yet</p>
        </div>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col bg-slate-50">
      {/* Header */}
      <div className="flex items-center justify-between p-4 border-b border-slate-200 bg-white">
        <div className="flex items-center gap-2">
          <h2 className="text-sm font-semibold text-slate-700">Artifacts</h2>
          <Badge variant="secondary" className="text-xs">
            {artifacts.length}
          </Badge>
        </div>
      </div>

      {/* Artifacts List - Vertical Scrollable */}
      <div className="flex-1 overflow-y-auto p-4 space-y-3">
        {artifacts.map((artifact) => {
          const isExpanded = expandedArtifactIds.has(artifact.id);

          return (
            <Card
              key={artifact.id}
              className="bg-white shadow-sm hover:shadow-md transition-shadow"
            >
              <div className="p-4">
                {/* Artifact Header */}
                <div className="flex items-start justify-between gap-3">
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 mb-2">
                      <Badge
                        variant="outline"
                        className={
                          artifact.type === "github_pr"
                            ? "bg-purple-50 border-purple-300 text-purple-700"
                            : "bg-slate-50"
                        }
                      >
                        {artifact.type === "github_pr" ? (
                          <div className="flex items-center gap-1">
                            <Github className="h-3 w-3" />
                            <span>GitHub PR</span>
                          </div>
                        ) : (
                          artifact.type
                        )}
                      </Badge>
                    </div>
                    <h3 className="text-sm font-medium text-slate-900 break-words">
                      {artifact.title}
                    </h3>
                  </div>

                  {/* Action Buttons */}
                  <div className="flex items-center gap-1 flex-shrink-0">
                    {artifact.url && (
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => window.open(artifact.url, "_blank")}
                        className="gap-1.5 h-8"
                        title="Open in new tab"
                      >
                        <ExternalLink className="h-3.5 w-3.5" />
                        <span className="text-xs">Open</span>
                      </Button>
                    )}
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => toggleArtifactExpanded(artifact.id)}
                      className="h-8 text-xs"
                    >
                      {isExpanded ? "Collapse" : "Expand"}
                    </Button>
                  </div>
                </div>

                {/* Artifact URL */}
                {artifact.url && (
                  <div className="mt-2 text-xs text-slate-500 truncate">
                    {artifact.url}
                  </div>
                )}

                {/* Artifact Content */}
                {renderArtifactContent(artifact, isExpanded)}
              </div>
            </Card>
          );
        })}
      </div>
    </div>
  );
}
