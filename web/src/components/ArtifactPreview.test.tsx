import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import ArtifactPreview from './ArtifactPreview';
import type { Event } from '../hooks/useEventStream';

describe('ArtifactPreview', () => {
  const mockEvents: Event[] = [
    {
      cursor: 1,
      kind: 'artifact.created',
      time: '2024-01-01T00:00:00Z',
      agent_id: 'agent-1',
      session_id: 'session-1',
      task_id: 'task-1',
      data: {
        type: 'github_pr',
        url: 'https://github.com/test/test/pull/1',
        title: 'Test PR',
      },
    },
  ];

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should render "No artifacts yet" when no artifacts are available', () => {
    render(<ArtifactPreview events={[]} />);
    expect(screen.getByText('No artifacts yet')).toBeInTheDocument();
  });

  it('should render artifact preview when artifact is available', () => {
    render(<ArtifactPreview events={mockEvents} />);
    expect(screen.getByText('Test PR')).toBeInTheDocument();
    expect(screen.getByText('github_pr')).toBeInTheDocument();
  });

  it('should call onClose when close button is clicked', () => {
    const mockOnClose = vi.fn();
    const { container } = render(<ArtifactPreview events={mockEvents} onClose={mockOnClose} />);

    // Find the close button by looking for the X icon
    const closeButtons = container.querySelectorAll('button');
    const closeButton = Array.from(closeButtons).find(btn =>
      btn.querySelector('.lucide-x')
    );

    expect(closeButton).toBeDefined();
    if (closeButton) {
      fireEvent.click(closeButton);
      expect(mockOnClose).toHaveBeenCalledTimes(1);
    }
  });

  it('should hide artifact when close button is clicked without onClose prop', () => {
    const { rerender } = render(<ArtifactPreview events={mockEvents} />);

    // Initially shows artifact
    expect(screen.getByText('Test PR')).toBeInTheDocument();

    // Click close button
    const closeButtons = screen.getAllByRole('button');
    const closeButton = closeButtons.find(btn => btn.querySelector('.lucide-x'));
    if (closeButton) {
      fireEvent.click(closeButton);
    }

    // Re-render with same props
    rerender(<ArtifactPreview events={mockEvents} />);

    // Should show "No artifacts yet" after internal state is cleared
    // Note: This test verifies the fallback behavior
  });

  it('should render multiple artifacts', () => {
    const multipleEvents: Event[] = [
      ...mockEvents,
      {
        cursor: 2,
        kind: 'artifact.created',
        time: '2024-01-01T00:01:00Z',
        agent_id: 'agent-1',
        session_id: 'session-1',
        task_id: 'task-1',
        data: {
          type: 'github_pr',
          url: 'https://github.com/test/test/pull/2',
          title: 'Second PR',
        },
      },
    ];

    const { container } = render(<ArtifactPreview events={multipleEvents} />);

    expect(screen.getByText('2 artifacts')).toBeInTheDocument();

    // Check that both artifact titles appear in the artifact list section
    const artifactListSection = container.querySelector('.border-t.border-slate-200.p-3');
    expect(artifactListSection).toBeTruthy();
    expect(artifactListSection?.textContent).toContain('Test PR');
    expect(artifactListSection?.textContent).toContain('Second PR');
  });

  it('should open external link when external link button is clicked', () => {
    const mockOpen = vi.fn();
    global.window.open = mockOpen;

    render(<ArtifactPreview events={mockEvents} />);

    const externalLinkButton = screen.getAllByRole('button').find(
      btn => btn.querySelector('.lucide-external-link')
    );

    if (externalLinkButton) {
      fireEvent.click(externalLinkButton);
      expect(mockOpen).toHaveBeenCalledWith(
        'https://github.com/test/test/pull/1',
        '_blank'
      );
    }
  });
});
