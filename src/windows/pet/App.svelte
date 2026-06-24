<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/core';
  import { currentSvg, currentState, dndEnabled, currentLang } from '../../lib/stores';
  import { get } from 'svelte/store';

  import _idleFollowRaw from '../../../assets/svg/clyde-idle-follow.svg?raw';
  import doroIdle from '../../../doroPic/processed/doro_idle_base_no_pupils.png';
  import doroIdleLeftPupil from '../../../doroPic/processed/doro_idle_left_pupil.png';
  import doroIdleRightPupil from '../../../doroPic/processed/doro_idle_right_pupil.png';
  import doroMiniIdle from '../../../doroPic/processed/doro1.png';
  import doroTyping from '../../../doroPic/processed/doro2.png';
  import doroPermission from '../../../doroPic/processed/doro3.png';
  import doroHappy from '../../../doroPic/processed/doro5.png';
  import doroSleepy from '../../../doroPic/processed/doro6.png';
  import doroNotification from '../../../doroPic/processed/doro7.png';
  import doroBuilding from '../../../doroPic/processed/doro8.png';
  import doroError from '../../../doroPic/processed/doro11.png';
  import doroSleeping from '../../../doroPic/processed/doro12.png';
  import doroThinking from '../../../doroPic/processed/doro13.png';
  import doroSpin from '../../../doroPic/转圈.webp';

  const rawModules = import.meta.glob('../../../assets/svg/*.svg', {
    query: '?raw',
    import: 'default',
    eager: true,
  }) as Record<string, string>;
  const timeImageModules = import.meta.glob('../../../doroPic/time/*.{jpg,jpeg,png,webp,gif}', {
    import: 'default',
    eager: true,
  }) as Record<string, string>;
  const timeImages = Object.values(timeImageModules);
  const TIME_IMAGE_VISIBLE_MS = 3_000;
  const SPIN_REACTION_VISIBLE_MS = 3_000;

  function stripSvgSize(raw: string): string {
    return raw.replace(/\s+width="[^"]*"/, '').replace(/\s+height="[^"]*"/, '');
  }

  // Pre-process all SVGs at init time — avoids regex on every state change
  const svgCache: Record<string, string> = {};
  for (const [key, raw] of Object.entries(rawModules)) {
    const filename = key.split('/').pop() ?? key;
    svgCache[filename] = stripSvgSize(raw);
  }
  if (_idleFollowRaw && !svgCache['clyde-idle-follow.svg']) {
    svgCache['clyde-idle-follow.svg'] = stripSvgSize(_idleFollowRaw);
  }

  function getSvg(filename: string): string {
    return svgCache[filename] ?? svgCache['clyde-idle-follow.svg'] ?? '';
  }

  type DoroAsset = {
    src: string;
    mood: string;
    leftPupil?: string;
    rightPupil?: string;
  };
  type PetConfig = {
    opacity: number;
    time_image_interval_secs: number;
    custom_pet_image_data_url?: string | null;
  };

  const doroAssets: Record<string, DoroAsset> = {
    'clyde-idle-follow.svg': {
      src: doroIdle,
      mood: 'idle',
      leftPupil: doroIdleLeftPupil,
      rightPupil: doroIdleRightPupil,
    },
    'clyde-idle-living.svg': {
      src: doroIdle,
      mood: 'idle',
      leftPupil: doroIdleLeftPupil,
      rightPupil: doroIdleRightPupil,
    },
    'clyde-idle-look.svg': {
      src: doroIdle,
      mood: 'idle',
      leftPupil: doroIdleLeftPupil,
      rightPupil: doroIdleRightPupil,
    },
    'clyde-notification.svg': { src: doroNotification, mood: 'notification' },
    'clyde-error.svg': { src: doroError, mood: 'error' },
    'clyde-happy.svg': { src: doroHappy, mood: 'happy' },
    'clyde-working-thinking.svg': { src: doroThinking, mood: 'thinking' },
    'clyde-working-typing.svg': { src: doroTyping, mood: 'typing' },
    'clyde-working-building.svg': { src: doroBuilding, mood: 'building' },
    'clyde-working-juggling.svg': { src: doroPermission, mood: 'juggling' },
    'clyde-working-conducting.svg': { src: doroPermission, mood: 'conducting' },
    'clyde-working-carrying.svg': { src: doroTyping, mood: 'carrying' },
    'clyde-working-sweeping.svg': { src: doroSleepy, mood: 'sweeping' },
    'clyde-sleeping.svg': { src: doroSleeping, mood: 'sleeping' },
    'clyde-idle-yawn.svg': { src: doroSleepy, mood: 'yawning' },
    'clyde-idle-doze.svg': { src: doroSleepy, mood: 'dozing' },
    'clyde-collapse-sleep.svg': { src: doroSleeping, mood: 'collapsing' },
    'clyde-wake.svg': { src: doroHappy, mood: 'waking' },
    'clyde-mini-idle.svg': { src: doroMiniIdle, mood: 'mini' },
    'clyde-mini-alert.svg': { src: doroNotification, mood: 'mini-alert' },
    'clyde-mini-happy.svg': { src: doroHappy, mood: 'mini-happy' },
    'clyde-mini-peek.svg': { src: doroPermission, mood: 'mini-peek' },
    'clyde-mini-enter.svg': { src: doroMiniIdle, mood: 'mini' },
    'clyde-mini-sleep.svg': { src: doroSleeping, mood: 'mini-sleep' },
    'clyde-react-double.svg': { src: doroHappy, mood: 'happy' },
    'clyde-react-drag.svg': { src: doroIdle, mood: 'drag' },
    'clyde-react-left.svg': { src: doroHappy, mood: 'happy' },
    'clyde-react-right.svg': { src: doroHappy, mood: 'happy' },
  };

  function getDoroAsset(filename: string): DoroAsset {
    return doroAssets[filename] ?? doroAssets['clyde-idle-follow.svg'];
  }

  function getCurrentDoroAsset(filename: string): DoroAsset {
    if (customPetImageDataUrl) {
      return { src: customPetImageDataUrl, mood: 'custom' };
    }
    return getDoroAsset(filename);
  }

  function resetPupils() {
    eyeOffsetX = 0;
    eyeOffsetY = 0;
  }

  function setPetVisual(filename: string, updateStore = true) {
    if (updateStore) currentSvg.set(filename);
    svgContent = getSvg(filename);
    doroAsset = getCurrentDoroAsset(filename);
    resetPupils();
  }

  let svgContent = $state(getSvg(get(currentSvg)));
  let customPetImageDataUrl = $state('');
  let doroAsset = $state(getCurrentDoroAsset(get(currentSvg)));
  let flipped = $state(false);
  let unlisten: UnlistenFn[] = [];
  let isReacting = false;
  let reactTimer: ReturnType<typeof setTimeout> | null = null;
  let timeImageTimer: ReturnType<typeof setTimeout> | null = null;
  let timeImageHideTimer: ReturnType<typeof setTimeout> | null = null;
  let isTimeImageShowing = false;
  let timeImageIntervalSecs = 60;
  let snapPreview = $state(false);
  let opacity = $state(1);
  let eyeOffsetX = $state(0);
  let eyeOffsetY = $state(0);

  function clamp(value: number, min: number, max: number) {
    return Math.min(max, Math.max(min, value));
  }

  function computePupilOffset(dx: number, dy: number) {
    const container = document.getElementById('pet-container');
    const size = container
      ? Math.min(container.clientWidth, container.clientHeight)
      : 200;
    const maxX = size * 0.010;
    const maxY = size * 0.004;

    return {
      x: clamp((dx / 3) * maxX, -maxX, maxX),
      y: clamp((dy / 3) * maxY, -maxY, maxY),
    };
  }

  function movePupils(dx: number, dy: number) {
    const offset = computePupilOffset(dx, dy);
    eyeOffsetX = offset.x;
    eyeOffsetY = offset.y;

    // Eyes follow cursor (larger movement)
    const eyes = document.getElementById('eyes-js');
    if (eyes) eyes.style.transform = `translate(${dx * 0.6}px, ${dy * 0.4}px)`;

    // Body tilts slightly toward cursor
    const body = document.getElementById('body-js');
    if (body) body.style.transform = `translate(${dx * 0.15}px, 0)`;

    // Shadow stretches opposite to lean
    const shadow = document.getElementById('shadow-js');
    if (shadow) shadow.style.transform = `scaleX(${1 + Math.abs(dx) * 0.03})`;
  }

  function playReaction(svgFile: string, durationMs: number) {
    if (reactTimer) clearTimeout(reactTimer);
    isReacting = true;
    setPetVisual(svgFile);
    reactTimer = setTimeout(() => { isReacting = false; }, durationMs);
  }

  function playDoroImageReaction(src: string, mood: string, durationMs: number) {
    if (reactTimer) clearTimeout(reactTimer);
    if (timeImageHideTimer) clearTimeout(timeImageHideTimer);
    timeImageHideTimer = null;
    isTimeImageShowing = false;
    isReacting = true;
    doroAsset = { src, mood };
    resetPupils();
    reactTimer = setTimeout(() => {
      isReacting = false;
      setPetVisual(get(currentSvg), false);
    }, durationMs);
  }

  function scheduleTimeImage() {
    if (timeImageTimer) clearTimeout(timeImageTimer);
    timeImageTimer = null;
    if (timeImages.length === 0 || timeImageIntervalSecs <= 0) return;
    const delayMs = Math.random() * timeImageIntervalSecs * 1000;
    timeImageTimer = setTimeout(showTimeImage, delayMs);
  }

  function showTimeImage() {
    timeImageTimer = null;
    if (timeImages.length === 0 || timeImageIntervalSecs <= 0) return;
    const src = timeImages[Math.floor(Math.random() * timeImages.length)];
    isTimeImageShowing = true;
    doroAsset = { src, mood: 'time' };
    resetPupils();

    timeImageHideTimer = setTimeout(() => {
      isTimeImageShowing = false;
      setPetVisual(get(currentSvg), false);
      scheduleTimeImage();
    }, TIME_IMAGE_VISIBLE_MS);
  }

  function applyTimeImageInterval(secs: number) {
    timeImageIntervalSecs = secs;
    if (timeImageHideTimer) clearTimeout(timeImageHideTimer);
    timeImageHideTimer = null;
    if (isTimeImageShowing) {
      isTimeImageShowing = false;
      setPetVisual(get(currentSvg), false);
    }
    scheduleTimeImage();
  }

  onMount(() => {
    const setup = async () => {
      const config = await invoke<PetConfig>('get_pet_config');
      opacity = config.opacity ?? 1;
      customPetImageDataUrl = config.custom_pet_image_data_url ?? '';
      setPetVisual(get(currentSvg), false);
      applyTimeImageInterval(config.time_image_interval_secs ?? 60);

      unlisten.push(await listen<{ state: string; svg: string; flip?: boolean }>('state-change', ({ payload }) => {
        if (isReacting) return;
        currentState.set(payload.state as any);
        if (isTimeImageShowing) {
          currentSvg.set(payload.svg);
        } else {
          setPetVisual(payload.svg);
        }
        flipped = payload.flip ?? false;
      }));

      unlisten.push(await listen<{ dx: number; dy: number }>('eye-move', ({ payload }) => {
        movePupils(payload.dx, payload.dy);
      }));

      unlisten.push(await listen<{ enabled: boolean }>('dnd-change', ({ payload }) => {
        dndEnabled.set(payload.enabled);
      }));

      unlisten.push(await listen<{ svg: string; duration_ms: number }>('play-click-reaction', ({ payload }) => {
        playReaction(payload.svg, payload.duration_ms);
      }));

      unlisten.push(await listen('play-spin-reaction', () => {
        playDoroImageReaction(doroSpin, 'spin', SPIN_REACTION_VISIBLE_MS);
      }));

      unlisten.push(await listen<PetConfig>('pet-config-changed', ({ payload }) => {
        opacity = payload.opacity ?? 1;
        customPetImageDataUrl = payload.custom_pet_image_data_url ?? '';
        if (!isTimeImageShowing && !isReacting) {
          setPetVisual(get(currentSvg), false);
        }
        applyTimeImageInterval(payload.time_image_interval_secs ?? 60);
      }));

      unlisten.push(await listen('start-drag-reaction', () => {
        setPetVisual('clyde-react-drag.svg');
      }));

      unlisten.push(await listen<{ active: boolean }>('snap-preview', ({ payload }) => {
        snapPreview = payload.active;
      }));

      unlisten.push(await listen('trigger-yawn', () => { invoke('trigger_sleep_sequence'); }));
      unlisten.push(await listen('trigger-wake', () => { invoke('trigger_wake'); }));
      unlisten.push(await listen('mini-peek-in', () => { invoke('mini_peek_in'); }));
      unlisten.push(await listen('mini-peek-out', () => { invoke('mini_peek_out'); }));
      unlisten.push(await listen<string>('set-size', ({ payload }) => { invoke('set_window_size', { size: payload }); }));
      unlisten.push(await listen<string>('set-lang', ({ payload }) => {
        currentLang.set(payload);
        invoke('set_lang', { lang: payload });
      }));
    };
    setup();
  });

  onDestroy(() => {
    unlisten.forEach(u => u());
    if (reactTimer) clearTimeout(reactTimer);
    if (timeImageTimer) clearTimeout(timeImageTimer);
    if (timeImageHideTimer) clearTimeout(timeImageHideTimer);
  });
