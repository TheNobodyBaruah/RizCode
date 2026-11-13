import "@testing-library/jest-dom/vitest";

// jsdom does not implement `matchMedia`, which some UI libraries expect.
if (typeof window !== "undefined" && !window.matchMedia) {
  Object.defineProperty(window, "matchMedia", {
    writable: true,
    value: (query: string) => ({
      matches: false,
      media: query,
      onchange: null,
      addListener: () => {},
      removeListener: () => {},
      addEventListener: () => {},
      removeEventListener: () => {},
      dispatchEvent: () => false,
    }),
  });
}

// jsdom does not implement the text measurement APIs that CodeMirror 6 uses to size its
// content. Without them the editor throws while it measures the document.
if (typeof Range !== "undefined") {
  const emptyRect: DOMRect = {
    top: 0,
    left: 0,
    right: 0,
    bottom: 0,
    width: 0,
    height: 0,
    x: 0,
    y: 0,
    toJSON: () => ({}),
  } as DOMRect;

  Range.prototype.getClientRects = function getClientRects() {
    const rects = [emptyRect] as unknown as DOMRectList;
    return rects;
  };

  Range.prototype.getBoundingClientRect = function getBoundingClientRect() {
    return emptyRect;
  };
}
