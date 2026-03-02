import { describe, it, expect } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { ToolCallMessage } from './ToolCallMessage';
import type { ToolCallMessage as ToolCallData } from './types';

describe('ToolCallMessage', () => {
  const createToolCall = (
    id: string,
    title: string,
    status: 'pending' | 'completed' | 'error'
  ): ToolCallData => ({
    type: 'tool_call',
    id,
    kind: 'test_tool',
    title,
    status,
    raw_input: { param: 'value' },
  });

  it('should render nothing when tools array is empty', () => {
    const { container } = render(<ToolCallMessage tools={[]} />);
    expect(container.firstChild).toBeNull();
  });

  it('should render tool calls header with correct count', () => {
    const tools = [
      createToolCall('1', 'Tool 1', 'completed'),
      createToolCall('2', 'Tool 2', 'completed'),
    ];
    render(<ToolCallMessage tools={tools} />);

    expect(screen.getByText('Tool Calls')).toBeInTheDocument();
    // There are multiple "2" elements (in badges), so we use getAllByText
    const badges = screen.getAllByText('2');
    expect(badges.length).toBeGreaterThan(0);
  });

  it('should render active tools (pending, error) individually', () => {
    const tools = [
      createToolCall('1', 'Pending Tool', 'pending'),
      createToolCall('2', 'Error Tool', 'error'),
    ];
    render(<ToolCallMessage tools={tools} />);

    expect(screen.getByText('Pending Tool')).toBeInTheDocument();
    expect(screen.getByText('Error Tool')).toBeInTheDocument();
  });

  it('should aggregate completed tools into collapsible section', () => {
    const tools = [
      createToolCall('1', 'Completed Tool 1', 'completed'),
      createToolCall('2', 'Completed Tool 2', 'completed'),
      createToolCall('3', 'Completed Tool 3', 'completed'),
    ];
    render(<ToolCallMessage tools={tools} />);

    // Should show aggregated completed tools
    expect(screen.getByText('3 completed tool calls')).toBeInTheDocument();

    // Individual tool titles should not be visible initially (collapsed)
    expect(screen.queryByText('Completed Tool 1')).not.toBeInTheDocument();
  });

  it('should expand completed tools when clicked', () => {
    const tools = [
      createToolCall('1', 'Completed Tool 1', 'completed'),
      createToolCall('2', 'Completed Tool 2', 'completed'),
    ];
    render(<ToolCallMessage tools={tools} />);

    // Initially collapsed
    expect(screen.queryByText('Completed Tool 1')).not.toBeInTheDocument();

    // Click to expand
    const expandButton = screen.getByText('2 completed tool calls');
    fireEvent.click(expandButton);

    // Should show all completed tool titles
    expect(screen.getByText('Completed Tool 1')).toBeInTheDocument();
    expect(screen.getByText('Completed Tool 2')).toBeInTheDocument();
  });

  it('should show both active and completed tools correctly', () => {
    const tools = [
      createToolCall('1', 'Pending Tool', 'pending'),
      createToolCall('2', 'Completed Tool 1', 'completed'),
      createToolCall('3', 'Completed Tool 2', 'completed'),
      createToolCall('4', 'Error Tool', 'error'),
    ];
    render(<ToolCallMessage tools={tools} />);

    // Active tools should be visible
    expect(screen.getByText('Pending Tool')).toBeInTheDocument();
    expect(screen.getByText('Error Tool')).toBeInTheDocument();

    // Completed tools should be aggregated
    expect(screen.getByText('2 completed tool calls')).toBeInTheDocument();

    // Total count should be 4
    expect(screen.getByText('4')).toBeInTheDocument();
  });

  it('should deduplicate tools by id and keep latest state', () => {
    const tools = [
      createToolCall('1', 'Tool 1', 'pending'),
      createToolCall('1', 'Tool 1', 'completed'), // Same id, updated status
    ];
    render(<ToolCallMessage tools={tools} />);

    // Should only show 1 tool with completed status
    expect(screen.getByText('Tool Calls')).toBeInTheDocument();
    expect(screen.getByText('1 completed tool call')).toBeInTheDocument();
    // There are multiple "1" elements (in badges), so we use getAllByText
    const badges = screen.getAllByText('1');
    expect(badges.length).toBeGreaterThan(0);
  });

  it('should expand individual tool to show input', () => {
    const toolWithInput: ToolCallData = {
      type: 'tool_call',
      id: '1',
      kind: 'bash',
      title: 'Run Command',
      status: 'pending',
      raw_input: { command: 'ls -la', timeout: 5000 },
    };
    render(<ToolCallMessage tools={[toolWithInput]} />);

    // Initially input is not visible
    expect(screen.queryByText('Input')).not.toBeInTheDocument();

    // Click to expand
    const toolButton = screen.getByText('Run Command');
    fireEvent.click(toolButton);

    // Input should now be visible
    expect(screen.getByText('Input')).toBeInTheDocument();
    expect(screen.getByText(/"command": "ls -la"/)).toBeInTheDocument();
  });

  it('should render timestamp when provided', () => {
    const tools = [createToolCall('1', 'Tool 1', 'completed')];
    const timestamp = new Date('2024-01-01T12:00:00Z').getTime() * 1000000; // nanoseconds
    render(<ToolCallMessage tools={tools} timestamp={timestamp} />);

    // Check that timestamp is rendered (format depends on locale)
    const timeElement = screen.getByText(/\d{1,2}:\d{2}:\d{2}/);
    expect(timeElement).toBeInTheDocument();
  });

  it('should handle single completed tool correctly', () => {
    const tools = [createToolCall('1', 'Single Completed Tool', 'completed')];
    render(<ToolCallMessage tools={tools} />);

    // Should use singular form
    expect(screen.getByText('1 completed tool call')).toBeInTheDocument();
  });

  it('should not show completed tools section when all tools are active', () => {
    const tools = [
      createToolCall('1', 'Pending Tool 1', 'pending'),
      createToolCall('2', 'Error Tool', 'error'),
    ];
    render(<ToolCallMessage tools={tools} />);

    // Should not show completed tools section
    expect(screen.queryByText(/completed tool call/)).not.toBeInTheDocument();
  });

  it('should only show completed tools section when all tools are completed', () => {
    const tools = [
      createToolCall('1', 'Completed Tool 1', 'completed'),
      createToolCall('2', 'Completed Tool 2', 'completed'),
    ];
    render(<ToolCallMessage tools={tools} />);

    // Should only show completed tools section
    expect(screen.getByText('2 completed tool calls')).toBeInTheDocument();

    // No individual tool items should be visible outside the collapsed section
    const toolButtons = screen.queryAllByRole('button');
    // Should have 2 buttons: the completed tools collapsible and the Tool Calls header is not a button
    // Actually, let's count the actual expanded state
    expect(toolButtons.length).toBe(1); // Only the collapsible button
  });
});
