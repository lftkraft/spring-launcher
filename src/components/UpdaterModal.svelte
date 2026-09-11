<script lang="ts">
  import { onMount } from 'svelte';
  import { Download, RefreshCw, Sparkles, X, CheckCircle2, AlertTriangle } from 'lucide-svelte';
  import { check } from '@tauri-apps/plugin-updater';
  import { relaunch } from '@tauri-apps/plugin-process';
  import { availableUpdate, showUpdaterModal, isCheckingUpdate } from '../stores/updater';

  let isDownloading = $state(false);
  let isInstalled = $state(false);
  let errorMessage = $state<string | null>(null);
  let downloadedBytes = $state(0);
  let totalBytes = $state(0);

  let progressPercentage = $derived(
    totalBytes > 0 ? Math.min(100, Math.round((downloadedBytes / totalBytes) * 100)) : 0
  );

  async function checkUpdate() {
    try {
      isCheckingUpdate.set(true);
      errorMessage = null;
      const update = await check();
      if (update && update.available) {
        availableUpdate.set(update);
      } else {
        availableUpdate.set(null);
      }
    } catch (err: any) {
      console.warn('[Updater] Could not check for updates:', err);
    } finally {
      isCheckingUpdate.set(false);
    }
  }

  async function startDownloadAndInstall() {
    const update = $availableUpdate;
    if (!update) return;

    try {
      isDownloading = true;
      errorMessage = null;
      downloadedBytes = 0;
      totalBytes = 0;

      await update.downloadAndInstall((event) => {
        switch (event.event) {
          case 'Started':
            totalBytes = event.data.contentLength || 0;
            downloadedBytes = 0;
            break;
          case 'Progress':
            downloadedBytes += event.data.chunkLength;
            break;
          case 'Finished':
            downloadedBytes = totalBytes;
            break;
        }
      });

      isDownloading = false;
      isInstalled = true;
    } catch (err: any) {
      console.error('[Updater] Download/install error:', err);
      isDownloading = false;
      errorMessage = err?.message || 'Nem sikerült letölteni a frissítést.';
    }
  }

  async function handleRelaunch() {
    try {
      await relaunch();
    } catch (err: any) {
      console.error('[Updater] Relaunch failed:', err);
    }
  }

  function dismiss() {
    showUpdaterModal.set(false);
  }

  onMount(() => {
    const timer = setTimeout(() => {
      checkUpdate();
    }, 2000);

    return () => clearTimeout(timer);
  });
</script>

