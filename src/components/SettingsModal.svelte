<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { X, FolderOpen, Save, HardDrive, Cpu, Languages, Settings as SettingsIcon } from 'lucide-svelte';
  import { portal } from '../utils/portal';
  import './SettingsModal.css';

  interface Settings {
    launcher_dir: string;
    default_memory: number;
    default_java: string | null;
    theme: string;
    language: string;
  }

  let { onClose } = $props<{
    onClose?: () => void;
  }>();

  let settings = $state<Settings | null>(null);
  let loading = $state(true);
  let isClosing = $state(false);
  let saving = $state(false);

  onMount(() => {
    loadSettings();
    document.body.classList.add('modal-open');
    return () => {
      document.body.classList.remove('modal-open');
    };
  });

  async function loadSettings() {
    try {
      const data: Settings = await invoke('get_settings');
      settings = data;
    } catch (err) {
      console.error('Failed to load settings:', err);
    } finally {
      loading = false;
    }
  }

  function handleClose() {
    isClosing = true;
    document.body.classList.remove('modal-open');
    setTimeout(() => {
      if (onClose) {
        onClose();
      } else {
        window.history.back();
      }
    }, 300);
  }

  async function handleSave() {
    if (!settings) return;
    try {
      saving = true;
      settings.default_memory = Number(settings.default_memory) || 4096;
      await invoke('save_settings', { newSettings: settings });
      handleClose();
    } catch (err) {
      alert('Hiba a mentéskor: ' + err);
      saving = false;
    }
  }

  async function handleSelectDir() {
    try {
      const newPath = await invoke<string | null>('select_folder');
      if (newPath && settings) {
        settings.launcher_dir = newPath;
      }
    } catch (err) {
      console.error(err);
    }
  }
</script>

{#if !loading && settings}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class={`settings-overlay-fixed ${isClosing ? 'closing' : ''}`} use:portal onclick={handleClose}>
    <div class="settings-modal-card" onclick={(e) => e.stopPropagation()}>
      <div class="settings-header">
        <div class="header-title">
          <SettingsIcon size={24} class="icon-green" />
          <h2>Beállítások</h2>
        </div>
        <button class="close-btn" onclick={handleClose} aria-label="Close"><X size={20} /></button>
      </div>

      <div class="settings-body">
        <section class="settings-section">
          <div class="section-label">
            <HardDrive size={18} />
            <span>Tárhely</span>
          </div>
          <div class="setting-control">
            <label for="launcher-dir">Launcher könyvtár</label>
            <div class="input-group">
              <input id="launcher-dir" type="text" value={settings.launcher_dir} readonly />
              <button class="icon-btn" onclick={handleSelectDir} type="button"><FolderOpen size={18} /></button>
            </div>
            <p class="helper-text">Itt lesznek tárolva az instance-ek és a játék adatok.</p>
          </div>
        </section>

        <section class="settings-section">
          <div class="section-label">
            <Cpu size={18} />
            <span>Teljesítmény</span>
          </div>
          <div class="memory-control-wrapper">
            <div class="memory-header">
              <label for="default-memory">Alapértelmezett Memória</label>
              <div class="memory-display-box">
                <input
                  id="default-memory-input"
                  type="number"
                  class="memory-number-input"
                  bind:value={settings.default_memory}
                  min={1024}
                  max={65536}
                  step={512}
                />
                <span class="memory-unit">MB</span>
                <span class="memory-gb-pill">{(settings.default_memory / 1024).toFixed(1)} GB</span>
              </div>
            </div>

            <div class="range-slider-wrapper">
              <div class="slider-track-container">
                <div
                  class="slider-thumb-tooltip"
                  style="left: calc({Math.min(100, Math.max(0, ((settings.default_memory - 1024) / (32768 - 1024)) * 100))}% + {(0.5 - Math.min(1, Math.max(0, (settings.default_memory - 1024) / (32768 - 1024)))) * 22}px);"
                >
                  <span class="tooltip-gb">{(settings.default_memory / 1024).toFixed(1)} GB</span>
                  <span class="tooltip-mb">({settings.default_memory} MB)</span>
                </div>
                <input
                  id="default-memory"
                  type="range"
                  class="ram-slider"
                  bind:value={settings.default_memory}
                  min={1024}
                  max={32768}
                  step={512}
                  style="background: linear-gradient(to right, var(--accent-green) 0%, var(--accent-green) {Math.min(100, Math.max(0, ((settings.default_memory - 1024) / (32768 - 1024)) * 100))}%, rgba(255, 255, 255, 0.1) {Math.min(100, Math.max(0, ((settings.default_memory - 1024) / (32768 - 1024)) * 100))}%, rgba(255, 255, 255, 0.1) 100%)"
                />
              </div>

              <div class="slider-marks-container">
                {#each [
                  { mb: 1024, label: '1 GB' },
                  { mb: 4096, label: '4 GB' },
                  { mb: 8192, label: '8 GB' },
                  { mb: 16384, label: '16 GB' },
                  { mb: 24576, label: '24 GB' },
                  { mb: 32768, label: '32 GB' }
                ] as mark}
                  {@const markPct = ((mark.mb - 1024) / (32768 - 1024)) * 100}
                  <button
                    type="button"
                    class="slider-mark-btn"
                    style="left: calc({markPct}% + {(0.5 - markPct / 100) * 22}px);"
                    class:active={settings ? Math.abs(settings.default_memory - mark.mb) < 256 : false}
                    onclick={() => { if (settings) settings.default_memory = mark.mb; }}
                    title="{mark.label} ({mark.mb} MB)"
                  >
                    <span class="mark-pip"></span>
                    <span class="mark-text">{mark.label}</span>
                  </button>
                {/each}
              </div>
            </div>

            <div class="memory-presets">
              {#each [2048, 4096, 6144, 8192, 12288, 16384] as preset}
                <button
                  type="button"
                  class="memory-preset-btn"
                  class:active={settings?.default_memory === preset}
                  onclick={() => { if (settings) settings.default_memory = preset; }}
                >
                  {preset / 1024} GB
                </button>
              {/each}
            </div>
          </div>
        </section>

        <section class="settings-section">
          <div class="section-label">
            <Languages size={18} />
            <span>Nyelv és Megjelenés</span>
          </div>
          <div class="setting-row">
            <div class="setting-control half">
              <label for="language-select">Nyelv</label>
              <select id="language-select" bind:value={settings.language}>
                <option value="hu">Magyar</option>
                <option value="en">English</option>
              </select>
            </div>
            <div class="setting-control half">
              <label for="theme-select">Téma</label>
              <select id="theme-select" bind:value={settings.theme}>
                <option value="dark">Sötét</option>
                <option value="light">Világos (Hamarosan)</option>
              </select>
            </div>
          </div>
        </section>
      </div>

      <div class="settings-footer">
        <button class="btn btn-secondary" onclick={handleClose}>Mégse</button>
        <button class="btn btn-primary" onclick={handleSave} disabled={saving}>
          <Save size={18} />
          <span>Mentés</span>
        </button>
      </div>
    </div>
  </div>
{/if}
