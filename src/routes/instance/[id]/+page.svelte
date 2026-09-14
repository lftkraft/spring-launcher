<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import {
    Play, FolderOpen, Trash2, Clock, Box, ShieldCheck, ChevronLeft, ChevronDown, ChevronRight, Loader2,
    Image as ImageIcon, Coffee, MoreVertical, RotateCcw, Cpu, Camera, Square, X, Copy, ZoomIn, ZoomOut,
    Activity, FileText, Download, Trash, Power, RefreshCcw, Maximize2, AlertTriangle,
    Search, ArrowDown, ArrowUp, Check, AlignLeft, Terminal, Zap, HardDrive
  } from 'lucide-svelte';
  import type { Instance } from '../../../types/instance';
  import type { Profile } from '../../../types/profile';
  import type { ModrinthVersion } from '../../../types/mod';
  import FileManager from '../../../components/FileManager.svelte';
  import ScreenshotGallery from '../../../components/ScreenshotGallery.svelte';
  import NoMods from '../../../components/NoMods.svelte';
  import CreateInstanceModal from '../../../components/CreateInstanceModal.svelte';
  import { showNotification } from '../../../stores/notification';
  import { t } from '../../../stores/i18n';
  import { portal } from '../../../utils/portal';
  import './InstanceDetail.css';

  interface DownloadProgress {
    total_files: number;
    current_files: number;
    total_bytes: number;
    current_bytes: number;
    percentage: number;
    speed: number;
    remaining_time: string;
    status: string;
    current_file_name: string;
  }

  interface ScreenshotInfo {
    name: string;
    path: string;
    created: number;
  }

  interface LogFile {
    name: string;
    path: string;
    modified: number;
    category?: 'minecraft' | 'launcher' | 'crash';
    size?: number;
  }

  interface LocalMod {
    name: string;
    filename: string;
    version: string;
    enabled: boolean;
    icon_path?: string;
  }

  let id = $derived($page.params.id);

  let instance = $state<Instance | null>(null);
  let loading = $state(true);
  let launching = $state(false);
  let isRunning = $state(false);
  let activeTab = $state<'overview' | 'files' | 'mods' | 'screenshots' | 'logs' | 'settings'>('overview');

  let recentScreenshots = $state<ScreenshotInfo[]>([]);
  let recentPreviews = $state<Record<string, string>>({});
  let selectedImage = $state<string | null>(null);
  let isClosingLightbox = $state(false);
  let zoomLevel = $state(1);
  let panOffset = $state({ x: 0, y: 0 });
  let isDragging = $state(false);
  let dragStartX = 0;
  let dragStartY = 0;
  let imageElement: HTMLImageElement | null = null;

  let panelPos = $state({ x: 50, y: 90 });
  let isDraggingPanel = $state(false);
  let panelDragOffsetX = 0;
  let panelDragOffsetY = 0;

  let downloadProgress = $state<DownloadProgress | null>(null);
  let showProgressModal = $state(false);

  let logFiles = $state<LogFile[]>([]);
  let selectedLogContent = $state<string | null>(null);
  let selectedLogPath = $state<string | null>(null);
  let logSearchQuery = $state('');
  let logLevelFilter = $state<'all' | 'error' | 'warn' | 'info'>('all');
  let logWrapLines = $state(false);
  let logCopied = $state(false);
  let logScrollContainer = $state<HTMLDivElement | null>(null);
  let isReadingLog = $state(false);

  let parsedLogLines = $derived.by(() => {
    if (!selectedLogContent) return [];
    const rawLines = selectedLogContent.split('\n');
    return rawLines.map((text, idx) => {
      const upper = text.toUpperCase();
      let level: 'error' | 'warn' | 'info' | 'normal' = 'normal';
      if (
        upper.includes('/ERROR') ||
        upper.includes('[ERROR]') ||
        upper.includes('FATAL') ||
        upper.includes('CRASH') ||
        upper.includes('EXCEPTION') ||
        upper.includes('FAILED TO') ||
        text.trim().startsWith('at ')
      ) {
        level = 'error';
      } else if (upper.includes('/WARN') || upper.includes('[WARN]') || upper.includes('WARNING')) {
        level = 'warn';
      } else if (upper.includes('/INFO') || upper.includes('[INFO]')) {
        level = 'info';
      }
      return { lineNum: idx + 1, text, level };
    });
  });

  let logCounts = $derived.by(() => {
    let errors = 0;
    let warns = 0;
    let infos = 0;
    for (const l of parsedLogLines) {
      if (l.level === 'error') errors++;
      else if (l.level === 'warn') warns++;
      else if (l.level === 'info') infos++;
    }
    return { total: parsedLogLines.length, errors, warns, infos };
  });

  let filteredLogLines = $derived.by(() => {
    let list = parsedLogLines;
    if (logLevelFilter !== 'all') {
      list = list.filter((l) => l.level === logLevelFilter);
    }
    if (logSearchQuery.trim()) {
      const q = logSearchQuery.toLowerCase();
      list = list.filter((l) => l.text.toLowerCase().includes(q));
    }
    return list;
  });

  let installedMods = $state<LocalMod[]>([]);
  let modSearchQuery = $state('');
  let filteredInstalledMods = $derived.by(() => {
    if (!modSearchQuery.trim()) return installedMods;
    const q = modSearchQuery.toLowerCase();
    return installedMods.filter(
      (m) => m.name.toLowerCase().includes(q) || m.filename.toLowerCase().includes(q)
    );
  });
  let modsLoading = $state(false);

  let replacingMod = $state<LocalMod | null>(null);
  let replaceVersions = $state<ModrinthVersion[]>([]);
  let replaceLoading = $state(false);
  let isClosingReplaceModal = $state(false);
  let showVersionDropdown = $state(false);
  let isClosingDropdown = $state(false);

  let showDeleteModal = $state(false);
  let isClosingModal = $state(false);
  let showActionsMenu = $state(false);
  let showDuplicateModal = $state(false);

  let tabsElement = $state<HTMLDivElement | null>(null);
  let indicatorStyle = $state<{ width?: string; left?: string }>({});

  let unlistenDl: (() => void) | undefined;
  let unlistenStopped: (() => void) | undefined;
  let unlistenUpdated: (() => void) | undefined;

  let prevId: string | null = null;

  function resetInstanceState() {
    loading = true;
    instance = null;
    selectedImage = null;
    isClosingLightbox = false;
    logFiles = [];
    selectedLogContent = null;
    selectedLogPath = null;
    installedMods = [];
    modSearchQuery = '';
    replacingMod = null;
    showDeleteModal = false;
    showActionsMenu = false;
    showDuplicateModal = false;
    downloadProgress = null;
    showProgressModal = false;
    recentScreenshots = [];
    recentPreviews = {};
  }

  function handleClickOutside(event: MouseEvent) {
    const target = event.target as HTMLElement;
    if (!target.closest('.more-actions-container')) {
      showActionsMenu = false;
    }
  }

  $effect(() => {
    const currentId = id;
    if (currentId && currentId !== prevId) {
      prevId = currentId;
      resetInstanceState();
      loadInstance();
      checkRunningStatus();
    }
  });

  onMount(async () => {
    window.addEventListener('resize', updateIndicator);
    document.addEventListener('mousedown', handleClickOutside);

    const saved = localStorage.getItem('lightbox-panel-pos-rel');
    if (saved) {
      try { panelPos = JSON.parse(saved); } catch (e) { console.error(e); }
    }

    unlistenDl = await listen<DownloadProgress>('dl-progress', (event) => {
      downloadProgress = event.payload;
      if (event.payload.percentage === 100) {
        setTimeout(() => {
          downloadProgress = null;
          showProgressModal = false;
        }, 3000);
      }
    });

    unlistenStopped = await listen<string>('instance-stopped', (event) => {
      if (event.payload === id) {
        isRunning = false;
        loadInstance();
      }
    });

    unlistenUpdated = await listen('instances-updated', () => {
      loadInstance();
    });
  });

  onDestroy(() => {
    window.removeEventListener('resize', updateIndicator);
    document.removeEventListener('mousedown', handleClickOutside);
    if (unlistenDl) unlistenDl();
    if (unlistenStopped) unlistenStopped();
    if (unlistenUpdated) unlistenUpdated();
  });

  $effect(() => {
    const _el = tabsElement;
    const _tab = activeTab;
    const _loading = loading;
    const _inst = instance;
    tick().then(() => {
      updateIndicator();
      requestAnimationFrame(updateIndicator);
    });
    if (activeTab === 'overview' && instance) loadRecentScreenshots();
    if (activeTab === 'logs' && instance) loadLogFiles();
    if (activeTab === 'mods' && instance) loadInstalledMods();
  });

  function updateIndicator() {
    if (tabsElement) {
      const activeBtn = tabsElement.querySelector('.tab-btn.active') as HTMLElement;
      if (activeBtn) {
        indicatorStyle = {
          width: `${activeBtn.offsetWidth}px`,
          left: `${activeBtn.offsetLeft}px`
        };
      }
    }
  }

  function handleToggleDropdown(val: boolean) {
    if (!val && showVersionDropdown) {
      isClosingDropdown = true;
      setTimeout(() => {
        showVersionDropdown = false;
        isClosingDropdown = false;
      }, 200);
    } else if (val) {
      showVersionDropdown = true;
    }
  }

  async function loadInstance() {
    try {
      const instances: any[] = await invoke('get_instances');
      const found = instances.find((i) => i.id === id);
      if (found) {
        instance = {
          id: found.id,
          name: found.name,
          mcVersion: found.mc_version || found.mcVersion,
          loader: found.loader,
          loaderVersion: found.loader_version || found.loaderVersion,
          memory: found.memory,
          javaPath: found.java_path || found.javaPath,
          gameDir: found.game_dir || found.gameDir,
          created: found.created,
          lastPlayed: found.last_played || found.lastPlayed,
          icon: found.icon,
          playtime: found.playtime || 0
        };
      } else {
        goto('/');
      }
    } catch (err) {
      goto('/');
    } finally {
      loading = false;
    }
  }

  async function checkRunningStatus() {
    try {
      const running = await invoke<boolean>('is_instance_running', { id });
      isRunning = running;
    } catch (err) {
      console.error(err);
    }
  }

  async function loadInstalledMods() {
    if (!instance) return;
    modsLoading = true;
    try {
      const mods: LocalMod[] = await invoke('get_installed_mods', { instancePath: instance.gameDir });
      installedMods = mods;
    } catch (err) {
      console.error(err);
    } finally {
      modsLoading = false;
    }
  }

  async function handleToggleMod(mod: LocalMod) {
    if (!instance) return;
    try {
      await invoke('toggle_mod', { instancePath: instance.gameDir, filename: mod.filename, enable: !mod.enabled });
      loadInstalledMods();
    } catch (err) {
      showNotification(`Hiba: ${err}`, 'error');
    }
  }

  async function handleDeleteMod(filename: string) {
    if (!instance) return;
    try {
      await invoke('delete_mod', { instancePath: instance.gameDir, filename });
      showNotification($t('notify.modRemoved'), 'info');
      loadInstalledMods();
    } catch (err) {
      showNotification($t('notify.errorDelete', { err: String(err) }), 'error');
    }
  }

