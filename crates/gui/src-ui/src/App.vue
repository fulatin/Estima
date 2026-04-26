<template>
  <div class="min-h-screen bg-[#121212] text-white">
    <header class="border-b border-[#333] px-6 py-4">
      <div class="flex items-center justify-between">
        <div>
          <h1 class="text-2xl font-bold tracking-tight">Estima</h1>
          <p class="text-sm text-[#a0a0a0]">AI-Controlled Real-Time Audio Effects</p>
        </div>
        <button 
          @click="toggleBypass"
          :class="[
            'flex items-center gap-2 px-4 py-2 border transition-colors',
            bypass 
              ? 'border-amber-500 bg-amber-500/10 text-amber-400' 
              : 'border-[#444] bg-[#252525] text-[#a0a0a0] hover:border-[#555]'
          ]"
        >
          <span class="w-2 h-2" :class="bypass ? 'bg-amber-500' : 'bg-green-500'"></span>
          <span class="text-sm font-medium">BYPASS</span>
          <span class="text-xs" :class="bypass ? 'text-amber-400' : 'text-green-400'">
            {{ bypass ? 'ON' : 'OFF' }}
          </span>
        </button>
      </div>
    </header>
    
    <main class="p-6">
      <div class="grid grid-cols-1 lg:grid-cols-2 gap-px bg-[#333]">
        <div class="bg-[#1e1e1e] p-4">
          <PluginManager />
        </div>
        <div class="bg-[#1e1e1e] p-4">
          <EffectChain />
        </div>
        <div class="bg-[#1e1e1e] p-4">
          <ParameterPanel />
        </div>
        <div class="bg-[#1e1e1e] p-4">
          <PresetManager />
        </div>
      </div>
      
      <div class="mt-px bg-[#1e1e1e]">
        <AIChat />
      </div>
    </main>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted } from 'vue'
import PluginManager from './components/PluginManager.vue'
import EffectChain from './components/EffectChain.vue'
import ParameterPanel from './components/ParameterPanel.vue'
import PresetManager from './components/PresetManager.vue'
import AIChat from './components/AIChat.vue'
import { useAudioStore } from './stores/audioStore'

const store = useAudioStore()
const bypass = computed(() => store.bypass)

onMounted(async () => {
  await store.refreshStatus()
})

async function toggleBypass() {
  await store.toggleBypass()
}
</script>
