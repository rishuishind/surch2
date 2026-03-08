import { createSignal, createEffect, onMount, For, Show } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./App.css";

interface SearchResultItem {
  id: string;
  title: string;
  subtitle: string | null;
  icon: string | null;
  icon_data: string | null;
  item_type: string;
  action_data: string;
}

function App() {
  const [viewMode, setViewMode] = createSignal<"main" | "clipboard">("main");
  const [query, setQuery] = createSignal("");
  const [results, setResults] = createSignal<SearchResultItem[]>([]);
  const [activeIndex, setActiveIndex] = createSignal(0);
  const [loading, setLoading] = createSignal(true);
  const [mathResult, setMathResult] = createSignal<string | null>(null);
  const [appCount, setAppCount] = createSignal(0);

  let inputRef!: HTMLInputElement;
  let resultsRef!: HTMLDivElement;

  // Search items
  async function searchItems(q: string) {
    try {
      const mode = viewMode();
      let items: SearchResultItem[] = [];

      if (mode === "main") {
        items = await invoke<SearchResultItem[]>("search_items", { query: q });
      } else if (mode === "clipboard") {
        items = await invoke<SearchResultItem[]>("search_clipboard_history", { query: q });
      }

      setResults(items);
      setActiveIndex(0);
    } catch (e) {
      console.error("Search failed:", e);
    }
  }

  // Try evaluating math expression
  async function tryMath(q: string) {
    if (viewMode() !== "main" || !q || q.length < 2) {
      setMathResult(null);
      return;
    }

    // Check if it looks like a math expression
    const mathPattern = /^[\d\s+\-*/().^%pisqrtcosntanlogbeab]+$/i;
    if (!mathPattern.test(q)) {
      setMathResult(null);
      return;
    }

    try {
      const result = await invoke<string>("evaluate_math", { expression: q });
      setMathResult(result);
    } catch {
      setMathResult(null);
    }
  }

  // Handle input changes
  createEffect(() => {
    const q = query();
    searchItems(q);
    tryMath(q);
  });

  // Re-run search when mode changes too
  createEffect(() => {
    searchItems(query());
  });

  // Execute an item
  async function executeItem(item: SearchResultItem) {
    if (item.action_data === "switch_view_clipboard") {
      setViewMode("clipboard");
      setQuery("");
      setMathResult(null);
      inputRef?.focus();
      return;
    }

    try {
      await invoke("execute_item", { item });
      // Hide window after execution
      const appWindow = getCurrentWindow();
      await appWindow.hide();
      setQuery("");
    } catch (e) {
      console.error("Launch failed:", e);
    }
  }

  // Keyboard navigation
  function handleKeyDown(e: KeyboardEvent) {
    const items = results();

    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        setActiveIndex((i) => Math.min(i + 1, items.length - 1));
        scrollToActive();
        break;
      case "ArrowUp":
        e.preventDefault();
        setActiveIndex((i) => Math.max(i - 1, 0));
        scrollToActive();
        break;
      case "Enter":
        e.preventDefault();
        if (items[activeIndex()]) {
          executeItem(items[activeIndex()]);
        }
        break;
      case "Escape":
        e.preventDefault();
        if (query()) {
          setQuery("");
          inputRef.value = "";
        } else if (viewMode() !== "main") {
          setViewMode("main");
        } else {
          getCurrentWindow().hide();
        }
        break;
      case "Tab":
        e.preventDefault();
        // Could be used for mode switching later
        break;
    }
  }

  function scrollToActive() {
    requestAnimationFrame(() => {
      const activeEl = resultsRef?.querySelector(".result-item.active");
      if (activeEl) {
        activeEl.scrollIntoView({ block: "nearest", behavior: "smooth" });
      }
    });
  }

  // Initialize
  onMount(async () => {
    setLoading(true);

    // Focus input
    inputRef?.focus();

    // Load initial items
    try {
      const count = await invoke<number>("refresh_items");
      setAppCount(count);
      await searchItems("");
    } catch (e) {
      console.error("Init failed:", e);
    }

    setLoading(false);

    // Listen for window focus to re-focus input
    const appWindow = getCurrentWindow();
    appWindow.onFocusChanged(({ payload: focused }) => {
      if (focused) {
        inputRef?.focus();
        // Select all text on refocus for quick replacement
        inputRef?.select();
      }
    });
  });

  return (
    <div class="launcher" onKeyDown={handleKeyDown}>
      {/* Search Bar */}
      <div class="search-container">
        <Show
          when={viewMode() === "clipboard"}
          fallback={
            <svg class="search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="11" cy="11" r="8" />
              <line x1="21" y1="21" x2="16.65" y2="16.65" />
            </svg>
          }
        >
          <div class="view-badge">Clipboard History</div>
        </Show>

        <input
          ref={inputRef!}
          class="search-input"
          type="text"
          placeholder={viewMode() === "main" ? "Search apps, commands, or type math..." : "Search clipboard..."}
          value={query()}
          onInput={(e) => setQuery(e.currentTarget.value)}
          autofocus
          spellcheck={false}
        />
        <div class="search-shortcut">
          <span class="kbd">⌘</span>
          <span class="kbd">Space</span>
        </div>
      </div>

      {/* Math Result */}
      <Show when={mathResult()}>
        <div class="math-result">
          <svg class="math-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <line x1="12" y1="2" x2="12" y2="22" />
            <line x1="2" y1="12" x2="22" y2="12" />
          </svg>
          <span class="math-expression">{query()}</span>
          <span class="math-equals">=</span>
          <span class="math-value">{mathResult()}</span>
        </div>
      </Show>

      {/* Results */}
      <div class="results-container" ref={resultsRef!}>
        <Show when={loading()}>
          <div class="loading-dots">
            <div class="loading-dot" />
            <div class="loading-dot" />
            <div class="loading-dot" />
          </div>
        </Show>

        <Show when={!loading() && results().length === 0 && query()}>
          <div class="empty-state">
            <svg class="empty-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <circle cx="12" cy="12" r="10" />
              <line x1="15" y1="9" x2="9" y2="15" />
              <line x1="9" y1="9" x2="15" y2="15" />
            </svg>
            <span class="empty-text">No results for "{query()}"</span>
            <span class="empty-hint">Try a different search term</span>
          </div>
        </Show>

        <Show when={!loading() && results().length > 0}>
          <div class="results-section-label">
            {query()
              ? "Search Results"
              : viewMode() === "clipboard"
                ? "Recent Clipboard Entries"
                : "Applications & Commands"}
          </div>
          <For each={results()}>
            {(item, index) => (
              <div
                class={`result-item ${index() === activeIndex() ? "active" : ""}`}
                onClick={() => executeItem(item)}
                onMouseEnter={() => setActiveIndex(index())}
              >
                <div class="result-icon-wrapper">
                  <Show
                    when={item.icon_data}
                    fallback={
                      <div class="result-icon-fallback">
                        {item.title.charAt(0).toUpperCase()}
                      </div>
                    }
                  >
                    <img
                      src={item.icon_data!}
                      alt={item.title}
                      loading="lazy"
                    />
                  </Show>
                </div>
                <div class="result-content">
                  <div class="result-name">{item.title}</div>
                  <Show when={item.subtitle}>
                    <div class="result-description">{item.subtitle}</div>
                  </Show>
                </div>
                <div class="result-action">
                  <span class="result-action-text">Select</span>
                  <span class="kbd">↵</span>
                </div>
              </div>
            )}
          </For>
        </Show>
      </div>

      {/* Status Bar */}
      <div class="status-bar">
        <div class="status-left">
          <span class="status-text">
            {appCount() > 0 ? `${appCount()} apps indexed` : "Loading..."}
          </span>
        </div>
        <div class="status-right">
          <div class="status-action">
            <span class="kbd">↑↓</span>
            <span class="status-text">Navigate</span>
          </div>
          <div class="status-action">
            <span class="kbd">↵</span>
            <span class="status-text">Open</span>
          </div>
          <div class="status-action">
            <span class="kbd">Esc</span>
            <span class="status-text">Close</span>
          </div>
        </div>
      </div>
    </div>
  );
}

export default App;
