<script lang="ts">
  import { onMount } from 'svelte';
  import { UserPlus, Trash2, CheckCircle2, User, AlertTriangle, Monitor, Globe, ChevronRight, X, Users, Sparkles } from 'lucide-svelte';
  import { invoke } from '@tauri-apps/api/core';
  import type { Profile } from '../../types/profile';
  import { showNotification } from '../../stores/notification';
  import { portal } from '../../utils/portal';
  import { t } from '../../stores/i18n';
  import './ProfileManager.css';

  let profiles = $state<Profile[]>([]);
  let activeProfileId = $state<string | null>(null);
  let newProfileName = $state('');

  let showAddModal = $state(false);
  let addStep = $state<'select' | 'offline' | 'microsoft'>('select');
  let profileToDelete = $state<Profile | null>(null);
  let isClosing = $state(false);
  let loading = $state(true);
  let isLoggingIn = $state(false);

  onMount(() => {
    loadProfiles();
    const activeId = localStorage.getItem('activeProfileId');
    if (activeId) activeProfileId = activeId;
  });

  $effect(() => {
    if (showAddModal || profileToDelete) {
      document.body.classList.add('modal-open');
    } else {
      document.body.classList.remove('modal-open');
    }
    return () => {
      document.body.classList.remove('modal-open');
    };
  });

  async function loadProfiles() {
    try {
      loading = true;
      const data: Profile[] = await invoke('get_profiles');
      profiles = data;
    } catch (err) {
      console.error('Failed to load profiles:', err);
    } finally {
      loading = false;
    }
  }

  function closeModal() {
    if (isLoggingIn) return;
    isClosing = true;
    setTimeout(() => {
      showAddModal = false;
      profileToDelete = null;
      isClosing = false;
      addStep = 'select';
      newProfileName = '';
    }, 300);
  }

  async function handleAddOffline(e: SubmitEvent) {
    e.preventDefault();
    if (!newProfileName.trim()) return;

    try {
      const newProfile: Profile = await invoke('create_profile', { name: newProfileName });
      profiles = [...profiles, newProfile];
      if (!activeProfileId) handleSelectProfile(newProfile.id);
      showNotification($t('profiles.created'), 'success');
      closeModal();
    } catch (err) {
      showNotification(`Error: ${err}`, 'error');
    }
  }

  async function handleMicrosoftLogin() {
    try {
      setIsLoggingIn(true);
      const newProfile: Profile = await invoke('login_microsoft');
      profiles = [...profiles, newProfile];
      handleSelectProfile(newProfile.id);
      showNotification($t('profiles.msSuccess'), 'success');
      setIsLoggingIn(false);
      closeModal();
    } catch (err) {
      console.error('Microsoft login failed:', err);
      setIsLoggingIn(false);
      showNotification($t('profiles.msFailed'), 'error');
    }
  }

  function setIsLoggingIn(val: boolean) {
    isLoggingIn = val;
  }

  async function confirmDelete() {
    if (!profileToDelete) return;
    try {
      await invoke('delete_profile', { id: profileToDelete.id });
      const updated = profiles.filter((p) => p.id !== profileToDelete?.id);
      profiles = updated;
      if (activeProfileId === profileToDelete.id) {
        const nextActive = updated.length > 0 ? updated[0].id : null;
        activeProfileId = nextActive;
        if (nextActive) localStorage.setItem('activeProfileId', nextActive);
        else localStorage.removeItem('activeProfileId');
      }
      showNotification($t('profiles.deleted'), 'success');
      closeModal();
    } catch (err) {
      showNotification('Error deleting profile', 'error');
    }
  }

  function handleSelectProfile(id: string) {
    activeProfileId = id;
    localStorage.setItem('activeProfileId', id);
  }
</script>

