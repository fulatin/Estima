<template>
  <div>
    <h2 class="text-sm font-semibold text-[#a0a0a0] uppercase tracking-wider mb-3">
      Presets
    </h2>
    
    <div class="flex gap-2 mb-3">
      <input 
        v-model="presetName"
        type="text" 
        placeholder="Preset name..."
        class="flex-1 px-3 py-2 bg-[#121212] border border-[#333] text-white placeholder-[#666] focus:border-amber-500 focus:outline-none text-sm"
      >
      <button 
        @click="savePreset"
        :disabled="!presetName"
        class="px-4 py-2 text-xs border border-amber-500 text-amber-400 hover:bg-amber-500/10 disabled:border-[#333] disabled:text-[#666] disabled:cursor-not-allowed transition-colors"
      >
        Save
      </button>
    </div>
    
    <div class="max-h-48 overflow-y-auto space-y-px">
      <div 
        v-for="preset in presets" 
        :key="preset"
        class="flex items-center justify-between p-3 bg-[#121212] border-l-2 border-transparent hover:border-amber-500 transition-colors"
      >
        <span class="text-sm text-[#a0a0a0]">{{ preset }}</span>
        <button 
          @click="loadPreset(preset)"
          class="px-2 py-1 text-xs border border-[#444] text-[#a0a0a0] hover:border-amber-500 hover:text-amber-400 transition-colors"
        >
          Load
        </button>
      </div>
      <div v-if="presets.length === 0" class="text-[#666] text-sm py-4 text-center">
        No presets saved
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useAudioStore } from '../stores/audioStore'

const store = useAudioStore()
const presetName = ref('')
const presets = ref<string[]>([])

onMounted(() => {
  refreshPresets()
})

async function refreshPresets() {
  presets.value = await store.listPresets()
}

async function savePreset() {
  if (!presetName.value) return
  await store.savePreset(presetName.value)
  presetName.value = ''
  await refreshPresets()
}

async function loadPreset(name: string) {
  await store.loadPreset(name)
}
</script>
