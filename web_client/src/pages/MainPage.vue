<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { useRouter } from 'vue-router';

import { useConnection } from 'src/stores/connection';
import { useQuasar } from 'quasar';

const $q = useQuasar();
const router = useRouter();
const connection = useConnection();

const isLocked = ref(false);
let yaw = 0;
let pitch = 0;
// A literal-origin head pose sits at floor level — coincident with (or
// inside) any scene geometry an app places near the origin, so apps look
// broken until the user manually moves. Default to a plausible standing eye
// height instead, matching a real headset. Kept in sync with the runtime's
// own default in runtime/src/input/device_state.rs, since this default
// overwrites that one the moment the client connects and starts streaming
// input every frame.
const DEFAULT_STANDING_HEIGHT = 1.6;
const position = { x: 0, y: DEFAULT_STANDING_HEIGHT, z: 0 };
const SENSITIVITY = 0.002;
const MOVE_SPEED = 1.5; // meters per second

const keysDown = new Set<string>();
let animationFrame = 0;
let lastFrameTime = 0;

// q-page only gets a min-height from Quasar (a floor, not a real height), so a
// flex-grow child has nothing definite to fill and content just grows the page
// taller than the viewport. Measure the actual space below the header instead.
const availableHeight = ref(0);
function updateAvailableHeight() {
  const header = document.querySelector('.q-header');
  availableHeight.value = window.innerHeight - (header?.clientHeight ?? 0);
}

// Bigger previews are harder to cross-eye-fuse into 3D, so let it be tuned down.
const previewScale = ref(Number(localStorage.getItem('previewScale')) || 0.6);
watch(previewScale, (value) => localStorage.setItem('previewScale', String(value)));

// Matches the resting controller pose in runtime/src/input/device_state.rs.
// Real 6DoF (gyro + arm model) comes later; for keyboard testing IJKL/arrows
// orient the hands (for aiming) and, held with Shift, move them instead.
const leftHandPos = { x: -0.3, y: DEFAULT_STANDING_HEIGHT - 0.3, z: -0.5 };
const rightHandPos = { x: 0.3, y: DEFAULT_STANDING_HEIGHT - 0.3, z: -0.5 };
let leftHandYaw = 0;
let leftHandPitch = 0;
let rightHandYaw = 0;
let rightHandPitch = 0;
const HAND_MOVE_SPEED = 0.5; // meters per second
const HAND_ANGULAR_SPEED = 2.0; // radians per second

function stickAxis(negKey: string, posKey: string): number {
  let v = 0;
  if (keysDown.has(posKey)) v += 1;
  if (keysDown.has(negKey)) v -= 1;
  return v;
}

function isShiftHeld(): boolean {
  return keysDown.has('ShiftLeft') || keysDown.has('ShiftRight');
}

// q = q_yaw (Y-axis) * q_pitch (X-axis)
function quatFromYawPitch(yaw: number, pitch: number) {
  const sy = Math.sin(yaw * 0.5);
  const cy = Math.cos(yaw * 0.5);
  const sp = Math.sin(pitch * 0.5);
  const cp = Math.cos(pitch * 0.5);
  return { x: cy * sp, y: sy * cp, z: -sy * sp, w: cy * cp };
}

watch(
  () => connection.isConnected,
  async (connected) => {
    if (!connected) {
      if (document.pointerLockElement) document.exitPointerLock();
      $q.notify({ type: 'warning', message: 'Server disconnected' });
      await router.push({ path: '/connect', replace: true });
    }
  },
);

onMounted(async () => {
  if (!connection.isConnected) {
    await router.push({ path: '/connect', replace: true });
    return;
  }
  document.addEventListener('pointerlockchange', onPointerLockChange);
  document.addEventListener('mousemove', onMouseMove);
  document.addEventListener('keydown', onKeyDown);
  document.addEventListener('keyup', onKeyUp);
  updateAvailableHeight();
  window.addEventListener('resize', updateAvailableHeight);
  lastFrameTime = performance.now();
  animationFrame = requestAnimationFrame(onFrame);
});

onUnmounted(() => {
  document.removeEventListener('pointerlockchange', onPointerLockChange);
  document.removeEventListener('mousemove', onMouseMove);
  document.removeEventListener('keydown', onKeyDown);
  document.removeEventListener('keyup', onKeyUp);
  window.removeEventListener('resize', updateAvailableHeight);
  if (document.pointerLockElement) document.exitPointerLock();
  cancelAnimationFrame(animationFrame);
});