<div class="profile-manager page-container animate-fade-in">
  <div class="profile-header">
    <div class="title-section">
      <div class="title-row">
        <h1>{$t('profiles.title')}</h1>
        <span class="profile-count-pill">{profiles.length}</span>
      </div>
      <p>{$t('profiles.subtitle')}</p>
    </div>
    <button class="btn btn-primary add-profile-btn" onclick={() => (showAddModal = true)}>
      <UserPlus size={18} />
      <span>{$t('profiles.addAccount')}</span>
    </button>
  </div>

  <div class="profiles-container">
    {#if loading}
      <div class="loading-profiles">{$t('profiles.loading')}</div>
    {:else if profiles.length > 0}
      <div class="profiles-grid">
        {#each profiles as profile (profile.id)}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            class={`profile-card ${activeProfileId === profile.id ? 'active' : ''}`}
            onclick={() => handleSelectProfile(profile.id)}
          >
            <div class="profile-avatar-wrapper">
              <div class="avatar-head-box">
                <img
                  src={`https://mc-heads.net/avatar/${encodeURIComponent(profile.name)}/64`}
                  alt={profile.name}
                  class="player-head-img"
                  loading="lazy"
                />
              </div>
              {#if activeProfileId === profile.id}
                <div class="active-indicator-badge" title={$t('profiles.activeAccount')}>
                  <CheckCircle2 size={13} />
                </div>
              {/if}
            </div>

            <div class="profile-info">
              <div class="profile-name-row">
                <h3 title={profile.name}>{profile.name}</h3>
                {#if activeProfileId === profile.id}
                  <span class="status-pill-active">{$t('profiles.active')}</span>
                {/if}
              </div>

              <div class="profile-meta">
                {#if profile.profile_type === 'microsoft'}
                  <span class="account-type-badge ms">
                    <svg class="ms-mini-logo" viewBox="0 0 21 21" width="11" height="11">
                      <rect x="1" y="1" width="9" height="9" fill="#f25022"/>
                      <rect x="11" y="1" width="9" height="9" fill="#7fba00"/>
                      <rect x="1" y="11" width="9" height="9" fill="#00a4ef"/>
                      <rect x="11" y="11" width="9" height="9" fill="#ffb900"/>
                    </svg>
                    Microsoft
                  </span>
                {:else}
                  <span class="account-type-badge offline">
                    <Monitor size={11} />
                    Offline
                  </span>
                {/if}
              </div>
            </div>

            <div class="profile-card-actions">
              {#if activeProfileId !== profile.id}
                <button
                  type="button"
                  class="select-profile-btn"
                  onclick={(e) => {
                    e.stopPropagation();
                    handleSelectProfile(profile.id);
                  }}
                >
                  {$t('profiles.select')}
                </button>
              {/if}
              <button
                class="delete-btn"
                onclick={(e) => {
                  e.stopPropagation();
                  profileToDelete = profile;
                }}
                title={$t('profiles.delete')}
                aria-label={$t('profiles.delete')}
              >
                <Trash2 size={16} />
              </button>
            </div>
          </div>
        {/each}
      </div>
    {:else}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="empty-profiles-wrapper">
        <div class="empty-profiles-content" onclick={() => (showAddModal = true)}>
          <div class="empty-icon-wrapper">
            <UserPlus size={44} />
          </div>
          <h3>{$t('profiles.emptyTitle')}</h3>
          <p>{$t('profiles.emptySub')}</p>
          <button class="btn btn-primary">{$t('profiles.createFirst')}</button>
        </div>
      </div>
    {/if}
  </div>

  {#if showAddModal}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class={`modal-overlay ${isClosing ? 'closing' : ''}`} use:portal onclick={closeModal}>
      <div class="modal-content" onclick={(e) => e.stopPropagation()}>
        <button class="modal-close-btn" onclick={closeModal} aria-label="Close">
          <X size={18} />
        </button>

        {#if addStep === 'select'}
          <div class="modal-step-content">
            <div class="modal-header centered">
              <div class="header-icon-circle">
                <UserPlus size={26} />
              </div>
              <h2>{$t('profiles.howToLogin')}</h2>
              <p class="modal-subtitle">{$t('profiles.selectAccountType')}</p>
            </div>

            <div class="auth-options">
              <div class="auth-option" onclick={() => (addStep = 'microsoft')}>
                <div class="option-icon ms">
                  <svg viewBox="0 0 21 21" width="24" height="24">
                    <rect x="1" y="1" width="9" height="9" fill="#f25022"/>
                    <rect x="11" y="1" width="9" height="9" fill="#7fba00"/>
                    <rect x="1" y="11" width="9" height="9" fill="#00a4ef"/>
                    <rect x="11" y="11" width="9" height="9" fill="#ffb900"/>
                  </svg>
                </div>
                <div class="option-text">
                  <h3>{$t('profiles.microsoft')}</h3>
                  <p>{$t('profiles.msDesc')}</p>
                </div>
                <ChevronRight size={18} class="arrow-icon" />
              </div>

              <div class="auth-option" onclick={() => (addStep = 'offline')}>
                <div class="option-icon offline">
                  <Monitor size={22} />
                </div>
                <div class="option-text">
                  <h3>{$t('profiles.offline')}</h3>
                  <p>{$t('profiles.offlineDesc')}</p>
                </div>
                <ChevronRight size={18} class="arrow-icon" />
              </div>
            </div>

            <div class="modal-actions">
              <button type="button" class="btn btn-secondary full-width" onclick={closeModal}>{$t('settings.cancel')}</button>
            </div>
          </div>
        {:else if addStep === 'offline'}
          <div class="modal-step-content">
            <div class="modal-header centered">
              <div class="header-icon-circle">
                <Monitor size={26} />
              </div>
              <h2>{$t('profiles.offlineTitle')}</h2>
              <p class="modal-subtitle">{$t('profiles.enterUsername')}</p>
            </div>

            <form onsubmit={handleAddOffline} class="full-width">
              <div class="offline-preview-card">
                <div class="preview-avatar-box">
                  <img
                    src={`https://mc-heads.net/avatar/${encodeURIComponent(newProfileName.trim() || 'Steve')}/80`}
                    alt="Skin preview"
                    class="preview-player-head"
                  />
                </div>
                <div class="preview-text">
                  <span class="preview-label">{$t('profiles.skinPreview')}</span>
                  <span class="preview-username">{newProfileName.trim() || 'Steve'}</span>
                </div>
              </div>

              <div class="input-group">
                <label for="offline-username-input">{$t('profiles.usernameLabel')}</label>
                <input
                  id="offline-username-input"
                  type="text"
                  placeholder={$t('profiles.usernamePlaceholder')}
                  bind:value={newProfileName}
                />
              </div>

              <div class="modal-actions spaced">
                <button type="button" class="btn btn-secondary" onclick={() => (addStep = 'select')}>{$t('profiles.back')}</button>
                <button type="submit" class="btn btn-primary flex-1" disabled={!newProfileName.trim()}>{$t('profiles.create')}</button>
              </div>
            </form>
          </div>
        {:else if addStep === 'microsoft'}
          <div class="modal-step-content">
            <div class="modal-header centered">
              <div class="ms-logo-large">
                <svg viewBox="0 0 21 21" width="56" height="56">
                  <rect x="1" y="1" width="9" height="9" fill="#f25022"/>
                  <rect x="11" y="1" width="9" height="9" fill="#7fba00"/>
                  <rect x="1" y="11" width="9" height="9" fill="#00a4ef"/>
                  <rect x="11" y="11" width="9" height="9" fill="#ffb900"/>
                </svg>
              </div>
              <h2>{$t('profiles.msTitle')}</h2>
              <p class="modal-subtitle">{$t('profiles.msSub')}</p>
            </div>

            <p class="modal-desc centered">
              {isLoggingIn
                ? $t('profiles.msLoggingIn')
                : $t('profiles.msPrompt')}
            </p>

            {#if isLoggingIn}
              <div class="login-spinner">
                <div class="spinner-ring"></div>
              </div>
            {/if}

            <div class="modal-actions spaced mt-40">
              <button type="button" class="btn btn-secondary" disabled={isLoggingIn} onclick={() => (addStep = 'select')}>{$t('profiles.back')}</button>
              <button
                type="button"
                class="btn btn-primary flex-1"
                disabled={isLoggingIn}
                onclick={handleMicrosoftLogin}
              >
                {isLoggingIn ? $t('profiles.inProgress') : $t('profiles.login')}
              </button>
            </div>
          </div>
        {/if}
      </div>
    </div>
  {/if}

  {#if profileToDelete}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class={`modal-overlay ${isClosing ? 'closing' : ''}`} use:portal onclick={closeModal}>
      <div class="modal-content" onclick={(e) => e.stopPropagation()}>
        <button class="modal-close-btn" onclick={closeModal} aria-label="Close">
          <X size={18} />
        </button>

        <div class="modal-header centered">
          <div class="alert-icon-wrapper">
            <AlertTriangle size={30} />
          </div>
          <h2>{$t('profiles.deleteTitle')}</h2>
        </div>

        <p class="modal-desc centered">
          {$t('profiles.deleteConfirm', { name: profileToDelete.name })}
        </p>

        <div class="modal-actions spaced">
          <button type="button" class="btn btn-secondary flex-1" onclick={closeModal}>{$t('settings.cancel')}</button>
          <button type="button" class="btn btn-danger flex-1" onclick={confirmDelete}>{$t('profiles.deleteBtn')}</button>
        </div>
      </div>
    </div>
  {/if}
</div>
