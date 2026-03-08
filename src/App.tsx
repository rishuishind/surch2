import { createSignal, createEffect, onMount, For, Show } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./App.css";

interface AppEntry {
  name: string;
  exec: string;
  icon: string | null;
  icon_data: string | null;
  description: string | null;
  categories: string | null;
  desktop_file: string;
}

function App() {
  const [query, setQuery] = createSignal("");
  const [results, setResults] = createSignal<AppEntry[]>([]);
  const [activeIndex, setActiveIndex] = createSignal(0);
  const [loading, setLoading] = createSignal(true);
  const [mathResult, setMathResult] = createSignal<string | null>(null);
  const [appCount, setAppCount] = createSignal(0);

  let inputRef!: HTMLInputElement;
  let resultsRef!: HTMLDivElement;

  // Search applications
  async function searchApps(q: string) {
    try {
      const apps = await invoke<AppEntry[]>("search_apps", { query: q });
      setResults(apps);
      setActiveIndex(0);
    } catch (e) {
      console.error("Search failed:", e);
    }
  }

  // Try evaluating math expression
  async function tryMath(q: string) {
    if (!q || q.length < 2) {
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
    searchApps(q);
    tryMath(q);
  });

  // Launch an app
  async function launchApp(app: AppEntry) {
    try {
      await invoke("launch_app", { exec: app.exec });
      // Hide window after launch
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
          launchApp(items[activeIndex()]);
        }
        break;
      case "Escape":
        e.preventDefault();
        if (query()) {
          setQuery("");
          inputRef.value = "";
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

    // Load initial apps
    try {
      const count = await invoke<number>("refresh_apps");
      setAppCount(count);
      await searchApps("");
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
        <svg class="search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="11" cy="11" r="8" />
          <line x1="21" y1="21" x2="16.65" y2="16.65" />
        </svg>
        <input
          ref={inputRef!}
          class="search-input"
          type="text"
          placeholder="Search apps, commands, or type math..."
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
            {query() ? "Search Results" : "Applications"}
          </div>
          <For each={results()}>
            {(app, index) => (
              <div
                class={`result-item ${index() === activeIndex() ? "active" : ""}`}
                onClick={() => launchApp(app)}
                onMouseEnter={() => setActiveIndex(index())}
              >
                <div class="result-icon-wrapper">
                  <Show
                    when={app.icon_data}
                    fallback={
                      <div class="result-icon-fallback">
                        {app.name.charAt(0).toUpperCase()}
                      </div>
                    }
                  >
                    <img
                      src={app.icon_data!}
                      alt={app.name}
                      loading="lazy"
                    />
                  </Show>
                </div>
                <div class="result-content">
                  <div class="result-name">{app.name}</div>
                  <Show when={app.description}>
                    <div class="result-description">{app.description}</div>
                  </Show>
                </div>
                <div class="result-action">
                  <span class="result-action-text">Open</span>
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
