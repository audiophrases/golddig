// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

// The right-click menu over dictionary text: "Look up" first, then the editing items the
// WebView2 default menu would have offered. It is a native menu (Tauri's menu API, already
// covered by the `core:default` capability), not an HTML imitation, because right-click in a
// desktop app should look and behave like right-click everywhere else on the desktop.
//
// The default WebView2 menu is suppressed only over non-editable content. The search box
// keeps its native Cut / Copy / Paste / Undo, which this module would otherwise have to
// re-implement badly. In the browser preview there is no native menu API, so the browser's
// own menu appears and there is no Look up.

import { Menu, MenuItem, PredefinedMenuItem } from '@tauri-apps/api/menu';
import { escapeMenuLabel, lookupLabel, normalizeLookupText, wordAt } from './lookup';

/** Inputs, text areas and editable regions keep the platform's own context menu. */
export function isEditable(target: EventTarget | null): boolean {
  if (!(target instanceof Element)) return false;
  if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement) return true;
  return target.closest('[contenteditable]:not([contenteditable="false"])') !== null;
}

/** Whether the point (client coordinates) lies inside the current selection's boxes. */
function pointInSelection(selection: Selection, x: number, y: number): boolean {
  for (let i = 0; i < selection.rangeCount; i++) {
    for (const rect of selection.getRangeAt(i).getClientRects()) {
      if (x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom) return true;
    }
  }
  return false;
}

/** The text node and offset under a point, via whichever API this engine exposes. */
function caretAt(x: number, y: number): { node: Node; offset: number } | null {
  const doc = document as Document & {
    caretPositionFromPoint?: (x: number, y: number) => { offsetNode: Node; offset: number } | null;
  };
  if (typeof doc.caretPositionFromPoint === 'function') {
    const pos = doc.caretPositionFromPoint(x, y);
    return pos ? { node: pos.offsetNode, offset: pos.offset } : null;
  }
  if (typeof document.caretRangeFromPoint === 'function') {
    const range = document.caretRangeFromPoint(x, y);
    return range ? { node: range.startContainer, offset: range.startOffset } : null;
  }
  return null;
}

/**
 * What a right-click at this point means to look up: the selected text when the click is
 * on the selection, otherwise the word under the pointer — which is then selected, so the
 * reader sees exactly what the menu is offering. Null when there is no word there.
 */
export function lookupTargetAt(event: MouseEvent): string | null {
  const selection = window.getSelection();
  if (selection && !selection.isCollapsed && pointInSelection(selection, event.clientX, event.clientY)) {
    return normalizeLookupText(selection.toString()) || null;
  }

  const caret = caretAt(event.clientX, event.clientY);
  if (!caret || caret.node.nodeType !== Node.TEXT_NODE) return null;
  const textNode = caret.node as Text;
  const hit = wordAt(textNode.data, caret.offset);
  if (!hit) return null;

  const range = document.createRange();
  range.setStart(textNode, hit.start);
  range.setEnd(textNode, hit.end);
  selection?.removeAllRanges();
  selection?.addRange(range);
  return normalizeLookupText(hit.word) || null;
}

async function copyText(text: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    // The async clipboard needs a secure context and a user gesture; the menu click is a
    // gesture but not one the page saw, so fall back to the classic command.
    document.execCommand('copy');
  }
}

function selectAll(): void {
  const selection = window.getSelection();
  if (!selection) return;
  selection.removeAllRanges();
  selection.selectAllChildren(document.body);
}

export interface ContextMenuRequest {
  /** Text to offer under "Look up"; null disables the item rather than hiding it. */
  lookupText: string | null;
  onLookup: (text: string) => void;
}

// Menus are native resources. One is kept alive at a time: closing it while its click is
// still being delivered would lose the click, so the previous menu is released when the
// next one is built — long after any click on it could arrive.
let liveMenu: Menu | null = null;

/** Builds and pops up the menu at the cursor. Resolves once the menu is showing. */
export async function showContextMenu(request: ContextMenuRequest): Promise<void> {
  const selectedText = window.getSelection()?.toString() ?? '';
  const { lookupText, onLookup } = request;

  const items = [
    await MenuItem.new({
      id: 'golddig-lookup',
      text: lookupText ? escapeMenuLabel(lookupLabel(lookupText)) : 'Look up',
      enabled: lookupText !== null,
      action: () => {
        if (lookupText) onLookup(lookupText);
      },
    }),
    await PredefinedMenuItem.new({ item: 'Separator' }),
    await MenuItem.new({
      id: 'golddig-copy',
      text: 'Copy',
      enabled: selectedText.length > 0,
      action: () => void copyText(selectedText),
    }),
    await MenuItem.new({
      id: 'golddig-select-all',
      text: 'Select All',
      action: selectAll,
    }),
  ];

  const menu = await Menu.new({ items });
  const previous = liveMenu;
  liveMenu = menu;
  if (previous) await previous.close().catch(() => {});
  await menu.popup();
}
