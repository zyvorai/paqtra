import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import ErrorBoundary from '../components/ErrorBoundary';

const ThrowingComponent = () => {
  throw new Error('Test error');
};

const WorkingComponent = () => <div>Works fine</div>;

describe('ErrorBoundary', () => {
  it('renders children when no error', () => {
    render(
      <ErrorBoundary>
        <WorkingComponent />
      </ErrorBoundary>
    );
    expect(screen.getByText('Works fine')).toBeDefined();
  });

  it('renders error UI when child throws', () => {
    // Suppress console.error for this test
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    render(
      <ErrorBoundary>
        <ThrowingComponent />
      </ErrorBoundary>
    );
    expect(screen.getByText('Something went wrong')).toBeDefined();
    expect(screen.getByText('Reload Page')).toBeDefined();

    consoleSpy.mockRestore();
  });
});
