<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { Copy, Trash2, ChevronDown, Terminal } from 'lucide-svelte';
  import './Console.css';

  let logs = $state<string[]>([]);
  let scrollContainer: HTMLDivElement | null = null;
  let autoScroll = $state(true);
  let unlistenLog: (() => void) | undefined;

  onMount(async () => {
    unlistenLog = await listen<string>('minecraft-log', (event) => {
      logs = [...logs, event.payload];
    });
  });

  onDestroy(() => {
    if (unlistenLog) unlistenLog();
  });

  $effect(() => {
    if (autoScroll && scrollContainer && logs.length) {
      scrollContainer.scrollTop = scrollContainer.scrollHeight;
    }
  });

  function handleCopy() {
    navigator.clipboard.writeText(logs.join('\n'));
  }

  function handleClear() {
    logs = [];
  }
</script>

<div class="console-container">
  <div class="console-header">
    <div class="header-left">
      <Terminal size={18} class="icon-green" />
      <span>Spring Launcher - Játék Konzol</span>
    </div>
    <div class="header-actions">
      <button class="icon-btn" onclick={handleCopy} title="Összes másolása" aria-label="Copy logs">
        <Copy size={16} />
      </button>
      <button class="icon-btn" onclick={handleClear} title="Log törlése" aria-label="Clear logs">
        <Trash2 size={16} />
      </button>
      <button
        class={`icon-btn ${autoScroll ? 'active' : ''}`}
        onclick={() => (autoScroll = !autoScroll)}
        title="Automatikus görgetés"
        aria-label="Toggle auto scroll"
      >
        <ChevronDown size={16} />
      </button>
    </div>
  </div>

  <div class="console-logs" bind:this={scrollContainer}>
    {#if logs.length === 0}
      <div class="console-empty">Várakozás a logokra...</div>
    {:else}
      {#each logs as log, i}
        <div class={`log-line ${log.includes('[ERROR]') ? 'error' : ''}`}>
          <span class="log-time">[{new Date().toLocaleTimeString()}]</span>
          <span class="log-text">{log}</span>
        </div>
      {/each}
    {/if}
  </div>
</div>
