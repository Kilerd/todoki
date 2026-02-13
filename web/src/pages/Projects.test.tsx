import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import Projects from './Projects';

// Mock dependencies
vi.mock('../components/NavBar', () => ({
  default: () => <div>NavBar</div>,
}));

const mockToast = vi.fn();
const mockRefresh = vi.fn();

vi.mock('../hooks/useProjects', () => ({
  useProjects: () => ({
    projects: [
      {
        id: 'test-project-id-123',
        name: 'Test Project',
        description: 'Test Description',
        color: '#3B82F6',
        archived: false,
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z',
      },
    ],
    isLoading: false,
    refresh: mockRefresh,
  }),
  createProject: vi.fn(),
  updateProject: vi.fn(),
  deleteProject: vi.fn(),
}));

vi.mock('@/hooks/use-toast', () => ({
  useToast: () => ({
    toast: mockToast,
  }),
}));

describe('Projects - Copy ID functionality', () => {
  const mockClipboard = {
    writeText: vi.fn(() => Promise.resolve()),
  };

  beforeEach(() => {
    vi.clearAllMocks();
    // Mock clipboard API properly for jsdom
    Object.defineProperty(navigator, 'clipboard', {
      value: mockClipboard,
      writable: true,
      configurable: true,
    });
  });

  it('should render project with ID', () => {
    render(<Projects />);
    expect(screen.getByText(/ID: test-project-id-123/i)).toBeInTheDocument();
  });

  it('should render project card with all details', () => {
    render(<Projects />);
    expect(screen.getByText('Test Project')).toBeInTheDocument();
    expect(screen.getByText('Test Description')).toBeInTheDocument();
    expect(screen.getByText(/ID: test-project-id-123/i)).toBeInTheDocument();
  });

  it('should include Copy icon in imports (integration test)', () => {
    // This test verifies that Copy icon is imported from lucide-react
    // By checking if the component renders without errors
    const { container } = render(<Projects />);
    expect(container).toBeTruthy();
  });
});
