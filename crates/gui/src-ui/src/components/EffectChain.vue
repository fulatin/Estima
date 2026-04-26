<template>
  <div>
    <h2 class="text-sm font-semibold text-[#a0a0a0] uppercase tracking-wider mb-3">
      Effect Chain
    </h2>
    
    <div v-if="plugins.length === 0" class="text-[#666] text-sm py-8 text-center border border-dashed border-[#333]">
      No plugins loaded
    </div>
    
    <div v-else class="space-y-px">
      <div 
        v-for="(plugin, index) in plugins" 
        :key="plugin.id"
        :class="[
          'flex items-center justify-between p-3 border-l-2 transition-colors',
          plugin.bypass 
            ? 'border-[#444] bg-[#121212]/50 text-[#666]' 
            : 'border-amber-500 bg-[#121212]'
        ]"
      >
        <div class="flex items-center gap-3">
          <div class="flex flex-col gap-0.5">
            <button 
              @click="movePlugin(plugin.id, -1)"
              :disabled="index === 0"
              class="text-[#666] hover:text-white disabled:text-[#333] disabled:cursor-not-allowed text-xs leading-none"
            >▲</button>
            <button 
              @click="movePlugin(plugin.id, 1)"
              :disabled="index === plugins.length - 1"
              class="text-[#666] hover:text-white disabled:text-[#333] disabled:cursor-not-allowed text-xs leading-none"
            >▼</button>
          </div>
          <span class="text-[#666] font-mono text-xs w-4">{{ index + 1 }}</span>
          <div>
            <div :class="['font-medium text-sm', plugin.bypass ? 'line-through' : 'text-white']">
              {{ plugin.name }}
            </div>
            <div class="text-xs text-[#666]">{{ plugin.plugin_type }}</div>
          </div>
        </div>
        <div class="flex gap-1">
          <button 
            v-if="plugin.hasUI"
            @click="showPluginUI(plugin)"
            class="px-2 py-1 text-xs border border-purple-500 text-purple-400 hover:bg-purple-500/10 transition-colors"
            title="Plugin has native UI (coming soon)"
          >
            UI
          </button>
          <button 
            @click="toggleBypass(plugin.id)"
            :class="[
              'px-2 py-1 text-xs border transition-colors',
              plugin.bypass 
                ? 'border-amber-500 text-amber-400 hover:bg-amber-500/10' 
                : 'border-[#444] text-[#a0a0a0] hover:border-[#555]'
            ]"
          >
            {{ plugin.bypass ? 'Enable' : 'Bypass' }}
          </button>
          <button 
            @click="selectPlugin(plugin)"
            class="px-2 py-1 text-xs border border-[#444] text-[#a0a0a0] hover:border-amber-500 hover:text-amber-400 transition-colors"
          >
            Edit
          </button>
          <button 
            @click="removePlugin(plugin.id)"
            class="px-2 py-1 text-xs border border-[#444] text-red-400 hover:border-red-500 hover:bg-red-500/10 transition-colors"
          >
            Remove
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useAudioStore } from '../stores/audioStore'

const store = useAudioStore()
const plugins = computed(() => store.plugins)

async function removePlugin(id: string) {
  await store.removePlugin(id)
}

async function selectPlugin(plugin: any) {
  await store.selectPlugin(plugin)
}

async function toggleBypass(id: string) {
  try {
    await store.togglePluginBypass(id)
  } catch (e) {
    console.error('togglePluginBypass error:', e)
  }
}

async function movePlugin(id: string, direction: number) {
  try {
    await store.movePlugin(id, direction)
  } catch (e) {
    console.error('movePlugin error:', e)
  }
}

function showPluginUI(plugin: any) {
  alert(`${plugin.name} has a native UI.\n\nThis feature is coming soon!`)
}
</script>
