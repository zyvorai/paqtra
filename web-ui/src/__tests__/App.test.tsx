import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import App from '../App';

describe('App', () => {
  it('renders without crashing', () => {
    const { container } = render(<App />);
    expect(container.firstChild).toBeTruthy();
  });

  it('renders the main layout', () => {
    render(<App />);
    // Check that something meaningful from the app is rendered
    // e.g., the app title, navigation, or a loading state
    expect(document.body.querySelector('[class*="min-h-screen"], [class*="app"], main, nav')).toBeTruthy();
  });
});