﻿  async function handleOpenReplace(mod: LocalMod) {
    replacingMod = mod;
    isClosingReplaceModal = false;
    handleToggleDropdown(false);
    replaceLoading = true;
    try {
      const searchResult: any = await invoke('search_mods', { query: mod.name, facets: null });
      if (searchResult.hits && searchResult.hits.length > 0) {
        const project = searchResult.hits[0];
        const versions: ModrinthVersion[] = await invoke('get_mod_versions', {
          projectId: project.project_id,
          loader: instance?.loader,
          gameVersion: instance?.mcVersion
        });
        replaceVersions = versions;
      } else {
        showNotification($t('notify.modNotFoundModrinth'), 'info');
      }
    } catch (err) {
      console.error(err);
    } finally {
      replaceLoading = false;
    }
  }

  function closeReplaceModal() {
    isClosingReplaceModal = true;
    handleToggleDropdown(false);
    setTimeout(() => {
      replacingMod = null;
      replaceVersions = [];
      isClosingReplaceModal = false;
    }, 300);
  }

  async function executeReplace(newVersionId: string) {
    if (!instance || !replacingMod || !newVersionId) return;
    modsLoading = true;
    handleToggleDropdown(false);
    try {
      await invoke('delete_mod', { instancePath: instance.gameDir, filename: replacingMod.filename });
      await invoke('install_mod', {
        instanceId: instance.id,
        instancePath: instance.gameDir,
        versionId: newVersionId
      });
      showNotification($t('notify.modReplaced'), 'success');
      closeReplaceModal();
      loadInstalledMods();
    } catch (err) {
      showNotification(`Hiba: ${err}`, 'error');
      modsLoading = false;
    }
  }

  function formatFileSize(bytes?: number): string {
    if (!bytes) return '';
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  async function loadLogFiles() {
    if (!instance) return;
    try {
      const logs: LogFile[] = [];

      // 1. Standard Minecraft logs (logs/)
      try {
        const mcLogs: any[] = await invoke('list_directory', { path: `${instance.gameDir}/logs` });
        for (const f of mcLogs) {
          if (!f.is_dir && (f.name.endsWith('.log') || f.name.endsWith('.txt'))) {
            logs.push({
              name: f.name,
              path: f.path,
              modified: f.modified,
              category: 'minecraft',
              size: f.size
            });
          }
        }
      } catch (_) {}

      // 2. Launcher logs (logs/launcher/)
      try {
        const launcherLogs: any[] = await invoke('list_directory', { path: `${instance.gameDir}/logs/launcher` });
        for (const f of launcherLogs) {
          if (!f.is_dir && (f.name.endsWith('.log') || f.name.endsWith('.txt'))) {
            if (!logs.some((l) => l.path === f.path)) {
              logs.push({
                name: `launcher/${f.name}`,
                path: f.path,
                modified: f.modified,
                category: 'launcher',
                size: f.size
              });
            }
          }
        }
      } catch (_) {}

      // 3. Crash reports (crash-reports/)
      try {
        const crashReports: any[] = await invoke('list_directory', { path: `${instance.gameDir}/crash-reports` });
        for (const f of crashReports) {
          if (!f.is_dir && (f.name.endsWith('.txt') || f.name.endsWith('.log'))) {
            logs.push({
              name: f.name,
              path: f.path,
              modified: f.modified,
              category: 'crash',
              size: f.size
            });
          }
        }
      } catch (_) {}

      // Sort newest first
      logs.sort((a, b) => b.modified - a.modified);
      logFiles = logs;

      // Automatically select newest log if none selected or if selected no longer exists
      if (logs.length > 0 && (!selectedLogPath || !logs.some((l) => l.path === selectedLogPath))) {
        readLog(logs[0].path);
      }
    } catch (err) {
      console.error('Failed to load log files:', err);
    }
  }

  async function readLog(path: string) {
    selectedLogPath = path;
    isReadingLog = true;
    try {
      const content: string = await invoke('read_text_file', { path });
      selectedLogContent = content;
      setTimeout(() => {
        if (logScrollContainer) {
          logScrollContainer.scrollTop = logScrollContainer.scrollHeight;
        }
      }, 50);
    } catch (err) {
      showNotification('Hiba a log olvasásakor: ' + err, 'error');
    } finally {
      isReadingLog = false;
    }
  }

  async function openLogFolder() {
    if (!instance) return;
    try {
      await invoke('open_folder', { path: `${instance.gameDir}/logs` });
    } catch (e) {
      showNotification($t('notify.openFolderFailed'), 'error');
    }
  }

  function scrollLogTo(position: 'top' | 'bottom') {
    if (!logScrollContainer) return;
    if (position === 'top') {
      logScrollContainer.scrollTop = 0;
    } else {
      logScrollContainer.scrollTop = logScrollContainer.scrollHeight;
    }
  }

  async function handleCopyLog() {
    if (!selectedLogContent) return;
    try {
      await navigator.clipboard.writeText(selectedLogContent);
      logCopied = true;
      showNotification($t('notify.copiedToClipboard'), 'success');
      setTimeout(() => {
        logCopied = false;
      }, 2000);
    } catch (err) {
      showNotification($t('notify.copyFailed'), 'error');
    }
  }

  async function loadRecentScreenshots() {
    if (!instance) return;
    try {
      const result: ScreenshotInfo[] = await invoke('get_screenshots', { gameDir: instance.gameDir });
      const lastThree = result.slice(0, 3);
      recentScreenshots = lastThree;
      for (const s of lastThree) {
        if (!recentPreviews[s.path]) {
          const data: string = await invoke('get_screenshot_full', { path: s.path });
          recentPreviews[s.path] = data;
        }
      }
    } catch (err) {
      console.error(err);
    }
  }

  function closeLightbox() {
    isClosingLightbox = true;
    setTimeout(() => {
      selectedImage = null;
      isClosingLightbox = false;
      zoomLevel = 1;
      panOffset = { x: 0, y: 0 };
    }, 300);
  }

  async function handleCopyImage() {
    if (!selectedImage) return;
    try {
      await navigator.clipboard.writeText(selectedImage);
      showNotification($t('notify.copiedToClipboard'), 'success');
    } catch (err) {
      showNotification($t('notify.copyFailed'), 'error');
    }
  }

﻿  function handleMouseDown(e: MouseEvent) {
    if (zoomLevel > 1) {
      isDragging = true;
      dragStartX = e.clientX - panOffset.x;
      dragStartY = e.clientY - panOffset.y;
    }
  }

  function handleMouseMove(e: MouseEvent) {
    if (isDragging && zoomLevel > 1 && imageElement) {
      const newX = e.clientX - dragStartX;
      const newY = e.clientY - dragStartY;
      const rect = imageElement.getBoundingClientRect();
      const container = imageElement.parentElement;
      if (!container) return;
      const contRect = container.getBoundingClientRect();
      const maxX = rect.width > contRect.width ? (rect.width - contRect.width) / 2 : 0;
      const maxY = rect.height > contRect.height ? (rect.height - contRect.height) / 2 : 0;
      panOffset = {
        x: Math.max(-maxX, Math.min(maxX, newX)),
        y: Math.max(-maxY, Math.min(maxY, newY))
      };
    }
  }

  function handleMouseUp() {
    isDragging = false;
  }

  function handlePanelMouseDown(e: MouseEvent) {
    isDraggingPanel = true;
    const mouseXPercent = (e.clientX / window.innerWidth) * 100;
    const mouseYPercent = (e.clientY / window.innerHeight) * 100;
    panelDragOffsetX = mouseXPercent - panelPos.x;
    panelDragOffsetY = mouseYPercent - panelPos.y;
    e.stopPropagation();
  }

  function handleGlobalMouseMove(e: MouseEvent) {
    if (isDraggingPanel) {
      const mouseXPercent = (e.clientX / window.innerWidth) * 100;
      const mouseYPercent = (e.clientY / window.innerHeight) * 100;
      panelPos = {
        x: Math.max(5, Math.min(95, mouseXPercent - panelDragOffsetX)),
        y: Math.max(5, Math.min(95, mouseYPercent - panelDragOffsetY))
      };
    } else {
      handleMouseMove(e);
    }
  }

  function handleGlobalMouseUp() {
    if (isDraggingPanel) {
      isDraggingPanel = false;
      localStorage.setItem('lightbox-panel-pos-rel', JSON.stringify(panelPos));
    }
    handleMouseUp();
  }

  function formatPlaytime(seconds: number) {
    if (!seconds) return $t('time.mins', { n: 0 });
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    return h > 0 ? `${$t('time.hours', { n: h })} ${$t('time.mins', { n: m })}` : $t('time.mins', { n: m });
  }

  function formatSize(bytes: number) {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  }

  async function handleLaunch() {
    if (!instance) return;
    if (isRunning) {
      try {
        await invoke('kill_instance', { id: instance.id });
        isRunning = false;
        showNotification($t('notify.gameStopped'), 'info');
      } catch (err) {
        showNotification(`${$t('detail.error')}: ${err}`, 'error');
      }
      return;
    }
    const activeProfileId = localStorage.getItem('activeProfileId');
    if (!activeProfileId) {
      goto('/profiles');
      return;
    }
    try {
      launching = true;
      await invoke('download_instance', {
        instance: {
          ...instance,
          mc_version: instance.mcVersion,
          loader_version: instance.loaderVersion,
          java_path: instance.javaPath,
          game_dir: instance.gameDir
        }
      });
      const profiles: Profile[] = await invoke('get_profiles');
      const profile = profiles.find((p) => p.id === activeProfileId);
      if (!profile) throw new Error('Profil nem található');
      await invoke('launch_instance', { id: instance.id, profile });
      isRunning = true;
      launching = false;
    } catch (err) {
      console.error('launch-error', err);
      showNotification(`${err}`, 'error');
      launching = false;
    }
  }

  async function handleOpenFolder() {
    if (instance) await invoke('open_folder', { path: instance.gameDir });
    showActionsMenu = false;
  }

  function browseModsForInstance() {
    showActionsMenu = false;
    if (!instance) {
      goto('/mods');
      return;
    }
    const params = new URLSearchParams();
    if (instance.loader && instance.loader.toLowerCase() !== 'vanilla') {
      params.set('loader', instance.loader.toLowerCase());
    }
    if (instance.mcVersion) {
      params.set('version', instance.mcVersion);
    }
    params.set('instance', instance.id);
    goto(`/mods?${params.toString()}`);
  }

  function closeDeleteModal() {
    isClosingModal = true;
    setTimeout(() => {
      showDeleteModal = false;
      isClosingModal = false;
      showActionsMenu = false;
    }, 300);
  }

  let isDeleting = $state(false);

  async function confirmDelete() {
    if (!instance || isDeleting) return;
    isDeleting = true;
    try {
      await invoke('delete_instance', { id: instance.id });
      goto('/');
    } catch (err) {
      showNotification(`Hiba: ${err}`, 'error');
      isDeleting = false;
    }
  }

  async function handleSelectGameDir() {
    if (!instance) return;
    try {
      const newPath = await invoke<string | null>('select_folder');
      if (newPath) {
        const updated = { ...instance, gameDir: newPath };
        await invoke('update_instance', {
          instance: {
            ...updated,
            mc_version: updated.mcVersion,
            loader_version: updated.loaderVersion,
            java_path: updated.javaPath,
            game_dir: updated.gameDir
          }
        });
        instance = updated;
      }
    } catch (err) {
      console.error(err);
    }
  }

  async function handleSelectIcon() {
    if (!instance) return;
    try {
      const newPath = await invoke<string | null>('select_file', {
        title: $t('detail.selectIconTitle'),
        filterName: $t('detail.imagesFilter'),
        filterExt: '*'
      });
      if (newPath) {
        const base64Data = await invoke<string>('get_screenshot_full', { path: newPath });
        await invoke('update_instance', {
          instance: {
            ...instance,
            icon: base64Data,
            mc_version: instance.mcVersion,
            loader_version: instance.loaderVersion,
            java_path: instance.javaPath,
            game_dir: instance.gameDir
          }
        });
        instance.icon = base64Data;
      }
    } catch (err) {
      console.error(err);
    }
  }

  async function resetGameDir() {
    if (!instance) return;
    try {
      const settings: any = await invoke('get_settings');
      const updated = { ...instance, gameDir: `${settings.launcher_dir}/instances/${instance.name}/.minecraft` };
      await invoke('update_instance', {
        instance: {
          ...updated,
          mc_version: updated.mcVersion,
          loader_version: updated.loaderVersion,
          java_path: updated.javaPath,
          game_dir: updated.gameDir
        }
      });
      instance = updated;
    } catch (err) {
      console.error(err);
    }
  }

  async function handleSelectJava() {
    if (!instance) return;
    try {
      const newPath = await invoke<string | null>('select_file', {
        title: 'Java Executable',
        filterName: 'Java',
        filterExt: '*'
      });
      if (newPath) {
        const updated = { ...instance, javaPath: newPath };
        await invoke('update_instance', {
          instance: {
            ...updated,
            mc_version: updated.mcVersion,
            loader_version: updated.loaderVersion,
            java_path: updated.javaPath,
            game_dir: updated.gameDir
          }
        });
        instance = updated;
      }
    } catch (err) {
      console.error(err);
    }
  }

  async function updateMemory(val: number) {
    if (!instance) return;
    const updated = { ...instance, memory: val };
    await invoke('update_instance', {
      instance: {
        ...updated,
        mc_version: updated.mcVersion,
        loader_version: updated.loaderVersion,
        java_path: updated.javaPath,
        game_dir: updated.gameDir
      }
    });
    instance = updated;
  }
</script>

﻿{#if loading}
  <div class="instance-detail-loading">{$t('dash.loading')}</div>
{:else if instance}
  <div class="instance-detail">
    <div class="detail-header">
      <button class="back-btn" onclick={() => goto('/')}>
        <ChevronLeft size={20} /><span>{$t('detail.back')}</span>
      </button>

      <div class="header-main-content">
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="instance-large-icon" onclick={handleSelectIcon}>
          {#if instance.icon}
            <img src={instance.icon} alt="" />
          {:else}
            <Box size={48} />
          {/if}
          <div class="icon-edit-overlay"><Camera size={24} /></div>
        </div>

        <div class="instance-info-main">
          <div class="info-text-wrapper">
            <h1>{instance.name}</h1>
            <div class="version-badges">
              <span class="badge version">{instance.mcVersion}</span>
              <span class="badge loader">{instance.loader}</span>
            </div>

            {#if downloadProgress}
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <div
                class="process-status-widget fade-in"
                onclick={() => (showProgressModal = true)}
                role="button"
                tabindex="0"
                title={$t('detail.clickDetails')}
              >
                <div class="widget-icon-box">
                  <div class="widget-pulse-glow"></div>
                  <Loader2 size={16} class="spin widget-spin-icon" />
                </div>
                <div class="widget-body">
                  <div class="widget-row-top">
                    <span class="widget-status-text">{downloadProgress.status || $t('detail.downloading')}</span>
                    <span class="widget-pct-pill">{Math.round(downloadProgress.percentage)}%</span>
                  </div>
                  <div class="widget-progress-track">
                    <div class="widget-progress-fill" style={`width: ${downloadProgress.percentage}%`}></div>
                  </div>
                  <div class="widget-row-bottom">
                    <div class="widget-metrics">
                      {#if downloadProgress.speed > 0}
                        <span class="widget-metric speed"><Zap size={11} /> {downloadProgress.speed.toFixed(1)} MB/s</span>
                      {/if}
                      {#if downloadProgress.remaining_time && downloadProgress.remaining_time !== '0 mp'}
                        <span class="widget-metric eta"><Clock size={11} /> {downloadProgress.remaining_time}</span>
                      {/if}
                      <span class="widget-metric files"><Box size={11} /> {downloadProgress.current_files}/{downloadProgress.total_files}</span>
                    </div>
                    <div class="widget-action-link">
                      <span>{$t('dash.details')}</span>
                      <ChevronRight size={13} />
                    </div>
                  </div>
                </div>
              </div>
            {/if}
          </div>
        </div>

        <div class="header-actions">
          <div class="play-row">
            <button
              class="icon-action-btn circle"
              onclick={browseModsForInstance}
              title={$t('detail.browseMods')}
            >
              <Download size={20} />
            </button>

            <div class="more-actions-container">
              <button
                class={`icon-action-btn circle ${showActionsMenu ? 'active' : ''}`}
                onclick={() => (showActionsMenu = !showActionsMenu)}
                aria-label={$t('detail.moreActions')}
              >
                <MoreVertical size={20} />
              </button>
              {#if showActionsMenu}
                <div class="actions-dropdown fade-in">
                  <button
                    class="dropdown-item"
                    onclick={() => {
                      showActionsMenu = false;
                      showDuplicateModal = true;
                    }}
                  >
                    <Copy size={16} /><span>{$t('detail.duplicate')}</span>
                  </button>
                  <button class="dropdown-item" onclick={browseModsForInstance}>
                    <Download size={16} /><span>{$t('detail.browseMods')}</span>
                  </button>
                  <button class="dropdown-item" onclick={handleOpenFolder}>
                    <FolderOpen size={16} /><span>{$t('detail.openFolder')}</span>
                  </button>
                  <button class="dropdown-item danger" onclick={() => (showDeleteModal = true)}>
                    <Trash2 size={16} /><span>{$t('detail.delete')}</span>
                  </button>
                </div>
              {/if}
            </div>

            <button
              class={`play-btn ${launching || (downloadProgress && downloadProgress.percentage < 100) ? 'launching' : ''} ${isRunning ? 'stop' : ''}`}
              onclick={handleLaunch}
              disabled={launching || (!!downloadProgress && downloadProgress.percentage < 100)}
            >
              {#if launching || (!!downloadProgress && downloadProgress.percentage < 100)}
                <Loader2 class="spin" />
              {:else if isRunning}
                <Square size={20} fill="currentColor" />
              {:else}
                <Play size={20} fill="currentColor" />
              {/if}
              <span>
                {launching || (!!downloadProgress && downloadProgress.percentage < 100)
                  ? downloadProgress
                    ? $t('detail.downloading')
                    : $t('detail.launching')
                  : isRunning
                    ? $t('detail.stop')
                    : $t('detail.play')}
              </span>
            </button>
          </div>
        </div>
      </div>
    </div>

    <div class="tabs-container">
      <div class="detail-tabs" bind:this={tabsElement}>
        <div class="tab-indicator" style={`width: ${indicatorStyle.width || '0px'}; left: ${indicatorStyle.left || '0px'};`}></div>
        <button class={`tab-btn ${activeTab === 'overview' ? 'active' : ''} ${!indicatorStyle.width || indicatorStyle.width === '0px' ? 'fallback-active' : ''}`} onclick={() => (activeTab = 'overview')}>{$t('detail.tabOverview')}</button>
        <button class={`tab-btn ${activeTab === 'files' ? 'active' : ''} ${!indicatorStyle.width || indicatorStyle.width === '0px' ? 'fallback-active' : ''}`} onclick={() => (activeTab = 'files')}>{$t('detail.tabFiles')}</button>
        <button class={`tab-btn ${activeTab === 'screenshots' ? 'active' : ''} ${!indicatorStyle.width || indicatorStyle.width === '0px' ? 'fallback-active' : ''}`} onclick={() => (activeTab = 'screenshots')}>{$t('detail.tabScreenshots')}</button>
        <button class={`tab-btn ${activeTab === 'mods' ? 'active' : ''} ${!indicatorStyle.width || indicatorStyle.width === '0px' ? 'fallback-active' : ''}`} onclick={() => (activeTab = 'mods')}>{$t('detail.tabMods')}</button>
        <button class={`tab-btn ${activeTab === 'logs' ? 'active' : ''} ${!indicatorStyle.width || indicatorStyle.width === '0px' ? 'fallback-active' : ''}`} onclick={() => (activeTab = 'logs')}>{$t('detail.tabLogs')}</button>
        <button class={`tab-btn ${activeTab === 'settings' ? 'active' : ''} ${!indicatorStyle.width || indicatorStyle.width === '0px' ? 'fallback-active' : ''}`} onclick={() => (activeTab = 'settings')}>{$t('detail.tabSettings')}</button>
      </div>
    </div>

    <div class="tab-content fade-in">
      {#if activeTab === 'overview'}
        <div class="overview-grid">
          <div class="info-card stats">
            <h3>{$t('detail.statistics')}</h3>
            <div class="stat-row">
              <Clock size={18} />
              <div class="stat-text">
                <span class="label">{$t('detail.playtime')}</span>
                <span class="value">{formatPlaytime(instance.playtime || 0)}</span>
              </div>
            </div>
            <div class="stat-row">
              <Clock size={18} />
              <div class="stat-text">
                <span class="label">{$t('detail.lastPlayed')}</span>
                <span class="value">{instance.lastPlayed ? new Date(instance.lastPlayed).toLocaleDateString() : $t('time.never')}</span>
              </div>
            </div>
            <div class="stat-row">
              <ShieldCheck size={18} />
              <div class="stat-text">
                <span class="label">{$t('detail.created')}</span>
                <span class="value">{new Date(instance.created).toLocaleDateString()}</span>
              </div>
            </div>
          </div>

          <div class="info-card recent-screenshots">
            <div class="card-header-with-action">
              <h3>{$t('detail.recentScreenshots')}</h3>
              {#if recentScreenshots.length > 0}
                <button class="view-all-link-styled" onclick={() => (activeTab = 'screenshots')}>
                  {$t('detail.viewAll')}
                </button>
              {/if}
            </div>
            {#if recentScreenshots.length === 0}
              <div class="empty-recent">
                <ImageIcon size={32} />
                <p>{$t('detail.noScreenshots')}</p>
              </div>
            {:else}
              <div class="recent-previews-grid">
                {#each recentScreenshots as s (s.path)}
                  <!-- svelte-ignore a11y_click_events_have_key_events -->
                  <!-- svelte-ignore a11y_no_static_element_interactions -->
                  <div
                    class="recent-preview-card"
                    onclick={() => {
                      selectedImage = recentPreviews[s.path];
                      isClosingLightbox = false;
                    }}
                  >
                    {#if recentPreviews[s.path]}
                      <img src={recentPreviews[s.path]} alt="" />
                    {:else}
                      <div class="placeholder-shimmer"></div>
                    {/if}
                    <div class="card-overlay"><Maximize2 size={24} /></div>
                  </div>
                {/each}
              </div>
            {/if}
          </div>
        </div>

      {:else if activeTab === 'files'}
        <FileManager basePath={instance.gameDir} />
      {:else if activeTab === 'screenshots'}
        <ScreenshotGallery gameDir={instance.gameDir} />
      {:else if activeTab === 'mods'}
        <div class="mods-tab-view fade-in">
          {#if instance.loader === 'vanilla'}
            <NoMods />
          {:else}
            <div class="mods-manager">
              <div class="mods-header-row">
                <div class="mods-stats">
                  <Box size={20} class="icon-green" />
                  <span>{$t('mods.installedModsCount', { n: installedMods.length })}</span>
                </div>

                {#if installedMods.length > 0}
                  <div class="mods-search-input-wrapper">
                    <Search size={15} />
                    <input
                      type="text"
                      placeholder={$t('mods.searchInstalledPlaceholder')}
                      bind:value={modSearchQuery}
                    />
                    {#if modSearchQuery}
                      <button class="clear-search-btn" onclick={() => (modSearchQuery = '')} aria-label={$t('mods.clearSearch')}>
                        <X size={13} />
                      </button>
                    {/if}
                  </div>
                {/if}

                <button class="btn btn-primary btn-small" onclick={browseModsForInstance}>
                  <Download size={16} /> <span>{$t('detail.browseMods')}</span>
                </button>
              </div>

              <div class="mods-container-fixed">
                {#if modsLoading}
                  <div class="mods-loading">
                    <Loader2 class="spin" /> <span>{$t('mods.pleaseWait')}</span>
                  </div>
                {:else if installedMods.length === 0}
                  <div class="empty-state-centered">
                    <div class="empty-box">
                      <Box size={48} class="icon-dim" />
                      <p>{$t('mods.noInstalledMods')}</p>
                      <button class="btn btn-secondary btn-small" onclick={browseModsForInstance}>
                        {$t('mods.browseModsBtn')}
                      </button>
                    </div>
                  </div>
                {:else if filteredInstalledMods.length === 0}
                  <div class="empty-state-centered">
                    <div class="empty-box">
                      <Search size={36} class="icon-dim" />
                      <p>{$t('mods.noSearchResult', { query: modSearchQuery })}</p>
                      <button class="btn btn-secondary btn-small" onclick={() => (modSearchQuery = '')}>
                        {$t('mods.clearSearch')}
                      </button>
                    </div>
                  </div>
                {:else}
                  <div class="mods-grid-layout">
                    {#each filteredInstalledMods as mod (mod.filename)}
                      <div class={`local-mod-card ${!mod.enabled ? 'disabled' : ''}`}>
                        <div class="mod-card-left">
                          <div class="mod-icon-wrapper">
                            {#if mod.icon_path}
                              <img src={mod.icon_path} alt="" class="local-mod-icon" />
                            {:else}
                              <Box size={18} />
                            {/if}
                          </div>
                          <div class="mod-info">
                            <div class="mod-name-row">
                              <span class="mod-name" title={mod.name}>{mod.name}</span>
                              {#if mod.version}
                                <span class="mod-version-badge" title={mod.version}>{mod.version}</span>
                              {/if}
                            </div>
                            <span class="mod-filename" title={mod.filename}>{mod.filename}</span>
                          </div>
                        </div>

                        <div class="mod-actions">
                          <button
                            class="mod-action-icon-btn"
                            title={$t('mods.replaceTitle')}
                            onclick={() => handleOpenReplace(mod)}
                          >
                            <RefreshCcw size={13} />
                          </button>
                          <button
                            class="mod-action-icon-btn danger"
                            title={$t('detail.delete')}
                            onclick={() => handleDeleteMod(mod.filename)}
                          >
                            <Trash size={13} />
                          </button>
                          <button
                            class={`mod-toggle-btn ${mod.enabled ? 'on' : 'off'}`}
                            onclick={() => handleToggleMod(mod)}
                            title={mod.enabled ? $t('mods.toggleOff') : $t('mods.toggleOn')}
                          >
                            <Power size={13} />
                          </button>
                        </div>
                      </div>
                    {/each}
                  </div>
                {/if}
              </div>
            </div>
          {/if}
        </div>
      {:else if activeTab === 'logs'}
        <div class="logs-view fade-in">
          <div class="logs-sidebar">
            <div class="logs-sidebar-header">
              <div class="logs-sidebar-title">
                <FileText size={16} class="icon-green" />
                <span>{$t('detail.logsTitle')}</span>
                <span class="log-count-badge">{logFiles.length}</span>
              </div>
              <div class="logs-sidebar-actions">
                <button
                  type="button"
                  class="icon-btn-micro"
                  onclick={loadLogFiles}
                  title={$t('detail.refreshLogs')}
                >
                  <RefreshCcw size={14} />
                </button>
                <button
                  type="button"
                  class="icon-btn-micro"
                  onclick={openLogFolder}
                  title={$t('detail.openLogFolder')}
                >
                  <FolderOpen size={14} />
                </button>
              </div>
            </div>

            <div class="logs-list custom-scrollbar">
              {#if logFiles.length === 0}
                <div class="empty-logs-sidebar">
                  <FileText size={32} class="icon-dim" />
                  <p>{$t('detail.noLogFiles')}</p>
                  <button class="btn btn-secondary btn-small" onclick={openLogFolder}>
                    {$t('detail.openFolder')}
                  </button>
                </div>
              {:else}
                {#each logFiles as log (log.path)}
                  <button
                    type="button"
                    class="log-item"
                    class:active={selectedLogPath === log.path}
                    onclick={() => readLog(log.path)}
                  >
                    <div class="log-item-icon">
                      {#if log.category === 'crash'}
                        <AlertTriangle size={16} class="icon-red" />
                      {:else if log.category === 'launcher'}
                        <Terminal size={16} class="icon-blue" />
                      {:else}
                        <FileText size={16} class="icon-green" />
                      {/if}
                    </div>

                    <div class="log-item-info">
                      <div class="log-item-header">
                        <span class="log-name" title={log.name}>{log.name}</span>
                        {#if log.category === 'crash'}
                          <span class="log-badge-crash">CRASH</span>
                        {:else if log.name === 'latest.log'}
                          <span class="log-badge-latest">{$t('detail.newBadge')}</span>
                        {/if}
                      </div>
                      <div class="log-meta">
                        <span class="log-date">{new Date(log.modified * 1000).toLocaleString(undefined, { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' })}</span>
                        {#if log.size}
                          <span class="log-size">{formatFileSize(log.size)}</span>
                        {/if}
                      </div>
                    </div>
                  </button>
                {/each}
              {/if}
            </div>
          </div>

          <div class="log-content-area">
            {#if isReadingLog}
              <div class="log-loading-box">
                <Loader2 size={32} class="spin icon-green" />
                <span>{$t('detail.loadingLog')}</span>
              </div>
            {:else if selectedLogContent !== null}
              <div class="log-viewer-container">
                <div class="log-viewer-header">
                  <div class="log-header-left">
                    <span class="log-file-title" title={selectedLogPath}>
                      {selectedLogPath?.split(/[\\/]/).pop()}
                    </span>
                    <span class="log-stats-badge">
                      {$t('detail.linesCount', { n: parsedLogLines.length })}
                    </span>
                  </div>

                  <!-- Quick Search in Log -->
                  <div class="log-search-bar">
                    <Search size={14} class="search-icon" />
                    <input
                      type="text"
                      placeholder={$t('detail.searchLinesPlaceholder')}
                      bind:value={logSearchQuery}
                      class="log-search-input"
                    />
                    {#if logSearchQuery}
                      <button type="button" class="clear-search-btn" onclick={() => (logSearchQuery = '')}>
                        <X size={12} />
                      </button>
                    {/if}
                  </div>

                  <!-- Level Filters -->
                  <div class="log-filters">
                    <button
                      type="button"
                      class="filter-pill"
                      class:active={logLevelFilter === 'all'}
                      onclick={() => (logLevelFilter = 'all')}
                    >
                      {$t('detail.filterAll')} <span class="pill-count">{logCounts.total}</span>
                    </button>
                    {#if logCounts.errors > 0}
                      <button
                        type="button"
                        class="filter-pill pill-error"
                        class:active={logLevelFilter === 'error'}
                        onclick={() => (logLevelFilter = 'error')}
                      >
                        {$t('detail.filterError')} <span class="pill-count">{logCounts.errors}</span>
                      </button>
                    {/if}
                    {#if logCounts.warns > 0}
                      <button
                        type="button"
                        class="filter-pill pill-warn"
                        class:active={logLevelFilter === 'warn'}
                        onclick={() => (logLevelFilter = 'warn')}
                      >
                        {$t('detail.filterWarn')} <span class="pill-count">{logCounts.warns}</span>
                      </button>
                    {/if}
                  </div>

                  <!-- Header Actions -->
                  <div class="log-viewer-actions">
                    <button
                      type="button"
                      class="btn-icon-action"
                      class:active={logWrapLines}
                      onclick={() => (logWrapLines = !logWrapLines)}
                      title={$t('detail.toggleWrap')}
                    >
                      <AlignLeft size={16} />
                    </button>
                    <button
                      type="button"
                      class="btn-icon-action"
                      onclick={() => scrollLogTo('top')}
                      title={$t('detail.scrollTop')}
                    >
                      <ArrowUp size={16} />
                    </button>
                    <button
                      type="button"
                      class="btn-icon-action"
                      onclick={() => scrollLogTo('bottom')}
                      title={$t('detail.scrollBottom')}
                    >
                      <ArrowDown size={16} />
                    </button>
                    <button
                      type="button"
                      class="btn btn-secondary btn-small"
                      onclick={handleCopyLog}
                      title={$t('detail.copyLog')}
                    >
                      {#if logCopied}
                        <Check size={14} class="icon-green" /> <span>{$t('detail.copied')}</span>
                      {:else}
                        <Copy size={14} /> <span>{$t('detail.copyLog')}</span>
                      {/if}
                    </button>
                  </div>
                </div>

                <!-- Log content scroll container -->
                <div
                  class="log-content-scroll custom-scrollbar"
                  class:wrap-lines={logWrapLines}
                  bind:this={logScrollContainer}
                >
                  {#if filteredLogLines.length === 0}
                    <div class="no-matching-lines">
                      {#if logSearchQuery}
                        <p>{$t('detail.noMatchingLines', { query: logSearchQuery })}</p>
                        <button class="btn btn-secondary btn-small" onclick={() => (logSearchQuery = '')}>
                          {$t('mods.clearSearch')}
                        </button>
                      {:else}
                        <p>{$t('detail.noFilterLines', { filter: logLevelFilter })}</p>
                        <button class="btn btn-secondary btn-small" onclick={() => (logLevelFilter = 'all')}>
                          {$t('detail.showAllLines')}
                        </button>
                      {/if}
                    </div>
                  {:else}
                    <div class="log-lines-table">
                      {#each filteredLogLines as line (line.lineNum)}
                        <div class="log-line-row {line.level}">
                          <span class="log-line-num">{line.lineNum}</span>
                          <span class="log-line-text selectable-text">{line.text}</span>
                        </div>
                      {/each}
                    </div>
                  {/if}
                </div>
              </div>
            {:else}
              <div class="log-placeholder">
                <div class="placeholder-icon-circle"><FileText size={48} /></div>
                <p>{$t('detail.selectLogPlaceholder')}</p>
              </div>
            {/if}
          </div>
        </div>
      {:else if activeTab === 'settings'}
        <div class="instance-settings-view">
          <div class="info-card settings-card">
            <h3>{$t('detail.configTitle')}</h3>
            <div class="setting-item-group">
              <div class="setting-item">
                <label>{$t('detail.changeIcon')}</label>
                <button class="action-btn-styled" onclick={handleSelectIcon}>
                  <ImageIcon size={18} />
                  <span>{$t('detail.selectIconBtn')}</span>
                </button>
              </div>
              <div class="setting-item">
                <div class="memory-control-wrapper">
                  <div class="memory-header">
                    <label for="mem-input">{$t('detail.memorySize')}</label>
                    <div class="memory-display-box">
                      <input
                        id="mem-input-number"
                        type="number"
                        class="memory-number-input"
                        value={instance.memory}
                        onchange={(e) => {
                          const val = Number((e.target as HTMLInputElement).value);
                          if (val >= 1024) {
                            updateMemory(val);
                          }
                        }}
                        min={1024}
                        max={65536}
                        step={512}
                      />
                      <span class="memory-unit">MB</span>
                      <span class="memory-gb-pill">{(instance.memory / 1024).toFixed(1)} GB</span>
                    </div>
                  </div>

                  <div class="range-slider-wrapper">
                    <div class="slider-track-container">
                      <div
                        class="slider-thumb-tooltip"
                        style="left: calc({Math.min(100, Math.max(0, ((instance.memory - 1024) / (32768 - 1024)) * 100))}% + {(0.5 - Math.min(1, Math.max(0, (instance.memory - 1024) / (32768 - 1024)))) * 22}px);"
                      >
                        <span class="tooltip-gb">{(instance.memory / 1024).toFixed(1)} GB</span>
                        <span class="tooltip-mb">({instance.memory} MB)</span>
                      </div>
                      <input
                        id="mem-input"
                        type="range"
                        class="ram-slider"
                        value={instance.memory}
                        oninput={(e) => {
                          const val = Number((e.target as HTMLInputElement).value);
                          if (instance) instance.memory = val;
                        }}
                        onchange={(e) => {
                          const val = Number((e.target as HTMLInputElement).value);
                          updateMemory(val);
                        }}
                        min={1024}
                        max={32768}
                        step={512}
                        style="background: linear-gradient(to right, var(--accent-green) 0%, var(--accent-green) {Math.min(100, Math.max(0, ((instance.memory - 1024) / (32768 - 1024)) * 100))}%, rgba(255, 255, 255, 0.1) {Math.min(100, Math.max(0, ((instance.memory - 1024) / (32768 - 1024)) * 100))}%, rgba(255, 255, 255, 0.1) 100%)"
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
                          class:active={Math.abs(instance.memory - mark.mb) < 256}
                          onclick={() => {
                            if (instance) instance.memory = mark.mb;
                            updateMemory(mark.mb);
                          }}
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
                        class:active={instance.memory === preset}
                        onclick={() => updateMemory(preset)}
                      >
                        {preset / 1024} GB
                      </button>
                    {/each}
                  </div>
                </div>
              </div>
              <div class="setting-item">
                <label>{$t('detail.gameData')}</label>
                <div class="input-with-button">
                  <input type="text" value={instance.gameDir} readonly />
                  <button class="icon-btn-small" onclick={handleSelectGameDir} title={$t('detail.changeFolder')}>
                    <FolderOpen size={16} />
                  </button>
                  <button class="icon-btn-small" onclick={resetGameDir} title={$t('detail.resetFolder')}>
                    <RotateCcw size={16} />
                  </button>
                </div>
              </div>
              <div class="setting-item">
                <label>{$t('detail.javaPath')}</label>
                <div class="input-with-button">
                  <input type="text" value={instance.javaPath || $t('detail.defaultJava')} readonly />
                  <button class="icon-btn-small" onclick={handleSelectJava} title={$t('detail.selectJava')}>
                    <Coffee size={16} />
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>
      {/if}
    </div>

    {#if replacingMod}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class={`modal-overlay replace-pill-overlay ${isClosingReplaceModal ? 'closing' : ''}`}
        use:portal
        onclick={closeReplaceModal}
      >
        <div class="replace-pill-container" onclick={(e) => e.stopPropagation()}>
          <div class={`replace-pill-bar ${isClosingReplaceModal ? 'closing' : ''}`}>
            <div class="pill-section mod-name-section">
              <RefreshCcw size={18} class="pill-icon spin-slow" />
              <span class="pill-label">{replacingMod.name}</span>
            </div>
            <div class="pill-divider"></div>
            <div class="pill-section select-section">
              <div
                class={`pill-custom-dropdown ${showVersionDropdown ? 'active' : ''}`}
                onclick={() => !replaceLoading && handleToggleDropdown(!showVersionDropdown)}
              >
                <span class="current-selection">
                  {replaceLoading ? $t('mods.searching') : $t('mods.chooseVersion')}
                </span>
                <ChevronDown size={16} class={`pill-arrow ${showVersionDropdown ? 'rotate' : ''}`} />

                {#if showVersionDropdown}
                  <div class={`pill-dropdown-menu picker-body-scroll ${isClosingDropdown ? 'closing' : ''}`}>
                    {#each replaceVersions as v (v.id)}
                      <!-- svelte-ignore a11y_click_events_have_key_events -->
                      <!-- svelte-ignore a11y_no_static_element_interactions -->
                      <div
                        class="pill-dropdown-item"
                        onclick={(e) => {
                          e.stopPropagation();
                          executeReplace(String(v.id));
                        }}
                      >
                        <span class="v-full-label">{replacingMod.name} {v.version_number}</span>
                        <ChevronRight size={14} class="v-arrow" />
                      </div>
                    {/each}
                    {#if replaceVersions.length === 0 && !replaceLoading}
                      <div class="pill-dropdown-empty">{$t('mods.noAvailableVersions')}</div>
                    {/if}
                  </div>
                {/if}
              </div>
            </div>
            <button class="pill-close-btn" onclick={closeReplaceModal} aria-label="Close">
              <X size={18} />
            </button>
          </div>
        </div>
      </div>
    {/if}

    {#if showProgressModal && downloadProgress}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="modal-overlay" use:portal onclick={() => (showProgressModal = false)}>
        <div class="modal-content progress-details-modal" onclick={(e) => e.stopPropagation()}>
          <div class="modal-header">
            <div class="header-title">
              <div class="modal-header-icon-badge">
                <Download size={20} class="icon-green" />
              </div>
              <div class="modal-header-texts">
                <h2>{$t('detail.processStatus')}</h2>
                <span class="modal-header-sub">{instance.name} • Minecraft {instance.mcVersion} ({instance.loader})</span>
              </div>
            </div>
            <button class="close-btn" onclick={() => (showProgressModal = false)} aria-label={$t('settings.cancel')}>
              <X size={20} />
            </button>
          </div>
          <div class="modal-body">
            <!-- Progress Bar Hero -->
            <div class="progress-hero-banner">
              <div class="hero-status-row">
                <span class="hero-status-tag">{downloadProgress.status || $t('detail.downloading')}</span>
                <span class="hero-pct-val">{Math.round(downloadProgress.percentage)}%</span>
              </div>
              <div class="hero-bar-track">
                <div class="hero-bar-fill" style={`width: ${downloadProgress.percentage}%`}></div>
              </div>
            </div>

            <!-- Stats Grid -->
            <div class="progress-stats-grid">
              <div class="prog-stat">
                <div class="stat-header">
                  <Box size={14} class="stat-icon" />
                  <span class="label">{$t('detail.progressFiles')}</span>
                </div>
                <span class="value">{downloadProgress.current_files} / {downloadProgress.total_files}</span>
              </div>

              <div class="prog-stat">
                <div class="stat-header">
                  <HardDrive size={14} class="stat-icon" />
                  <span class="label">{$t('detail.progressData')}</span>
                </div>
                <span class="value">
                  {#if downloadProgress.total_bytes > 1}
                    {formatSize(downloadProgress.current_bytes)} / {formatSize(downloadProgress.total_bytes)}
                  {:else}
                    {formatSize(downloadProgress.current_bytes)}
                  {/if}
                </span>
              </div>

              <div class="prog-stat">
                <div class="stat-header">
                  <Zap size={14} class="stat-icon icon-green" />
                  <span class="label">{$t('detail.progressSpeed')}</span>
                </div>
                <span class="value icon-green">
                  {downloadProgress.speed > 0 ? downloadProgress.speed.toFixed(2) + ' MB/s' : '--'}
                </span>
              </div>

              <div class="prog-stat">
                <div class="stat-header">
                  <Clock size={14} class="stat-icon" />
                  <span class="label">{$t('detail.progressRemaining')}</span>
                </div>
                <span class="value">
                  {downloadProgress.remaining_time || '--'}
                </span>
              </div>
            </div>

            <!-- Current File -->
            <div class="current-file-section">
              <div class="current-file-heading">
                <FileText size={14} />
                <span>{$t('detail.currentFile')}</span>
              </div>
              <div class="file-name-scroll">
                <code>{downloadProgress.current_file_name || $t('mods.preparingFiles')}</code>
              </div>
            </div>

            <!-- Modal Footer -->
            <div class="progress-modal-footer">
              <p class="footer-hint-text">
                {$t('detail.bgHint')}
              </p>
              <button type="button" class="btn btn-secondary btn-small" onclick={() => (showProgressModal = false)}>
                {$t('detail.hideBtn')}
              </button>
            </div>
          </div>
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
            <h2>{$t('detail.deleteTitle')}</h2>
          </div>
          <p class="modal-desc centered">{$t('detail.deleteConfirm')}</p>
          <div class="modal-actions spaced">
            <button type="button" class="btn btn-secondary flex-1" onclick={closeDeleteModal} disabled={isDeleting}>{$t('settings.cancel')}</button>
            <button type="button" class="btn btn-danger flex-1" onclick={confirmDelete} disabled={isDeleting}>
              {isDeleting ? $t('detail.deleting') : $t('detail.delete')}
            </button>
          </div>
        </div>
      </div>
    {/if}

    {#if selectedImage}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class={`image-lightbox-overlay fade-in ${isClosingLightbox ? 'closing' : ''}`}
        use:portal
        onmousemove={handleGlobalMouseMove}
        onmouseup={handleGlobalMouseUp}
        onmouseleave={handleGlobalMouseUp}
      >
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <div class="lightbox-image-wrapper" onclick={closeLightbox}>
          <div
            class="lightbox-image-container"
            onclick={(e) => e.stopPropagation()}
            onmousedown={handleMouseDown}
            style={`cursor: ${zoomLevel > 1 ? (isDragging ? 'grabbing' : 'grab') : 'default'}`}
          >
            <img
              bind:this={imageElement}
              src={selectedImage}
              alt=""
              draggable={false}
              style={`transform: translate(${panOffset.x}px, ${panOffset.y}px) scale(${zoomLevel}); transition: ${isDragging ? 'none' : 'transform 0.2s ease-out'}`}
              onwheel={(e) => {
                const newZoom = e.deltaY < 0 ? Math.min(5, zoomLevel + 0.1) : Math.max(1, zoomLevel - 0.1);
                zoomLevel = newZoom;
                if (newZoom === 1) panOffset = { x: 0, y: 0 };
              }}
            />
          </div>
        </div>

        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <div
          class="lightbox-controls-floating"
          onmousedown={handlePanelMouseDown}
          style={`position: fixed; left: ${panelPos.x}%; top: ${panelPos.y}%; transform: translate(-50%, -50%); cursor: ${isDraggingPanel ? 'grabbing' : 'grab'}`}
          onclick={(e) => e.stopPropagation()}
        >
          <button
            class="ctrl-btn-round"
            onclick={(e) => {
              e.stopPropagation();
              handleCopyImage();
            }}
            title={$t('detail.copyLog')}
          >
            <Copy size={20} />
          </button>
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div class="zoom-capsule" onmousedown={(e) => e.stopPropagation()}>
            <button
              class="ctrl-btn-round mini"
              onclick={() => {
                zoomLevel = Math.max(1, zoomLevel - 0.2);
                if (zoomLevel <= 1.2) panOffset = { x: 0, y: 0 };
              }}
            >
              <ZoomOut size={18} />
            </button>
            <span class="zoom-val">{Math.round(zoomLevel * 100)}%</span>
            <button
              class="ctrl-btn-round mini"
              onclick={() => {
                zoomLevel = Math.min(5, zoomLevel + 0.2);
              }}
            >
              <ZoomIn size={18} />
            </button>
          </div>
        </div>
      </div>
    {/if}
  </div>
{/if}

{#if showDuplicateModal && instance}
  <CreateInstanceModal
    duplicateSource={instance}
    onClose={() => (showDuplicateModal = false)}
    onCreated={(newInstance) => {
      showDuplicateModal = false;
      showNotification($t('notify.instanceDuplicated'), 'success');
      if (newInstance && newInstance.id) {
        goto(`/instance/${newInstance.id}`);
      }
    }}
  />
{/if}