function onPointerLockChange() {
  isLocked.value = !!document.pointerLockElement;
}

function onMouseMove(e: MouseEvent) {
  if (!document.pointerLockElement) return;
  yaw -= e.movementX * SENSITIVITY;
  pitch -= e.movementY * SENSITIVITY;
  pitch = Math.max(-Math.PI / 2 + 0.01, Math.min(Math.PI / 2 - 0.01, pitch));
}

function onKeyDown(e: KeyboardEvent) {
  keysDown.add(e.code);
}

function onKeyUp(e: KeyboardEvent) {
  keysDown.delete(e.code);
}

function onFrame(time: number) {
  const dt = (time - lastFrameTime) / 1000;
  lastFrameTime = time;

  const shiftHeld = isShiftHeld();

  // Shift+WASD moves the head; plain WASD instead drives the movement
  // joystick (left thumbstick) below. Space/Ctrl (unconditional) move it
  // vertically, since neither hand's controls have a vertical axis to
  // contend with.
  let moveX = 0;
  let moveZ = 0;
  let moveY = 0;
  if (shiftHeld) {
    // Forward/right vectors on the horizontal plane, from yaw only (ignore pitch).
    const forward = { x: -Math.sin(yaw), z: -Math.cos(yaw) };
    const right = { x: Math.cos(yaw), z: -Math.sin(yaw) };
    if (keysDown.has('KeyW')) {
      moveX += forward.x;
      moveZ += forward.z;
    }
    if (keysDown.has('KeyS')) {
      moveX -= forward.x;
      moveZ -= forward.z;
    }
    if (keysDown.has('KeyD')) {
      moveX += right.x;
      moveZ += right.z;
    }
    if (keysDown.has('KeyA')) {
      moveX -= right.x;
      moveZ -= right.z;
    }
  }
  if (keysDown.has('Space')) moveY += 1;
  if (keysDown.has('ControlLeft')) moveY -= 1;

  const moveLen = Math.hypot(moveX, moveZ);
  if (moveLen > 0) {
    position.x += (moveX / moveLen) * MOVE_SPEED * dt;
    position.z += (moveZ / moveLen) * MOVE_SPEED * dt;
  }
  position.y += moveY * MOVE_SPEED * dt;

  // Plain WASD = the movement joystick (left hand thumbstick, for the game's
  // own locomotion to read) — reads as 0 while Shift repurposes those same
  // keys to move the head instead.
  const leftStick = shiftHeld
    ? { x: 0, y: 0 }
    : { x: stickAxis('KeyA', 'KeyD'), y: stickAxis('KeyS', 'KeyW') };
  // No keyboard binding drives the right thumbstick currently.
  const rightStick = { x: 0, y: 0 };

  // IJKL/arrows orient their hand (for aiming) by default, and move it
  // instead when Shift is held.
  if (shiftHeld) {
    const leftMove = { x: stickAxis('KeyJ', 'KeyL'), y: stickAxis('KeyK', 'KeyI') };
    leftHandPos.x += leftMove.x * HAND_MOVE_SPEED * dt;
    leftHandPos.y += leftMove.y * HAND_MOVE_SPEED * dt;
    const rightMove = { x: stickAxis('ArrowLeft', 'ArrowRight'), y: stickAxis('ArrowDown', 'ArrowUp') };
    rightHandPos.x += rightMove.x * HAND_MOVE_SPEED * dt;
    rightHandPos.y += rightMove.y * HAND_MOVE_SPEED * dt;
  } else {
    leftHandYaw -= stickAxis('KeyJ', 'KeyL') * HAND_ANGULAR_SPEED * dt;
    leftHandPitch += stickAxis('KeyK', 'KeyI') * HAND_ANGULAR_SPEED * dt;
    rightHandYaw -= stickAxis('ArrowLeft', 'ArrowRight') * HAND_ANGULAR_SPEED * dt;
    rightHandPitch += stickAxis('ArrowDown', 'ArrowUp') * HAND_ANGULAR_SPEED * dt;
  }

  connection.sendInput({
    head: {
      position: { x: position.x, y: position.y, z: position.z },
      orientation: quatFromYawPitch(yaw, pitch),
    },
    leftHand: {
      pose: { position: { ...leftHandPos }, orientation: quatFromYawPitch(leftHandYaw, leftHandPitch) },
      buttons: {
        trigger: keysDown.has('KeyF') ? 1 : 0,
        squeeze: keysDown.has('KeyR') ? 1 : 0,
        thumbstick_x: leftStick.x,
        thumbstick_y: leftStick.y,
        thumbstick_click: keysDown.has('Digit3'),
        primary_click: keysDown.has('Digit1'),
        secondary_click: keysDown.has('Digit2'),
        menu_click: keysDown.has('Digit4'),
      },
    },
    rightHand: {
      pose: { position: { ...rightHandPos }, orientation: quatFromYawPitch(rightHandYaw, rightHandPitch) },
      buttons: {
        trigger: keysDown.has('Enter') ? 1 : 0,
        squeeze: keysDown.has('ControlRight') ? 1 : 0,
        thumbstick_x: rightStick.x,
        thumbstick_y: rightStick.y,
        thumbstick_click: keysDown.has('Digit9'),
        primary_click: keysDown.has('Digit7'),
        secondary_click: keysDown.has('Digit8'),
        menu_click: keysDown.has('Digit0'),
      },
    },
  });

  animationFrame = requestAnimationFrame(onFrame);
}

