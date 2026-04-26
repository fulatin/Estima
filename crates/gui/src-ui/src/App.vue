<template>
  <div class="min-h-screen bg-[#121212] text-white">
    <header class="border-b border-[#333] px-6 py-4">
      <div class="flex items-center justify-between">
        <div>
          <h1 class="text-2xl font-bold tracking-tight">Estima</h1>
          <p class="text-sm text-[#a0a0a0]">AI-Controlled Real-Time Audio Effects</p>
        </div>
        <div class="flex items-center gap-3">
          <button 
            @click="settingsOpen = true"
            class="flex items-center gap-2 px-3 py-2 border border-[#444] text-[#a0a0a0] hover:border-[#555] hover:text-white transition-colors"
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
            </svg>
            <span class="text-sm">Settings</span>
          </button>
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

    <Settings :is-open="settingsOpen" @close="settingsOpen = false" />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import PluginManager from './components/PluginManager.vue'
import EffectChain from './components/EffectChain.vue'
import ParameterPanel from './components/ParameterPanel.vue'
import PresetManager from './components/PresetManager.vue'
import AIChat from './components/AIChat.vue'
import Settings from './components/Settings.vue'
import { useAudioStore } from './stores/audioStore'

const store = useAudioStore()
const bypass = computed(() => store.bypass)
const settingsOpen = ref(false)

onMounted(async () => {
  await store.refreshStatus()
})

async function toggleBypass() {
  await store.toggleBypass()
}
</script>