</script>

<div id="pet-container" class:snap-preview={snapPreview} style:opacity={opacity}>
  <div class="svg-wrapper" style:transform={flipped ? 'scaleX(-1)' : ''}>
    {#if doroAsset?.leftPupil && doroAsset?.rightPupil}
      <div class={`doro-layered doro-${doroAsset.mood}`}>
        <img
          class="doro-layer"
          src={doroAsset.src}
          alt=""
          draggable="false"
        />
        <img
          class="doro-layer doro-pupil"
          src={doroAsset.leftPupil}
          alt=""
          draggable="false"
          style:transform={`translate(${eyeOffsetX}px, ${eyeOffsetY}px)`}
        />
        <img
          class="doro-layer doro-pupil"
          src={doroAsset.rightPupil}
          alt=""
          draggable="false"
          style:transform={`translate(${eyeOffsetX}px, ${eyeOffsetY}px)`}
        />
      </div>
    {:else if doroAsset}
      <img
        class={`doro-pet doro-${doroAsset.mood}`}
        src={doroAsset.src}
        alt=""
        draggable="false"
      />
    {:else}
      {@html svgContent}
    {/if}
  </div>
</div>

<style>
  #pet-container {
    width: 100%;
    height: 100%;
    position: relative;
    background: transparent;
    overflow: hidden;
  }
  .svg-wrapper {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }
  .svg-wrapper :global(svg) {
    display: block;
    width: 100%;
    height: 100%;
  }
  .doro-pet {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: contain;
    pointer-events: none;
    user-select: none;
    transform-origin: 50% 78%;
  }
  .doro-time {
    position: absolute;
    left: 25%;
    bottom: 25%;
    width: 50%;
    height: 50%;
  }
  .doro-layered {
    position: absolute;
    inset: 0;
    transform-origin: 50% 78%;
  }
  .doro-layer {
    position: absolute;
    inset: 0;
    display: block;
    width: 100%;
    height: 100%;
    object-fit: contain;
    pointer-events: none;
    user-select: none;
  }
  .doro-pupil {
    transition: transform 80ms ease-out;
  }
  .doro-idle,
  .doro-mini {
    animation: doro-breathe 3.4s infinite ease-in-out;
  }
  .doro-thinking,
  .doro-typing,
  .doro-building {
    animation: doro-work 0.8s infinite ease-in-out;
  }
  .doro-happy,
  .doro-notification,
  .doro-mini-alert,
  .doro-mini-happy,
  .doro-waking {
    animation: doro-hop 0.58s infinite ease-in-out;
  }
  .doro-error {
    animation: doro-shake 0.18s infinite linear;
  }
  .doro-sleeping,
  .doro-yawning,
  .doro-dozing,
  .doro-collapsing,
  .doro-mini-sleep {
    animation: doro-sleep 3.2s infinite ease-in-out;
    opacity: 0.86;
  }
  .doro-juggling,
  .doro-conducting,
  .doro-carrying,
  .doro-sweeping,
  .doro-mini-peek,
  .doro-drag {
    animation: doro-sway 1.2s infinite ease-in-out;
  }
  @keyframes doro-breathe {
    0%, 100% { transform: translateY(0) scale(1); }
    50% { transform: translateY(2px) scale(1.02, 0.98); }
  }
  @keyframes doro-work {
    0%, 100% { transform: translateY(0) rotate(-1deg); }
    50% { transform: translateY(-5px) rotate(1deg); }
  }
  @keyframes doro-hop {
    0%, 100% { transform: translateY(0) scale(1); }
    45% { transform: translateY(-10px) scale(1.04, 0.96); }
  }
  @keyframes doro-shake {
    0%, 100% { transform: translateX(0); }
    25% { transform: translateX(-4px) rotate(-2deg); }
    75% { transform: translateX(4px) rotate(2deg); }
  }
  @keyframes doro-sleep {
    0%, 100% { transform: translateY(5px) rotate(-4deg) scale(0.94); }
    50% { transform: translateY(8px) rotate(-5deg) scale(0.92); }
  }
  @keyframes doro-sway {
    0%, 100% { transform: translateX(0) rotate(-2deg); }
    50% { transform: translateX(3px) rotate(2deg); }
  }
  /* Snap preview: scale down + slight transparency when near screen edge during drag */
  #pet-container.snap-preview {
    transform: scale(0.7);
    opacity: 0.6;
    transition: transform 150ms ease-out, opacity 150ms ease-out;
  }
  #pet-container:not(.snap-preview) {
    transition: transform 150ms ease-out, opacity 150ms ease-out;
  }
  /* Smooth eye/body tracking — interpolate between 50ms tick updates */
  .svg-wrapper :global(#eyes-js),
  .svg-wrapper :global(#body-js),
  .svg-wrapper :global(#shadow-js) {
    transition: transform 80ms ease-out;
  }
</style>
