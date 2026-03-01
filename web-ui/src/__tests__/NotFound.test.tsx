import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { MemoryRouter } from 'react-router-dom';
import NotFound from '../pages/NotFound';

const mockNavigate = vi.fn();
vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual('react-router-dom');
  return { ...actual, useNavigate: () => mockNavigate };
});

describe('NotFound', () => {
  it('renders 404 text', () => {
    render(
      <MemoryRouter>
        <NotFound />
      </MemoryRouter>
    );
    expect(screen.getByText('404')).toBeDefined();
    expect(screen.getByText(/page not found/i)).toBeDefined();
  });

  it('has a button to go to dashboard', () => {
    render(
      <MemoryRouter>
        <NotFound />
      </MemoryRouter>
    );
    const button = screen.getByText('Go to Dashboard');
    expect(button).toBeDefined();
    fireEvent.click(button);
    expect(mockNavigate).toHaveBeenCalledWith('/');
  });
});
