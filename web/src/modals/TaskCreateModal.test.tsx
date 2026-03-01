import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import TaskCreateModal from './TaskCreateModal';

// Mock functions must be declared before vi.mock calls
const createProjectMock = vi.fn();
const createTaskMock = vi.fn();

// Mock dependencies
vi.mock('../hooks/useProjects', () => ({
  useProjects: () => ({
    projects: [
      {
        id: 'project-1',
        name: 'Project Alpha',
        color: '#3B82F6',
        description: null,
        archived: false,
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z',
      },
      {
        id: 'project-2',
        name: 'Project Beta',
        color: '#10B981',
        description: null,
        archived: false,
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z',
      },
    ],
    isLoading: false,
  }),
  createProject: (...args: unknown[]) => createProjectMock(...args),
}));

vi.mock('../hooks/useTasks', () => ({
  createTask: (...args: unknown[]) => createTaskMock(...args),
}));

const mockOnSuccess = vi.fn();
const mockOnOpenChange = vi.fn();

describe('TaskCreateModal', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    createProjectMock.mockResolvedValue({
      id: 'new-project-id',
      name: 'New Project',
      color: '#3B82F6',
    });
    createTaskMock.mockResolvedValue({});
  });

  it('should render task creation form', () => {
    render(
      <TaskCreateModal
        open={true}
        onOpenChange={mockOnOpenChange}
        onSuccess={mockOnSuccess}
      />
    );

    expect(screen.getByLabelText(/task content/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/priority/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/project/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /create task/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /cancel/i })).toBeInTheDocument();
  });

  it('should display project list in select dropdown', () => {
    render(
      <TaskCreateModal
        open={true}
        onOpenChange={mockOnOpenChange}
        onSuccess={mockOnSuccess}
      />
    );

    // Projects should be available in the form
    const projectInput = screen.getByLabelText(/project/i);
    expect(projectInput).toBeInTheDocument();
  });

  it('should disable project selection when projectId is provided', () => {
    render(
      <TaskCreateModal
        open={true}
        onOpenChange={mockOnOpenChange}
        onSuccess={mockOnSuccess}
        projectId="project-1"
      />
    );

    // Find the select trigger button by the project label
    const projectLabel = screen.getByLabelText(/project/i);
    expect(projectLabel).toBeInTheDocument();

    // Find the disabled button within the select component
    const selectTrigger = document.querySelector('#task-project');
    expect(selectTrigger).toBeDisabled();
  });

  it('should create task with valid input', async () => {
    const user = userEvent.setup();

    render(
      <TaskCreateModal
        open={true}
        onOpenChange={mockOnOpenChange}
        onSuccess={mockOnSuccess}
        projectId="project-1"
      />
    );

    const contentInput = screen.getByLabelText(/task content/i);
    await user.type(contentInput, 'Test Task Content');

    const createButton = screen.getByRole('button', { name: /create task/i });
    await user.click(createButton);

    await waitFor(() => {
      expect(createTaskMock).toHaveBeenCalledWith({
        content: 'Test Task Content',
        priority: 1,
        project_id: 'project-1',
        status: 'todo',
      });
    });

    expect(mockOnSuccess).toHaveBeenCalled();
    expect(mockOnOpenChange).toHaveBeenCalledWith(false);
  });

  it('should not submit with empty task content', async () => {
    const user = userEvent.setup();

    render(
      <TaskCreateModal
        open={true}
        onOpenChange={mockOnOpenChange}
        onSuccess={mockOnSuccess}
        projectId="project-1"
      />
    );

    const createButton = screen.getByRole('button', { name: /create task/i });
    await user.click(createButton);

    expect(createTaskMock).not.toHaveBeenCalled();
    expect(mockOnSuccess).not.toHaveBeenCalled();
  });

  it('should show create project form when clicking create new project', async () => {
    const user = userEvent.setup();

    render(
      <TaskCreateModal
        open={true}
        onOpenChange={mockOnOpenChange}
        onSuccess={mockOnSuccess}
      />
    );

    const createProjectButton = screen.getByRole('button', { name: /create new project/i });
    await user.click(createProjectButton);

    await waitFor(() => {
      expect(screen.getByLabelText(/project name/i)).toBeInTheDocument();
      expect(screen.getByText(/color/i)).toBeInTheDocument();
    });
  });

  it('should create new project and select it', async () => {
    const user = userEvent.setup();

    render(
      <TaskCreateModal
        open={true}
        onOpenChange={mockOnOpenChange}
        onSuccess={mockOnSuccess}
      />
    );

    // Navigate to project creation form
    const createProjectButton = screen.getByRole('button', { name: /create new project/i });
    await user.click(createProjectButton);

    // Fill in project details
    const projectNameInput = screen.getByLabelText(/project name/i);
    await user.type(projectNameInput, 'New Project');

    const createButton = screen.getByRole('button', { name: /create project/i });
    await user.click(createButton);

    await waitFor(() => {
      expect(createProjectMock).toHaveBeenCalledWith({
        name: 'New Project',
        color: '#3B82F6',
      });
    });
  });

  it('should display keyboard shortcut hint', () => {
    render(
      <TaskCreateModal
        open={true}
        onOpenChange={mockOnOpenChange}
        onSuccess={mockOnSuccess}
      />
    );

    const hint = screen.getByText(/press.*enter to create/i);
    expect(hint).toBeInTheDocument();
  });

  it('should cancel and close modal', async () => {
    const user = userEvent.setup();

    render(
      <TaskCreateModal
        open={true}
        onOpenChange={mockOnOpenChange}
        onSuccess={mockOnSuccess}
      />
    );

    const cancelButton = screen.getByRole('button', { name: /cancel/i });
    await user.click(cancelButton);

    expect(mockOnOpenChange).toHaveBeenCalledWith(false);
    expect(createTaskMock).not.toHaveBeenCalled();
  });

  it('should disable submit button while creating task', async () => {
    const user = userEvent.setup();
    createTaskMock.mockImplementation(() => new Promise(resolve => setTimeout(resolve, 100)));

    render(
      <TaskCreateModal
        open={true}
        onOpenChange={mockOnOpenChange}
        onSuccess={mockOnSuccess}
        projectId="project-1"
      />
    );

    const contentInput = screen.getByLabelText(/task content/i);
    await user.type(contentInput, 'Test Task');

    const createButton = screen.getByRole('button', { name: /create task/i });
    await user.click(createButton);

    // Button should show "Creating..." and be disabled
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /creating/i })).toBeDisabled();
    });
  });
});