{#if $showUpdaterModal && $availableUpdate}
  <div class="updater-overlay">
    <div class="updater-modal">
      <div class="updater-header">
        <div class="updater-title">
          <Sparkles class="icon-sparkle" size={24} />
          <div>
            <h3>Új frissítés érhető el!</h3>
            <span class="version-tag">v{$availableUpdate.version}</span>
          </div>
        </div>
        {#if !isDownloading}
          <button class="close-btn" onclick={dismiss} title="Bezárás">
            <X size={18} />
          </button>
        {/if}
      </div>

      <div class="updater-body">
        {#if $availableUpdate.body}
          <div class="release-notes">
            <h4>Kiadási megjegyzések:</h4>
            <div class="notes-content">
              {$availableUpdate.body}
            </div>
          </div>
        {:else}
          <p class="desc">Új verzió tölthető le a Spring Launcher-höz.</p>
        {/if}

        {#if errorMessage}
          <div class="error-box">
            <AlertTriangle size={18} />
            <span>{errorMessage}</span>
          </div>
        {/if}

        {#if isDownloading}
          <div class="progress-section">
            <div class="progress-info">
              <span>Telepítő letöltése...</span>
              <span>{progressPercentage}%</span>
            </div>
            <div class="progress-bar-bg">
              <div class="progress-bar-fill" style="width: {progressPercentage}%"></div>
            </div>
          </div>
        {/if}

        {#if isInstalled}
          <div class="success-box">
            <CheckCircle2 size={20} />
            <span>A frissítés sikeresen telepítve! Indítsd újra az alkalmazást.</span>
          </div>
        {/if}
      </div>

      <div class="updater-footer">
        {#if isInstalled}
          <button class="btn btn-primary" onclick={handleRelaunch}>
            <RefreshCw size={16} />
            Újraindítás most
          </button>
        {:else if isDownloading}
          <button class="btn btn-disabled" disabled>
            <RefreshCw size={16} class="spinning" />
            Letöltés folyamatban...
          </button>
        {:else}
          <button class="btn btn-secondary" onclick={dismiss}>Mégse</button>
          <button class="btn btn-primary" onclick={startDownloadAndInstall}>
            <Download size={16} />
            Frissítés most
          </button>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .updater-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.7);
    backdrop-filter: blur(8px);
    z-index: 9999;
    display: flex;
    align-items: center;
    justify-content: center;
    animation: fadeIn 0.25s ease-out;
  }

  .updater-modal {
    background: #18181b;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 16px;
    width: 460px;
    max-width: 90vw;
    box-shadow: 0 20px 40px rgba(0, 0, 0, 0.6);
    overflow: hidden;
    color: #f4f4f5;
    font-family: inherit;
  }

  .updater-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 20px 24px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  }

  .updater-title {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  :global(.icon-sparkle) {
    color: #a855f7;
  }

  .updater-title h3 {
    margin: 0;
    font-size: 1.1rem;
    font-weight: 600;
  }

  .version-tag {
    font-size: 0.8rem;
    background: rgba(168, 85, 247, 0.2);
    color: #c084fc;
    padding: 2px 8px;
    border-radius: 12px;
    font-weight: 500;
  }

  .close-btn {
    background: transparent;
    border: none;
    color: #a1a1aa;
    cursor: pointer;
    padding: 4px;
    border-radius: 6px;
    transition: all 0.2s;
  }

  .close-btn:hover {
    color: #fff;
    background: rgba(255, 255, 255, 0.1);
  }

  .updater-body {
    padding: 20px 24px;
  }

  .desc {
    margin: 0;
    color: #a1a1aa;
    font-size: 0.95rem;
  }

  .release-notes {
    background: rgba(0, 0, 0, 0.3);
    border-radius: 8px;
    padding: 12px 16px;
    border: 1px solid rgba(255, 255, 255, 0.05);
  }

  .release-notes h4 {
    margin: 0 0 6px 0;
    font-size: 0.85rem;
    color: #a1a1aa;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .notes-content {
    font-size: 0.9rem;
    color: #e4e4e7;
    white-space: pre-wrap;
    max-height: 140px;
    overflow-y: auto;
  }

  .error-box {
    margin-top: 12px;
    display: flex;
    align-items: center;
    gap: 8px;
    background: rgba(239, 68, 68, 0.15);
    border: 1px solid rgba(239, 68, 68, 0.3);
    color: #fca5a5;
    padding: 10px 14px;
    border-radius: 8px;
    font-size: 0.85rem;
  }

  .success-box {
    margin-top: 12px;
    display: flex;
    align-items: center;
    gap: 10px;
    background: rgba(34, 197, 94, 0.15);
    border: 1px solid rgba(34, 197, 94, 0.3);
    color: #86efac;
    padding: 12px 14px;
    border-radius: 8px;
    font-size: 0.9rem;
  }

  .progress-section {
    margin-top: 16px;
  }

  .progress-info {
    display: flex;
    justify-content: space-between;
    font-size: 0.85rem;
    color: #a1a1aa;
    margin-bottom: 6px;
  }

  .progress-bar-bg {
    height: 8px;
    background: rgba(255, 255, 255, 0.1);
    border-radius: 4px;
    overflow: hidden;
  }

  .progress-bar-fill {
    height: 100%;
    background: linear-gradient(90deg, #9333ea, #c084fc);
    border-radius: 4px;
    transition: width 0.2s ease;
  }

  .updater-footer {
    padding: 16px 24px;
    border-top: 1px solid rgba(255, 255, 255, 0.08);
    display: flex;
    justify-content: flex-end;
    gap: 12px;
    background: rgba(0, 0, 0, 0.2);
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 10px 18px;
    border-radius: 8px;
    font-size: 0.9rem;
    font-weight: 500;
    cursor: pointer;
    border: none;
    transition: all 0.2s;
  }

  .btn-secondary {
    background: rgba(255, 255, 255, 0.08);
    color: #e4e4e7;
  }

  .btn-secondary:hover {
    background: rgba(255, 255, 255, 0.15);
  }

  .btn-primary {
    background: #9333ea;
    color: #ffffff;
  }

  .btn-primary:hover {
    background: #a855f7;
  }

  .btn-disabled {
    background: rgba(255, 255, 255, 0.1);
    color: #71717a;
    cursor: not-allowed;
  }

  :global(.spinning) {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  @keyframes fadeIn {
    from { opacity: 0; transform: scale(0.96); }
    to { opacity: 1; transform: scale(1); }
  }
</style>
