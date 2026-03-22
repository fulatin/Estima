<template>
  <div class="fixed bottom-4 right-4">
    <button 
      @click="toggleBypass"
      :class="[
        'px-6 py-3 rounded-lg font-bold transition-all',
        bypass 
          ? 'bg-red-500 hover:bg-red-600 text-white' 
          : 'bg-green-500 hover:bg-green-600 text-white'
      ]"
    >
      {{ bypass ? 'BYPASS ON' : 'BYPASS OFF' }}
    </button>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useAudioStore } from '../stores/audioStore'

const store = useAudioStore()
const bypass = computed(() => store.bypass)

onMounted(() => {
  store.refreshStatus()
})

async function toggleBypass() {
  await store.toggleBypass()
}
</script>
