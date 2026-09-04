<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from 'vue';
import { useRouter } from 'vue-router';

import { useConnection } from 'src/stores/connection';
import { useQuasar } from 'quasar';

const $q = useQuasar();
const router = useRouter();
const connection = useConnection();

const isLocked = ref(false);
let yaw = 0;
let pitch = 0;
const SENSITIVITY = 0.002;

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
});

onUnmounted(() => {
  document.removeEventListener('pointerlockchange', onPointerLockChange);
  document.removeEventListener('mousemove', onMouseMove);
  if (document.pointerLockElement) document.exitPointerLock();
});

function onPointerLockChange() {
  isLocked.value = !!document.pointerLockElement;
}

function onMouseMove(e: MouseEvent) {
  if (!document.pointerLockElement) return;
  yaw -= e.movementX * SENSITIVITY;
  pitch -= e.movementY * SENSITIVITY;
  pitch = Math.max(-Math.PI / 2 + 0.01, Math.min(Math.PI / 2 - 0.01, pitch));
  connection.sendLook(yaw, pitch);
}

async function requestPointerLock() {
  await document.documentElement.requestPointerLock();
}
</script>

<template>
  <q-page class="column items-center q-gutter-md q-pa-md" @click="requestPointerLock">
    <div v-if="!isLocked" class="text-caption text-grey">
      Click anywhere to capture mouse — move to look around
    </div>
    <div v-else class="text-caption text-grey">Press Esc to release mouse</div>
    <div class="row q-gutter-md">
      <div v-for="(src, id) in connection.frames" :key="id" class="column items-center">
        <div class="text-caption">Swapchain {{ id }}</div>
        <img :src="src" style="max-width: 512px; image-rendering: pixelated" />
      </div>
    </div>
    <q-btn @click.stop="connection.ping()">Ping</q-btn>
  </q-page>
</template>
