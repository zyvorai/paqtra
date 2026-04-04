import { render } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import App from '../App';

describe('App', () => {
  it('renders without crashing', () => {
    const { container } = render(<App />);
    expect(container).toBeDefined();
    expect(container.querySelector('#root') ?? container.firstChild).toBeDefined();
  });
});
