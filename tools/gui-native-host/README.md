# GUI Native Host

React-based renderer for ApexForge NightScript `forge.gui.native` module.

## Overview

This is the host application that renders widget trees from the AFNS runtime. It communicates via JSON over stdio:

- **Receives**: `{"kind":"render","tree":{...}}` render messages from runtime stdout
- **Sends**: `{"kind":"event","event":"click","target":"3","handler":"App.on_click"}` event messages to runtime stdin

## Development

```bash
# Install dependencies
npm install

# Start development server (demo mode)
npm run dev
```

## Widget Types

| Widget | Props | Description |
|--------|-------|-------------|
| `Text` | `text` | Display text content |
| `Button` | `label`, `handler` | Clickable button |
| `Row` | - | Horizontal flex container |
| `Column` | - | Vertical flex container |
| `Container` | `padding`, `background` | Styled wrapper |

## JSON Schema

### Render Message
```json
{
  "kind": "render",
  "tree": {
    "id": 1,
    "type": "Column",
    "props": {},
    "children": [
      {"id": 2, "type": "Text", "props": {"text": "Hello"}, "children": []}
    ]
  }
}
```

### Event Message
```json
{
  "kind": "event",
  "event": "click",
  "target": "3",
  "handler": "App.on_click"
}
```

## Building for Production

```bash
npm run build
```

The built files will be in the `dist/` directory.
