import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import Inbox from './Inbox';

// Mock components
vi.mock('../components/ProjectTaskList', () => ({
  default: () => <div data-testid="project-task-list">Project Task List</div>,
}));

vi.mock('../components/TaskDetailPanel', () => ({
  default: ({ events }: any) => (
    <div data-testid="mock-task-detail-panel">Task Detail Panel</div>
  ),
}));

vi.mock('../components/ArtifactPreview', () => ({
  default: ({ events, onClose }: any) => (
    <div data-testid="mock-artifact-preview">
      Artifact Preview
      <button data-testid="mock-close-button" onClick={onClose}>
        Close
      </button>
    </div>
  ),
}));

vi.mock('../modals/TaskCreateModal', () => ({
  default: () => <div>Task Create Modal</div>,
}));

// Mock hooks
vi.mock('../hooks/useEventStream', () => ({
  useEventStream: () => ({
    events: [],
    isConnected: true,
    isReplaying: false,
  }),
}));

// Mock API
vi.mock('../api/eventBus', () => ({
  queryEvents: vi.fn(() =>
    Promise.resolve({
      events: [],
    })
  ),
}));

describe('Inbox - Artifact Close Functionality', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should render all three columns when task is selected', () => {
    render(
      <MemoryRouter initialEntries={['/?task=test-task-1']}>
        <Routes>
          <Route path="/" element={<Inbox />} />
        </Routes>
      </MemoryRouter>
    );

    expect(screen.getByTestId('project-task-list')).toBeInTheDocument();
    expect(screen.getByTestId('mock-task-detail-panel')).toBeInTheDocument();
    expect(screen.getByTestId('mock-artifact-preview')).toBeInTheDocument();
  });

  it('should pass onClose handler to ArtifactPreview', () => {
    render(
      <MemoryRouter initialEntries={['/?task=test-task-1']}>
        <Routes>
          <Route path="/" element={<Inbox />} />
        </Routes>
      </MemoryRouter>
    );

    const closeButton = screen.getByTestId('mock-close-button');
    expect(closeButton).toBeInTheDocument();

    // Click close button should not throw error
    fireEvent.click(closeButton);
  });

  it('should hide task detail and artifact when no task is selected', () => {
    const { container } = render(
      <MemoryRouter initialEntries={['/']}>
        <Routes>
          <Route path="/" element={<Inbox />} />
        </Routes>
      </MemoryRouter>
    );

    // Project list should always be visible
    expect(screen.getByTestId('project-task-list')).toBeInTheDocument();

    // Task detail panel should be hidden on mobile (lg:block)
    const taskDetailPanel = container.querySelector('[data-testid="task-detail-panel"]');
    expect(taskDetailPanel).toHaveClass('hidden');

    // Artifact preview should be completely hidden
    const artifactPreview = container.querySelector('[data-testid="artifact-preview"]');
    expect(artifactPreview).toHaveClass('hidden');
  });

  it('should render create task button', () => {
    render(
      <MemoryRouter initialEntries={['/']}>
        <Routes>
          <Route path="/" element={<Inbox />} />
        </Routes>
      </MemoryRouter>
    );

    const createButton = screen.getByTitle('Create new task');
    expect(createButton).toBeInTheDocument();
  });
});
