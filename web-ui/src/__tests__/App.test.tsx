import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import App from '../App';

describe('App', () => {
  it('renders the layout with navigation', () => {
    render(<App />);
    expect(screen.getAllByText('Cilium Vision').length).toBeGreaterThan(0);
  });

  it('renders the dashboard page by default', () => {
    render(<App />);
    // "Dashboard" appears in both the nav sidebar and page heading
    expect(screen.getAllByText('Dashboard').length).toBeGreaterThanOrEqual(1);
  });
});
