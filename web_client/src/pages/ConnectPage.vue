<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { useRouter } from 'vue-router';

import { useConnection } from 'src/stores/connection';
import { useQuasar } from 'quasar';

const $q = useQuasar();
const router = useRouter();
const connection = useConnection();

onMounted(async () => {
  if (connection.isConnected) {
    await router.push({ path: '/main', replace: true });
  }
});

function validAddress(val: string) {
  return /https?:\/\/[\w-.]+:\d+/.test(val) || 'Please enter a valid address';
}

const isConnecting = ref(false);

async function onSubmit() {
  isConnecting.value = true;
  try {
    await connection.connect(input.value);
    if (connection.isConnected) {
      await router.push({ path: '/main', replace: true });
    }
  } catch (err) {
    console.error(err);
    if (err instanceof Error) {
      $q.notify({ type: 'negative', message: `Error connecting to server: ${err.message}` });
    } else {
      $q.notify({ type: 'negative', message: `Error connecting to server` });
    }
  } finally {
    isConnecting.value = false;
  }
}

const input = ref('http://localhost:3050');
</script>

<template>
  <q-page class="row items-center justify-evenly">
    <q-form style="width: 300px" @submit="onSubmit" class="q-gutter-md">
      <q-input v-model="input" label="Address" debounce="true" :rules="[validAddress]" />
      <div>
        <q-btn
          type="submit"
          :label="isConnecting ? 'Connecting...' : 'Connect'"
          color="primary"
          class="full-width"
          :disable="isConnecting"
        />
      </div>
    </q-form>
  </q-page>
</template>
