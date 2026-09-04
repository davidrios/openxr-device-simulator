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
const position = { x: 0, y: 0, z: 0 };
const SENSITIVITY = 0.002;
const MOVE_SPEED = 1.5; // meters per second

const keysDown = new Set<string>();
let animationFrame = 0;
let lastFrameTime = 0;

// Matches the resting controller pose in runtime/src/input/device_state.rs.
// Real 6DoF (gyro + arm model) comes later; for keyboard testing the thumbstick
// just pushes the hand around in the X/Y plane so movement is visible.
const IDENTITY_ORIENTATION = { x: 0, y: 0, z: 0, w: 1 };
const leftHandPos = { x: -0.3, y: -0.3, z: -0.5 };
const rightHandPos = { x: 0.3, y: -0.3, z: -0.5 };
const HAND_MOVE_SPEED = 0.5; // meters per second

function stickAxis(negKey: string, posKey: string): number {
  let v = 0;
  if (keysDown.has(posKey)) v += 1;
  if (keysDown.has(negKey)) v -= 1;
  return v;
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
  lastFrameTime = performance.now();
  animationFrame = requestAnimationFrame(onFrame);
});

onUnmounted(() => {
  document.removeEventListener('pointerlockchange', onPointerLockChange);
  document.removeEventListener('mousemove', onMouseMove);
  document.removeEventListener('keydown', onKeyDown);
  document.removeEventListener('keyup', onKeyUp);
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

  // Forward/right vectors on the horizontal plane, from yaw only (ignore pitch).
  const forward = { x: -Math.sin(yaw), z: -Math.cos(yaw) };
  const right = { x: Math.cos(yaw), z: -Math.sin(yaw) };

  let moveX = 0;
  let moveZ = 0;
  let moveY = 0;
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
  if (keysDown.has('Space')) moveY += 1;
  if (keysDown.has('ShiftLeft')) moveY -= 1;

  const moveLen = Math.hypot(moveX, moveZ);
  if (moveLen > 0) {
    position.x += (moveX / moveLen) * MOVE_SPEED * dt;
    position.z += (moveZ / moveLen) * MOVE_SPEED * dt;
  }
  position.y += moveY * MOVE_SPEED * dt;

  const leftStick = { x: stickAxis('KeyJ', 'KeyL'), y: stickAxis('KeyK', 'KeyI') };
  const rightStick = { x: stickAxis('ArrowLeft', 'ArrowRight'), y: stickAxis('ArrowDown', 'ArrowUp') };
  leftHandPos.x += leftStick.x * HAND_MOVE_SPEED * dt;
  leftHandPos.y += leftStick.y * HAND_MOVE_SPEED * dt;
  rightHandPos.x += rightStick.x * HAND_MOVE_SPEED * dt;
  rightHandPos.y += rightStick.y * HAND_MOVE_SPEED * dt;

  // q = q_yaw (Y-axis) * q_pitch (X-axis)
  const sy = Math.sin(yaw * 0.5);
  const cy = Math.cos(yaw * 0.5);
  const sp = Math.sin(pitch * 0.5);
  const cp = Math.cos(pitch * 0.5);

  connection.sendInput({
    head: {
      position: { x: position.x, y: position.y, z: position.z },
      orientation: {
        x: cy * sp,
        y: sy * cp,
        z: -sy * sp,
        w: cy * cp,
      },
    },
    leftHand: {
      pose: { position: { ...leftHandPos }, orientation: IDENTITY_ORIENTATION },
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
      pose: { position: { ...rightHandPos }, orientation: IDENTITY_ORIENTATION },
      buttons: {
        trigger: keysDown.has('ShiftRight') ? 1 : 0,
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

// Swapchains are created in view order (view 0 = left eye, view 1 = right eye
// per the OpenXR stereo view convention), so the lower id is the left eye.
const eyeIds = computed(() => {
  const ids = Object.keys(connection.frames)
    .map(Number)
    .sort((a, b) => a - b);
  return { left: ids[0], right: ids[1] };
});
</script>

<template>
  <q-page class="column items-center q-gutter-md q-pa-md" @click="requestPointerLock">
    <div v-if="!isLocked" class="text-caption text-grey">
      Click anywhere to capture mouse — move to look around, WASD to move, Space/Shift for
      up/down
    </div>
    <div v-else class="text-caption text-grey">Press Esc to release mouse</div>
    <div class="text-caption text-grey">
      Left controller: IJKL stick, R squeeze, F trigger, 1/2/3/4 = A/B/stick-click/menu &nbsp;·&nbsp;
      Right controller: arrow keys stick, RCtrl squeeze, RShift trigger, 7/8/9/0 = A/B/stick-click/menu
    </div>
    <div v-if="eyeIds.left !== undefined && eyeIds.right !== undefined" class="column items-center">
      <div class="text-caption text-grey">Cross-eyed view — cross your eyes to fuse the pair into 3D</div>
      <div class="row q-gutter-sm">
        <img
          :src="connection.frames[eyeIds.right]"
          style="width: 360px; image-rendering: pixelated"
        />
        <img
          :src="connection.frames[eyeIds.left]"
          style="width: 360px; image-rendering: pixelated"
        />
      </div>
    </div>
    <q-btn @click.stop="connection.ping()">Ping</q-btn>
  </q-page>
</template>
