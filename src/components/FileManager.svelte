<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import {
    Folder, FileText, ChevronRight, MoreVertical,
    Trash2, Plus, ArrowLeft, X, Save, FileCode, FileImage, AlertTriangle
  } from 'lucide-svelte';
  import { showNotification } from '../stores/notification';
  import { portal } from '../utils/portal';
  import './FileManager.css';

  interface FileInfo {
    name: string;
    path: string;
    is_dir: boolean;
    size: number;
    created: number;
    modified: number;
    item_count?: number;
  }

  let { basePath } = $props<{ basePath: string }>();

  let currentPath = $state(basePath);
  let files = $state<FileInfo[]>([]);
  let loading = $state(true);
  let selectedPaths = $state<string[]>([]);
  let editingFile = $state<{ path: string; name: string; content: string } | null>(null);
  let showNewMenu = $state(false);
  let showDeleteModal = $state(false);
  let isClosingModal = $state(false);

  async function loadFiles(path: string) {
    try {
      loading = true;
      const result: FileInfo[] = await invoke('list_directory', { path });
      files = result;
      selectedPaths = [];
    } catch (err) {
      showNotification(`Hiba: ${err}`, 'error');
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    currentPath = basePath;
    loadFiles(basePath);
  });

  $effect(() => {
    if (currentPath) {
      loadFiles(currentPath);
    }
  });

  function navigateTo(path: string) {
    currentPath = path;
  }

  function navigateBack() {
    const parts = currentPath.split(/[\\/]/);
    if (parts.length > 1) {
      parts.pop();
      const parent = parts.join('/');
      if (parent.length >= basePath.length) navigateTo(parent);
    }
  }

  function getBreadcrumbs() {
    const relative = currentPath.substring(basePath.length);
    const parts = relative.split(/[\\/]/).filter((p: string) => p);
    return [
      { name: 'GameDir', path: basePath },
      ...parts.map((p: string, i: number) => ({
        name: p,
        path: basePath + '/' + parts.slice(0, i + 1).join('/')
      }))
    ];
  }

  async function handleFileClick(file: FileInfo) {
    if (file.is_dir) {
      navigateTo(file.path);
    } else {
      const ext = file.name.split('.').pop()?.toLowerCase();
      const textExtensions = ['txt', 'json', 'log', 'properties', 'toml', 'yaml', 'yml', 'cfg', 'conf', 'js', 'py'];
      if (textExtensions.includes(ext || '')) {
        try {
          const content: string = await invoke('read_text_file', { path: file.path });
          editingFile = { path: file.path, name: file.name, content };
        } catch (err) {
          showNotification('Nem sikerült megnyitni a fájlt.', 'error');
        }
      }
    }
  }

  function handleToggleSelect(path: string, e: MouseEvent) {
    e.stopPropagation();
    if (selectedPaths.includes(path)) {
      selectedPaths = selectedPaths.filter((p) => p !== path);
    } else {
      selectedPaths = [...selectedPaths, path];
    }
  }

  function handleSelectAll() {
    if (selectedPaths.length === files.length) {
      selectedPaths = [];
    } else {
      selectedPaths = files.map((f) => f.path);
    }
  }

  function closeDeleteModal() {
    isClosingModal = true;
    setTimeout(() => {
      showDeleteModal = false;
      isClosingModal = false;
    }, 300);
  }

  async function confirmDelete() {
    try {
      for (const t of selectedPaths) {
        await invoke('delete_file_item', { path: t });
      }
      showNotification('Sikeres törlés', 'success');
      closeDeleteModal();
      loadFiles(currentPath);
    } catch (err) {
      showNotification(`Hiba: ${err}`, 'error');
    }
  }

  async function handleCreate(isDir: boolean) {
    const name = prompt(isDir ? 'Mappa neve:' : 'Fájl neve:');
    if (!name) return;
    try {
      const fullPath = `${currentPath}/${name}`;
      await invoke('create_file_item', { path: fullPath, isDir });
      loadFiles(currentPath);
      showNewMenu = false;
    } catch (err) {
      showNotification(`Hiba: ${err}`, 'error');
    }
  }

  async function handleSaveFile() {
    if (!editingFile) return;
    try {
      await invoke('write_text_file', { path: editingFile.path, content: editingFile.content });
      showNotification('Fájl mentve', 'success');
      editingFile = null;
      loadFiles(currentPath);
    } catch (err) {
      showNotification('Hiba a mentéskor', 'error');
    }
  }

  function formatSize(bytes: number) {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  }

  function getFileExtension(filename: string) {
    return filename.split('.').pop()?.toLowerCase() || '';
  }
</script>