async function requestPointerLock() {
  await document.documentElement.requestPointerLock();
}

// Views are created/laid out in order (view 0 = left eye, view 1 = right eye
// per the OpenXR stereo view convention) — whether that's two swapchains
// (one per eye) or one array swapchain (one layer per eye), sorting frame
// keys by (swapchain_id, layer) puts the left eye first either way.
const eyeIds = computed(() => {
  const keys = Object.keys(connection.frames).sort((a, b) => {
    const [aSwapchain, aLayer] = a.split(':').map(Number);
    const [bSwapchain, bLayer] = b.split(':').map(Number);
    return aSwapchain! - bSwapchain! || aLayer! - bLayer!;
  });
  return { left: keys[0], right: keys[1] };
});
</script>

<template>
  <q-page
    class="column no-wrap"
    :style="{ height: availableHeight + 'px', overflow: 'hidden' }"
    @click="requestPointerLock"
  >
    <div class="row items-center no-wrap q-px-sm q-py-xs" style="flex: 0 0 auto">
      <span class="text-caption text-grey">
        {{
          isLocked
            ? 'Esc to release mouse'
            : 'Click to capture mouse — move to look, WASD = move joystick, Shift = move head/hands'
        }}
      </span>
      <q-space />
      <span v-if="eyeIds.left !== undefined" class="text-caption text-grey q-mr-sm">
        Cross your eyes for 3D
      </span>
      <q-icon name="photo_size_select_large" size="16px" class="text-grey q-mr-xs" />
      <q-slider
        v-model="previewScale"
        :min="0.2"
        :max="1"
        :step="0.05"
        dense
        style="width: 100px"
        class="q-mr-sm"
        @click.stop
      />
      <q-icon name="help_outline" size="18px" class="text-grey cursor-help">
        <q-tooltip>
          Head: WASD = movement joystick (left thumbstick); mouse-look; Shift+WASD = move head; Space/Ctrl =
          head up/down<br />
          Left hand (IJKL): aim (yaw/pitch); Shift+IJKL = move hand; R squeeze, F trigger,
          1/2/3/4 = A/B/stick-click/menu<br />
          Right hand (arrows): aim (yaw/pitch); Shift+arrows = move hand; RCtrl squeeze, Enter
          trigger, 7/8/9/0 = A/B/stick-click/menu
        </q-tooltip>
      </q-icon>
    </div>
    <div class="col row justify-center items-center no-wrap" style="min-height: 0">
      <div
        class="row justify-center items-center no-wrap"
        :style="{ width: previewScale * 100 + '%', height: previewScale * 100 + '%', gap: '8px' }"
      >
        <img
          v-if="eyeIds.right !== undefined"
          :src="connection.frames[eyeIds.right]"
          style="max-width: calc(50% - 4px); max-height: 100%; image-rendering: pixelated"
        />
        <img
          v-if="eyeIds.left !== undefined"
          :src="connection.frames[eyeIds.left]"
          style="max-width: calc(50% - 4px); max-height: 100%; image-rendering: pixelated"
        />
      </div>
    </div>
    <div class="row justify-center q-py-xs" style="flex: 0 0 auto">
      <q-btn dense size="sm" @click.stop="connection.ping()">Ping</q-btn>
    </div>
  </q-page>
</template>
