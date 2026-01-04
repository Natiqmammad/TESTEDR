/**
 * AFNS React Renderer - Public API
 */

export { Text, Button, Row, Column, Container } from './components';
export {
    renderWidget,
    AfnsApp,
    type WidgetValue,
    type RenderContext,
    type AfnsAppProps,
} from './WidgetRenderer';

// Default styles
export const defaultStyles = `
.afns-text {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  font-size: 16px;
  color: #333;
}

.afns-button {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  font-size: 14px;
  padding: 8px 16px;
  border: none;
  border-radius: 4px;
  background: #007bff;
  color: white;
  cursor: pointer;
  transition: background-color 0.2s;
}

.afns-button:hover:not(:disabled) {
  background: #0056b3;
}

.afns-button:disabled {
  background: #ccc;
  cursor: not-allowed;
}

.afns-window {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
}

.afns-window-title {
  font-size: 24px;
  font-weight: 600;
  color: #333;
}

.afns-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 100vh;
  color: #666;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
}
`;