<div class="file-manager">
  <div class="fm-toolbar">
    <div class="fm-breadcrumbs">
      <button class="back-nav" onclick={navigateBack} disabled={currentPath === basePath} aria-label="Back">
        <ArrowLeft size={18} />
      </button>
      {#each getBreadcrumbs() as bc, i}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <span class="bc-item" onclick={() => navigateTo(bc.path)}>{bc.name}</span>
        {#if i < getBreadcrumbs().length - 1}
          <ChevronRight size={14} class="bc-sep" />
        {/if}
      {/each}
    </div>

    <div class="fm-actions">
      <div class="new-menu-container">
        <button class="btn-action" onclick={() => (showNewMenu = !showNewMenu)}>
          <Plus size={18} /><span>Létrehozás</span>
        </button>
        {#if showNewMenu}
          <div class="fm-dropdown fade-in">
            <button onclick={() => handleCreate(true)}><Folder size={16} /> Új mappa</button>
            <button onclick={() => handleCreate(false)}><FileText size={16} /> Új fájl</button>
          </div>
        {/if}
      </div>
      {#if selectedPaths.length > 0}
        <button class="btn-action danger" onclick={() => (showDeleteModal = true)}>
          <Trash2 size={18} /><span>Törlés ({selectedPaths.length})</span>
        </button>
      {/if}
    </div>
  </div>

  <div class="fm-list-container">
    <table class="fm-table">
      <thead>
        <tr>
          <th class="col-check">
            <div class="custom-checkbox">
              <input
                type="checkbox"
                onchange={handleSelectAll}
                checked={files.length > 0 && selectedPaths.length === files.length}
              />
              <span></span>
            </div>
          </th>
          <th class="col-name">Név</th>
          <th class="col-size">Méret / Elem</th>
          <th class="col-modified">Módosítva</th>
          <th class="col-more"></th>
        </tr>
      </thead>
      <tbody>
        {#if loading}
          <tr><td colspan="5" class="fm-loading">Betöltés...</td></tr>
        {:else if files.length === 0}
          <tr><td colspan="5" class="fm-empty">A mappa üres</td></tr>
        {:else}
          {#each files as file (file.path)}
            <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
            <tr
              class={selectedPaths.includes(file.path) ? 'selected' : ''}
              ondblclick={() => handleFileClick(file)}
            >
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <td class="col-check" onclick={(e) => handleToggleSelect(file.path, e)}>
                <div class="custom-checkbox">
                  <input type="checkbox" checked={selectedPaths.includes(file.path)} readonly />
                  <span></span>
                </div>
              </td>
              <td class="col-name">
                <div class="name-wrapper">
                  {#if file.is_dir}
                    <Folder class="icon-folder" size={18} />
                  {:else if ['json', 'toml', 'cfg', 'properties'].includes(getFileExtension(file.name))}
                    <FileCode class="icon-code" size={18} />
                  {:else if ['png', 'jpg', 'jpeg'].includes(getFileExtension(file.name))}
                    <FileImage class="icon-image" size={18} />
                  {:else}
                    <FileText class="icon-file" size={18} />
                  {/if}
                  <span>{file.name}</span>
                </div>
              </td>
              <td class="col-size">
                {file.is_dir ? `${file.item_count ?? 0} elem` : formatSize(file.size)}
              </td>
              <td class="col-modified">
                {new Date(file.modified * 1000).toLocaleString()}
              </td>
              <td class="col-more">
                <button class="more-btn" aria-label="More"><MoreVertical size={16} /></button>
              </td>
            </tr>
          {/each}
        {/if}
      </tbody>
    </table>
  </div>

  {#if editingFile}
    <div class="modal-overlay editor-overlay" use:portal>
      <div class="modal-content editor-modal">
        <div class="editor-header">
          <div class="editor-title">
            <FileText size={18} class="icon-green" />
            <h3>{editingFile.name}</h3>
          </div>
          <div class="editor-actions">
            <button class="btn-save" onclick={handleSaveFile}>
              <Save size={18} /> Mentés
            </button>
            <button class="close-btn" onclick={() => (editingFile = null)} aria-label="Close">
              <X size={20} />
            </button>
          </div>
        </div>
        <textarea
          class="editor-textarea"
          bind:value={editingFile.content}
          spellcheck="false"
        ></textarea>
      </div>
    </div>
  {/if}

  {#if showDeleteModal}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class={`modal-overlay ${isClosingModal ? 'closing' : ''}`} use:portal onclick={closeDeleteModal}>
      <div class="modal-content delete-modal" onclick={(e) => e.stopPropagation()}>
        <div class="modal-header centered">
          <div class="alert-icon-wrapper"><AlertTriangle size={32} /></div>
          <h2>Fájlok törlése</h2>
        </div>
        <p class="modal-desc centered">
          Biztosan törölni szeretnél <strong>{selectedPaths.length}</strong> elemet?
        </p>
        <div class="modal-actions spaced">
          <button class="btn btn-secondary flex-1" onclick={closeDeleteModal}>Mégse</button>
          <button class="btn btn-danger flex-1" onclick={confirmDelete}>Törlés</button>
        </div>
      </div>
    </div>
  {/if}
</div>
